use std::str::FromStr;

use anyhow::anyhow;
use async_trait::async_trait;
use color_eyre::owo_colors::OwoColorize;
use cosmrs::tendermint::chain::Id as ChainId; // TODO see if this redundancy in dependencies can be decreased
use futures_util::stream::StreamExt;
use reqwest::Url;
use serde_json::json;
use tendermint_rpc::{query::EventType, HttpClient, SubscriptionClient, WebSocketClient};
use tm_prover::{config::Config as TmProverConfig, prover::prove};
use tracing::{debug, info};
use wasmd_client::{CliWasmdClient, WasmdClient};

use super::utils::{helpers::block_tx_commit, types::WasmdTxResponse};
use crate::{
    config::Config,
    error::Error,
    handler::{
        utils::{helpers::read_cached_hash_height, relay::RelayMessage},
        Handler,
    },
    request::handshake::HandshakeRequest,
    response::{handshake::HandshakeResponse, Response},
};

#[async_trait]
impl Handler for HandshakeRequest {
    type Error = Error;
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(
        self,
        config: C,
    ) -> Result<Self::Response, Self::Error> {
        let config = config.as_ref().clone();

        info!("{}", "\nPeforming Handshake".blue().bold());

        // TODO: may need to import verbosity here
        let pub_key = handshake(self, config)
            .await
            .map_err(|e| Error::GenericErr(e.to_string()))?;

        Ok(HandshakeResponse { pub_key }.into())
    }
}

async fn handshake(args: HandshakeRequest, config: Config) -> Result<String, anyhow::Error> {
    let httpurl = config.node_url.clone();
    let wsurl = config.websocket_url.clone();

    let tmrpc_client = HttpClient::new(httpurl.as_str())?;
    let wasmd_client = CliWasmdClient::new(Url::parse(httpurl.as_str())?);

    let (trusted_height, trusted_hash) = read_cached_hash_height(&config).await?;

    info!("Running SessionCreate");

    let res: serde_json::Value = RelayMessage::SessionCreate
        .run_relay(config.enclave_rpc())
        .await?;
    info!("\n\n Enclave run realy response: {:?}", res);

    let output: WasmdTxResponse = serde_json::from_str(
        wasmd_client
            .tx_execute(
                &args.contract.clone(),
                &config.chain_id,
                2000000,
                &config.tx_sender,
                json!(res),
                "11000untrn", // Add the fee here
            )?
            .as_str(),
    )?;
    info!("\n\n SessionCreate tx output: {:?}", output);

    // Wait for tx to commit
    block_tx_commit(&tmrpc_client, output.txhash).await?;
    info!("SessionCreate tx committed");

    // Wait 2 blocks
    info!("Waiting 2 blocks for light client proof");
    two_block_waitoor(&wsurl).await?;

    // Call tm prover with trusted hash and height
    let prover_config = TmProverConfig {
        primary: httpurl.as_str().parse()?,
        witnesses: httpurl.as_str().parse()?,
        trusted_height,
        trusted_hash,
        verbose: "1".parse()?, // TODO: both tm-prover and cli define the same Verbosity struct. Need to define this once and import
        contract_address: args.contract.clone(),
        storage_key: "quartz_session".to_string(),
        chain_id: config.chain_id.to_string(),
        ..Default::default()
    };

    let proof_output = prove(prover_config)
        .await
        .map_err(|report| anyhow!("Tendermint prover failed. Report: {}", report))?;

    // Execute SessionSetPubKey on enclave
    info!("Running SessionSetPubKey");
    let res: serde_json::Value = RelayMessage::SessionSetPubKey {
        proof: proof_output,
    }
    .run_relay(config.enclave_rpc())
    .await?;

    // Submit SessionSetPubKey to contract
    let output: WasmdTxResponse = serde_json::from_str(
        wasmd_client
            .tx_execute(
                &args.contract.clone(),
                &ChainId::from_str("pion-1")?,
                2000000,
                &config.tx_sender,
                json!(res),
                "11000untrn", // Add the fee here
            )?
            .as_str(),
    )?;
    info!("\n\n SessionCreate tx output: {:?}", output);

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
