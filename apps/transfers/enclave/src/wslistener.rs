//TODO: get rid of this
use std::str::FromStr;

use anyhow::{anyhow, Result};
use cosmrs::{tendermint::chain::Id as ChainId, AccountId};
use cosmwasm_std::{Addr, HexBinary};
use futures_util::StreamExt;
use quartz_common::{
    contract::msg::execute::attested::{
        MockAttestation, RawAttested, RawAttestedMsgSansHandler, RawMockAttestation,
    },
    enclave::{
        attestor::Attestor,
        server::{WebSocketHandler, WsListenerConfig},
    },
};
use reqwest::Url;
use serde_json::json;
use tendermint_rpc::{event::Event, query::EventType, SubscriptionClient, WebSocketClient};
use tm_prover::{config::Config as TmProverConfig, prover::prove};
use tonic::Request;
use tracing::info;
use transfers_contract::msg::{
    execute::{QueryResponseMsg, Request as TransferRequest, UpdateMsg},
    AttestedMsg, ExecuteMsg,
    QueryMsg::{GetRequests, GetState},
};
use wasmd_client::{CliWasmdClient, QueryResult, WasmdClient};

use crate::{
    proto::{settlement_server::Settlement, QueryRequest, UpdateRequest},
    transfers_server::{QueryRequestMessage, TransfersService, UpdateRequestMessage},
};

// TODO: Need to prevent listener from taking actions until handshake is completed
#[async_trait::async_trait]
impl<A: Attestor> WebSocketHandler for TransfersService<A> {
    async fn handle(&self, event: Event, config: WsListenerConfig) -> Result<()> {
        // Validation
        let is_transfer = is_transfer_event(&event);
        let is_query = is_query_event(&event);

        if !is_transfer && !is_query {
            return Ok(());
        } else {
            let mut sender = None;
            let mut contract_address = None;
            let mut emphemeral_pubkey = None;

            if let Some(events) = &event.events {
                for (key, values) in events {
                    match key.as_str() {
                        "message.sender" => {
                            sender = values.first().cloned();
                        }
                        "execute._contract_address" => {
                            contract_address = values.first().cloned();
                        }
                        "wasm-query_balance.emphemeral_pubkey" => {
                            // TODO: fix typo
                            emphemeral_pubkey = values.first().cloned();
                        }
                        _ => {}
                    }
                }
            }

            if contract_address.is_none() {
                return Ok(());
            }

            if is_transfer {
                println!("Processing transfer event");
                transfer_handler(
                    self,
                    &contract_address
                        .expect("must be included in transfers event")
                        .parse::<AccountId>()
                        .map_err(|e| anyhow!(e))?,
                    &config,
                )
                .await?;
            } else if is_query {
                println!("Processing query event");
                query_handler(
                    self,
                    &contract_address
                        .expect("must be included in query event")
                        .parse::<AccountId>()
                        .map_err(|e| anyhow!(e))?,
                    sender.expect("must be included in query event"),
                    emphemeral_pubkey.expect("must be included in query event"),
                    &config,
                )
                .await?;
            }
        }

        Ok(())
    }
}

fn is_transfer_event(event: &Event) -> bool {
    // Check if the event is a transaction type
    if let Some(EventType::Tx) = event.event_type() {
        // Check for the "wasm.action" key with the value "init_clearing"
        if let Some(events) = &event.events {
            return events.iter().any(|(key, _)| key == "wasm-transfer.action");
        }
    }
    false
}

fn is_query_event(event: &Event) -> bool {
    // Check if the event is a transaction type
    if let Some(EventType::Tx) = event.event_type() {
        // Check for the "wasm.action" key with the value "init_clearing"
        if let Some(events) = &event.events {
            return events
                .iter()
                .any(|(key, _)| key.starts_with("wasm-query_balance"));
        }
    }
    false
}

async fn transfer_handler<A: Attestor>(
    client: &TransfersService<A>,
    contract: &AccountId,
    ws_config: &WsListenerConfig,
) -> Result<()> {
    let chain_id = &ChainId::from_str(&ws_config.chain_id)?;
    let httpurl = Url::parse(&format!("http://{}", ws_config.node_url))?;
    let wasmd_client = CliWasmdClient::new(httpurl.clone());

    // Query chain
    // Get epoch, obligations, liquidity sources
    let resp: QueryResult<Vec<TransferRequest>> = wasmd_client
        .query_smart(contract, json!(GetRequests {}))
        .map_err(|e| anyhow!("Problem querying epoch: {}", e))?;
    let requests = resp.data;

    let resp: QueryResult<HexBinary> = wasmd_client
        .query_smart(contract, json!(GetState {}))
        .map_err(|e| anyhow!("Problem querying epoch: {}", e))?;
    let state = resp.data;

    // Request body contents
    let update_contents = UpdateRequestMessage { state, requests };

    // Wait 2 blocks
    info!("Waiting 2 blocks for light client proof");
    let wsurl = format!("ws://{}/websocket", ws_config.node_url);
    two_block_waitoor(&wsurl).await?;

    // Call tm prover with trusted hash and height
    let prover_config = TmProverConfig {
        primary: httpurl.as_str().parse()?,
        witnesses: httpurl.as_str().parse()?,
        trusted_height: ws_config.trusted_height,
        trusted_hash: ws_config.trusted_hash,
        verbose: "1".parse()?, // TODO: both tm-prover and cli define the same Verbosity struct. Need to define this once and import
        contract_address: contract.clone(),
        storage_key: "requests".to_string(),
        chain_id: ws_config.chain_id.to_string(),
        ..Default::default()
    };

    let proof_output = tokio::task::spawn_blocking(move || {
        // Create a new runtime inside the blocking thread.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            prove(prover_config)
                .await
                .map_err(|report| anyhow!("Tendermint prover failed. Report: {}", report))
        })
    })
    .await??; // Handle both JoinError and your custom error

    // Merge the UpdateRequestMessage with the proof
    let mut proof_json = serde_json::to_value(proof_output)?;
    proof_json["msg"] = serde_json::to_value(&update_contents)?;

    // Build final request object
    let request = Request::new(UpdateRequest {
        message: json!(proof_json).to_string(),
    });

    // Send UpdateRequestMessage request to enclave over tonic gRPC client
    let update_response = client
        .run(request)
        .await
        .map_err(|e| anyhow!("Failed to communicate to relayer. {e}"))?
        .into_inner();

    // Extract json from enclave response
    let attested: RawAttested<UpdateMsg, HexBinary> =
        serde_json::from_str(&update_response.message)
            .map_err(|e| anyhow!("Error deserializing UpdateMsg from enclave: {}", e))?;

    // Build on-chain response
    // TODO add non-mock support
    let setoffs_msg = ExecuteMsg::Update::<RawMockAttestation>(AttestedMsg {
        msg: RawAttestedMsgSansHandler(attested.msg),
        attestation: MockAttestation(
            attested
                .attestation
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("slice with incorrect length"))?,
        )
        .into(),
    });

    // Post response to chain
    let output = wasmd_client.tx_execute(
        contract,
        chain_id,
        2000000,
        &ws_config.tx_sender,
        json!(setoffs_msg),
    )?;

    println!("Output TX: {}", output);
    Ok(())
}

async fn query_handler<A: Attestor>(
    client: &TransfersService<A>,
    contract: &AccountId,
    msg_sender: String,
    pubkey: String,
    ws_config: &WsListenerConfig,
) -> Result<()> {
    let chain_id = &ChainId::from_str(&ws_config.chain_id)?;
    let httpurl = Url::parse(&format!("http://{}", ws_config.node_url))?;
    let wasmd_client = CliWasmdClient::new(httpurl);

    // Query Chain
    // Get state
    let resp: QueryResult<HexBinary> = wasmd_client
        .query_smart(contract, json!(GetState {}))
        .map_err(|e| anyhow!("Problem querying epoch: {}", e))?;
    let state = resp.data;

    // Build request
    let update_contents = QueryRequestMessage {
        state,
        address: Addr::unchecked(&msg_sender), // sender comes from TX event, therefore is checked
        ephemeral_pubkey: HexBinary::from_hex(&pubkey)?,
    };

    // Send QueryRequestMessage to enclave over tonic gRPC client
    let request = Request::new(QueryRequest {
        message: json!(update_contents).to_string(),
    });

    let query_response = client
        .query(request)
        .await
        .map_err(|e| anyhow!("Failed to communicate to relayer. {e}"))?
        .into_inner();

    // Extract json from the enclave response
    let attested: RawAttested<QueryResponseMsg, HexBinary> =
        serde_json::from_str(&query_response.message)
            .map_err(|e| anyhow!("Error deserializing QueryResponseMsg from enclave: {}", e))?;

    // Build on-chain response
    // TODO add non-mock support
    let setoffs_msg = ExecuteMsg::QueryResponse::<RawMockAttestation>(AttestedMsg {
        msg: RawAttestedMsgSansHandler(attested.msg),
        attestation: MockAttestation(
            attested
                .attestation
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("slice with incorrect length"))?,
        )
        .into(),
    });

    // Post response to chain
    let output = wasmd_client.tx_execute(
        contract,
        chain_id,
        2000000,
        &ws_config.tx_sender,
        json!(setoffs_msg),
    )?;

    println!("Output TX: {}", output);
    Ok(())
}

async fn two_block_waitoor(wsurl: &str) -> Result<(), anyhow::Error> {
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
