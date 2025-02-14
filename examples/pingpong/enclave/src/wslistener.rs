use std::{collections::BTreeMap, str::FromStr};

use anyhow::{anyhow, Error, Result};
use cosmrs::{tendermint::chain::Id as ChainId, AccountId};
use cw_client::{CwClient, GrpcClient};
use futures_util::StreamExt;
use ping_pong_contract::msg::{
    execute::{Ping, Pong},
    AttestedMsg, ExecuteMsg,
};
use quartz_common::{
    contract::msg::execute::attested::{RawAttested, RawNoop},
    enclave::{
        attestor::Attestor,
        server::{WebSocketHandler, WsListenerConfig},
    },
};
use quartz_tm_prover::{config::Config as TmProverConfig, prover::prove};
use serde::Deserialize;
use serde_json::json;
use tendermint_rpc::{event::Event, query::EventType, SubscriptionClient, WebSocketClient};
use tonic::Request;
use tracing::info;

use crate::{
    ping_pong_server::{PingOpEvent, PingPongService, PongOp},
    proto::{ping_pong_server::PingPong, PingRequest},
};

impl TryFrom<Event> for PingOpEvent {
    type Error = Error;

    fn try_from(event: Event) -> Result<Self, Error> {
        if let Some(events) = &event.events {
            for key in events.keys() {
                match key.as_str() {
                    k if k.starts_with("wasm.action") => {
                        let (contract, ping) = extract_event_info(events)
                            .map_err(|_| anyhow!("Failed to extract event info from event"))?;

                        return Ok(PingOpEvent::Ping { contract, ping });
                    }
                    _ => {}
                }
            }
        }

        Err(anyhow!("Unsupported event."))
    }
}

// TODO: Need to prevent listener from taking actions until handshake is completed
#[async_trait::async_trait]
impl<A: Attestor + Clone> WebSocketHandler for PingPongService<A> {
    async fn handle(&self, event: Event, config: WsListenerConfig) -> Result<()> {
        let op_event = PingOpEvent::try_from(event)?;

        self.queue_producer
            .send(PongOp {
                client: self.clone(),
                event: op_event,
                config,
            })
            .await?;

        Ok(())
    }
}

#[tonic::async_trait]
pub trait WsListener: Send + Sync + 'static {
    async fn process(&self, event: PingOpEvent, config: WsListenerConfig) -> Result<()>;
}

#[async_trait::async_trait]
impl<A> WsListener for PingPongService<A>
where
    A: Attestor,
    A::RawAttestation: for<'de> Deserialize<'de> + Send,
{
    async fn process(&self, event: PingOpEvent, config: WsListenerConfig) -> Result<()> {
        match event {
            PingOpEvent::Ping { contract, ping } => {
                println!("Processing ping event");
                ping_handler(self, &contract, ping, &config).await?;
            }
        }

        // Wait some blocks to make sure transaction was confirmed
        two_block_waitoor(config.ws_url.as_str()).await?;

        Ok(())
    }
}

fn extract_event_info(events: &BTreeMap<String, Vec<String>>) -> Result<(AccountId, Ping)> {
    // Set common info data for all events
    let contract_address = events
        .get("execute._contract_address")
        .ok_or_else(|| anyhow!("Missing execute._contract_address in events"))?
        .first()
        .ok_or_else(|| anyhow!("execute._contract_address is empty"))?
        .parse::<AccountId>()
        .map_err(|e| anyhow!("Failed to parse contract address: {}", e))?;

    let ping_str = events
        .get("wasm.ping_data")
        .and_then(|v| v.first())
        .ok_or_else(|| anyhow!("Missing ping data in event"))?;
    println!("Ping: {}", ping_str);

    let ping: Ping = serde_json::from_str(&ping_str)?;

    Ok((contract_address, ping))
}

async fn ping_handler<A>(
    client: &PingPongService<A>,
    contract: &AccountId,
    ping: Ping,
    ws_config: &WsListenerConfig,
) -> Result<()>
where
    A: Attestor,
    A::RawAttestation: for<'de> Deserialize<'de>,
{
    let sk = hex::decode(ws_config.admin_sk.clone())?
        .as_slice()
        .try_into()
        .map_err(|e| anyhow!("failed to read/parse sk: {}", e))?;
    let cw_client = GrpcClient::new(sk, ws_config.grpc_url.clone());

    // Sometimes we may need to query the chain at this point for data to send to the enclave.
    // In this case, everything we need is in the event data.

    // Wait 2 blocks
    info!("Waiting 2 blocks for light client proof");
    two_block_waitoor(ws_config.ws_url.as_str()).await?;

    // Generate proof that the requested data (ping.message) is stored on-chain.
    // Call tm prover with trusted hash and height
    let prover_config = TmProverConfig {
        primary: ws_config.node_url.as_str().parse()?,
        witnesses: ws_config.node_url.as_str().parse()?,
        trusted_height: ws_config.trusted_height,
        trusted_hash: ws_config.trusted_hash,
        verbose: "1".parse()?,
        contract_address: contract.clone(),
        storage_key: ping.pubkey.to_string(), // For Maps, storage key is the key/value pair's key. We prove the inclusion of the value.
        storage_namespace: Some("pings".to_string()),
        chain_id: ws_config.chain_id.to_string(),
        ..Default::default()
    };

    let proof_output = tokio::task::spawn_blocking(move || {
        // Create a new runtime inside the blocking thread.
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            prove(prover_config)
                .await
                .map_err(|report| anyhow!("Tendermint prover failed. Report: {}", report))
        })
    })
    .await??; // Handle both JoinError and your custom error

    // Merge the data with the proof
    let mut proof_json = serde_json::to_value(proof_output)?;
    proof_json["msg"] = serde_json::to_value(&ping)?;

    // Build final request object
    let request = Request::new(PingRequest {
        message: json!(proof_json).to_string(),
    });

    // Send request to enclave over tonic gRPC client
    let pong_response = client
        .run(request)
        .await
        .map_err(|e| anyhow!("Failed to communicate to relayer. {e}"))?
        .into_inner();

    // Extract json from enclave response
    let attested: RawAttested<Pong, A::RawAttestation> =
        serde_json::from_str(&pong_response.message)
            .map_err(|e| anyhow!("Error deserializing Pong response from enclave: {}", e))?;

    // Build on-chain response
    // TODO add non-mock support
    let pong_msg = ExecuteMsg::Pong(AttestedMsg {
        msg: RawNoop(attested.msg),
        attestation: attested.attestation,
    });

    // Post response to chain
    let chain_id = &ChainId::from_str(&ws_config.chain_id)?;
    let output = cw_client
        .tx_execute(
            contract,
            chain_id,
            2000000,
            &ws_config.tx_sender,
            json!(pong_msg),
            "11000untrn",
        )
        .await?;

    println!("Output TX: {}", output);
    Ok(())
}

async fn two_block_waitoor(wsurl: &str) -> Result<(), Error> {
    let (client, driver) = WebSocketClient::new(wsurl).await?;

    let driver_handle = tokio::spawn(async move { driver.run().await });

    // Subscription functionality
    let mut subs = client.subscribe(EventType::NewBlock.into()).await?;

    // Wait 2 NewBlock events
    let mut ev_count = 2_i32;

    while let Some(res) = subs.next().await {
        let _ev = res?;
        ev_count -= 1;
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
