use std::{env::current_dir, fs, path::Path, str::FromStr};

use anyhow::anyhow;
use async_trait::async_trait;
use cosmrs::tendermint::chain::Id as ChainId; // TODO see if this redundancy in dependencies can be decreased
use cycles_sync::wasmd_client::{CliWasmdClient, WasmdClient};
use futures_util::stream::StreamExt;
use reqwest::Url;
use serde::Serialize;
use serde_json::json;
use tendermint::{block::Height, Hash};
use tendermint_rpc::{query::EventType, HttpClient, SubscriptionClient, WebSocketClient};
use tm_prover::{config::Config as TmProverConfig, prover::prove};
use tracing::{debug, info, trace};

use super::utils::{
    helpers::{block_tx_commit, run_relay},
    types::WasmdTxResponse,
};
use crate::{
    error::Error,
    handler::{utils::types::RelayMessage, Handler},
    request::handshake::HandshakeRequest,
    response::{handshake::HandshakeResponse, Response},
    Config,
};

#[async_trait]
impl Handler for HandshakeRequest {
    type Error = Error;
    type Response = Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error> {
        trace!("starting handshake...");

        // TODO: may need to import verbosity here
        let pub_key = handshake(self, config.mock_sgx)
            .await
            .map_err(|e| Error::GenericErr(e.to_string()))?;

        Ok(HandshakeResponse { pub_key }.into())
    }
}

#[derive(Serialize)]
struct Message<'a> {
    message: &'a str,
}

async fn handshake(args: HandshakeRequest, mock_sgx: bool) -> Result<String, anyhow::Error> {
    let httpurl = Url::parse(&format!("http://{}", args.node_url))?;
    let wsurl = format!("ws://{}/websocket", args.node_url);

    let tmrpc_client = HttpClient::new(httpurl.as_str())?;
    let wasmd_client = CliWasmdClient::new(Url::parse(httpurl.as_str())?);

    // TODO: dir logic issue #125
    // Read trusted hash and height from files
    let base_path = current_dir()?.join("../");
    let trusted_files_path = args.app_dir;
    let (trusted_height, trusted_hash) = read_hash_height(trusted_files_path.as_path()).await?;

    info!("Running SessionCreate");
    let res: serde_json::Value =
        run_relay(base_path.as_path(), mock_sgx, RelayMessage::SessionCreate)?;

    let output: WasmdTxResponse = serde_json::from_str(
        wasmd_client
            .tx_execute(
                &args.contract.clone(),
                &args.chain_id,
                2000000,
                &args.sender,
                json!(res),
            )?
            .as_str(),
    )?;
    debug!("\n\n SessionCreate tx output: {:?}", output);

    // Wait for tx to commit
    block_tx_commit(&tmrpc_client, output.txhash).await?;
    info!("SessionCreate tx committed");

    // Wait 2 blocks
    info!("Waiting 2 blocks for light client proof");
    two_block_waitoor(&wsurl).await?;

    // TODO: dir logic issue #125
    let proof_path = current_dir()?.join("../utils/tm-prover/light-client-proof.json");
    debug!("Proof path: {:?}", proof_path.to_str());

    // Call tm prover with trusted hash and height
    let config = TmProverConfig {
        primary: httpurl.as_str().parse()?,
        witnesses: httpurl.as_str().parse()?,
        trusted_height,
        trusted_hash,
        trace_file: Some(proof_path.clone()),
        verbose: "1".parse()?, // TODO: both tm-prover and cli define the same Verbosity struct. Need to define this once and import
        contract_address: args.contract.clone(),
        storage_key: "quartz_session".to_string(),
        chain_id: args.chain_id.to_string(),
        ..Default::default()
    };
    debug!("config: {:?}", config);
    if let Err(report) = prove(config).await {
        return Err(anyhow!("Tendermint prover failed. Report: {}", report));
    }

    // Read proof file
    let proof = fs::read_to_string(proof_path.as_path())?;
    let proof_json = serde_json::to_string(&Message {
        message: proof.trim(),
    })?;

    // Execute SessionSetPubKey on enclave
    info!("Running SessionSetPubKey");
    let res: serde_json::Value = run_relay(
        base_path.as_path(),
        mock_sgx,
        RelayMessage::SessionSetPubKey(proof_json),
    )?;

    // Submit SessionSetPubKey to contract
    let output: WasmdTxResponse = serde_json::from_str(
        wasmd_client
            .tx_execute(
                &args.contract.clone(),
                &ChainId::from_str("testing")?,
                2000000,
                &args.sender,
                json!(res),
            )?
            .as_str(),
    )?;

    // Wait for tx to commit
    block_tx_commit(&tmrpc_client, output.txhash).await?;
    info!("SessionSetPubKey tx committed");

    let output: WasmdTxResponse = wasmd_client.query_tx(&output.txhash.to_string())?;

    let wasm_event = output
        .events
        .iter()
        .find(|e| e.kind == "wasm")
        .expect("Wasm transactions are guaranteed to contain a 'wasm' event");

    if let Some(pubkey) = wasm_event.attributes.iter().find(|a| {
        a.key_str()
            .expect("SessionSetPubKey tx is expected to have 'pub_key' attribute")
            == "pub_key"
    }) {
        Ok(pubkey.value_str()?.to_string())
    } else {
        Err(anyhow!("Failed to find pubkey from SetPubKey message"))
    }
}

async fn two_block_waitoor(wsurl: &str) -> Result<(), anyhow::Error> {
    let (client, driver) = WebSocketClient::new(wsurl).await?;

    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Subscription functionality
    let mut subs = client.subscribe(EventType::NewBlock.into()).await?;

    // Wait 2 NewBlock events
    let mut ev_count = 2_i32;
    debug!("Blocks left: {ev_count} ...");

    while let Some(res) = subs.next().await {
        let _ev = res?;
        ev_count -= 1;
        debug!("Blocks left: {ev_count} ...");
        if ev_count == 0 {
            break;
        }
    }

    // Signal to the driver to terminate.
    client.close()?;
    // Await the driver's termination to ensure proper connection closure.
    let _ = driver_handle.await?;

    Ok(())
}

async fn read_hash_height(base_path: &Path) -> Result<(Height, Hash), anyhow::Error> {
    let height_path = base_path.join("trusted.height");
    let trusted_height: Height = fs::read_to_string(height_path.as_path())?.trim().parse()?;

    let hash_path = base_path.join("trusted.hash");
    let trusted_hash: Hash = fs::read_to_string(hash_path.as_path())?.trim().parse()?;

    Ok((trusted_height, trusted_hash))
}
