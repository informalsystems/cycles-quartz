//TODO: get rid of this
use std::{collections::BTreeMap, str::FromStr};

use anyhow::{anyhow, Result};
use base64::prelude::*;
use cosmrs::{tendermint::chain::Id as ChainId, AccountId};
use cosmwasm_std::{HexBinary, Uint64};
use cw_tee_mtcs::msg::{
    execute::SubmitSetoffsMsg, AttestedMsg, ExecuteMsg, GetLiquiditySourcesResponse,
    QueryMsg::GetLiquiditySources,
};
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
// use quartz_tee_ra::{intel_sgx::epid::types::ReportBody, IASReport};
use serde_json::json;
use tendermint_rpc::{event::Event, query::EventType};
use tonic::Request;
use wasmd_client::{CliWasmdClient, QueryResult, WasmdClient};

use crate::{
    mtcs_server::MtcsService,
    proto::{clearing_server::Clearing, RunClearingRequest},
    types::RunClearingMessage,
};

// TODO: Need to prevent listener from taking actions until handshake is completed
#[async_trait::async_trait]
impl<A: Attestor> WebSocketHandler for MtcsService<A> {
    async fn handle(&self, event: Event, config: WsListenerConfig) -> Result<()> {
        // Validation
        if !is_init_clearing_event(&event) {
            return Ok(());
        } else {
            println!("Found clearing event");

            let mut sender = None;
            let mut contract_address = None;

            if let Some(events) = &event.events {
                for (key, values) in events {
                    match key.as_str() {
                        "message.sender" => {
                            sender = values.first().cloned();
                        }
                        "neutron._contract_address" => {
                            contract_address = values.first().cloned();
                        }
                        _ => {}
                    }
                }
            }

            // TODO: add some checks based on event messages

            if sender.is_none() || contract_address.is_none() {
                return Ok(()); // TODO: change return type
            }

            handler(
                self,
                &contract_address
                    .expect("infallible")
                    .parse::<AccountId>()
                    .map_err(|e| anyhow!(e))?,
                sender.expect("infallible"),
                &config.node_url,
                &config.chain_id,
            )
            .await?;
        }

        Ok(())
    }
}

fn is_init_clearing_event(event: &Event) -> bool {
    // Check if the event is a transaction type
    if let Some(EventType::Tx) = event.event_type() {
        // Check for the "wasm.action" key with the value "init_clearing"
        if let Some(events) = &event.events {
            return events.iter().any(|(key, values)| {
                key == "wasm.action" && values.contains(&"init_clearing".to_string())
            });
        }
    }
    false
}

async fn handler<A: Attestor>(
    client: &MtcsService<A>,
    contract: &AccountId,
    sender: String,
    node_url: &str,
    chain_id: &str,
) -> Result<()> {
    let chain_id = &ChainId::from_str(chain_id)?;
    let httpurl = Url::parse(node_url)?; // TODO Improve
    let wasmd_client = CliWasmdClient::new(httpurl);

    // Query obligations and liquidity sources from chain
    let clearing_contents = query_chain(&wasmd_client, contract).await?;

    // Send queried data to enclave over gRPC
    let request = Request::new(RunClearingRequest {
        message: json!(clearing_contents).to_string(),
    });

    let clearing_response = client
        .run(request)
        .await
        .map_err(|e| anyhow!("Failed to communicate to relayer. {e}"))?
        .into_inner();
    // Extract json from the Protobuf message
    let attested: RawAttested<SubmitSetoffsMsg, Vec<u8>> =
        serde_json::from_str(&clearing_response.message)
            .map_err(|e| anyhow!("Error serializing SubmitSetoffs: {}", e))?;

    // TODO add non-mock support, get IAS report and build attested message
    let msg = RawAttestedMsgSansHandler(attested.msg);

    let setoffs_msg = ExecuteMsg::SubmitSetoffs::<RawMockAttestation>(AttestedMsg {
        msg,
        attestation: MockAttestation(attested.attestation.try_into().unwrap()).into(),
    });

    // Send setoffs to mtcs contract on chain
    let output = wasmd_client.tx_execute(
        contract,
        chain_id,
        2000000,
        &sender,
        json!(setoffs_msg),
        "1000untrn",
    )?;

    println!("Setoffs TX: {}", output);
    Ok(())
}

// TODO: replace raw queries with smart
async fn query_chain(
    wasmd_client: &CliWasmdClient,
    contract: &AccountId,
) -> Result<RunClearingMessage> {
    // Get epoch counter
    let resp: QueryResult<String> = wasmd_client
        .query_raw(contract, hex::encode("epoch_counter"))
        .map_err(|e| anyhow!("Problem querying epoch: {}", e))?;

    let mut epoch_counter: usize = String::from_utf8(BASE64_STANDARD.decode(resp.data)?)?
        .trim_matches('"')
        .parse::<usize>()?;

    if epoch_counter > 1 {
        epoch_counter -= 1;
    }

    // TODO: replace with tracer log here
    // println!("epoch: {}", epoch_counter);

    // Get obligations
    let resp: QueryResult<String> = wasmd_client
        .query_raw(
            contract,
            hex::encode(format!("{}/obligations", epoch_counter)),
        )
        .map_err(|e| anyhow!("Problem querying obligatons: {}", e))?;

    let decoded_obligs = BASE64_STANDARD.decode(resp.data)?;
    let obligations_map: BTreeMap<HexBinary, HexBinary> =
        serde_json::from_slice(&decoded_obligs).unwrap_or_default();
    // println!("obligations \n {:?}", obligations_map);
    // TODO: replace with tracer log here

    // Get liquidity sources
    let resp: QueryResult<GetLiquiditySourcesResponse> = wasmd_client
        .query_smart(
            contract,
            json!(GetLiquiditySources {
                epoch: Some(Uint64::new(epoch_counter as u64))
            }),
        )
        .map_err(|e| anyhow!("Problem querying liquidity sources: {}", e))?;

    let liquidity_sources = resp.data.liquidity_sources;
    // TODO: replace with tracer log here
    // println!("liquidity_sources \n {:?}", liquidity_sources);

    Ok(RunClearingMessage {
        intents: obligations_map,
        liquidity_sources: liquidity_sources.into_iter().collect(),
    })
}

// Request the IAS report for EPID attestations
// async fn gramine_ias_request(
//     attested_msg: Vec<u8>,
//     user: &str,
// ) -> Result<EpidAttestation, anyhow::Error> {
//     let ias_api_key = String::from("669244b3e6364b5888289a11d2a1726d");
//     let ra_client_spid = String::from("51CAF5A48B450D624AEFE3286D314894");
//     let quote_file = format!("/tmp/{}_test.quote", user);
//     let report_file = format!("/tmp/{}_datareport", user);
//     let report_sig_file = format!("/tmp/{}_datareportsig", user);

//     // Write the binary data to a file
//     let mut file = File::create(&quote_file).await?;
//     file.write_all(&attested_msg)
//         .await
//         .map_err(|e| anyhow!("Couldn't write to file. {e}"))?;

//     let mut gramine = Command::new("gramine-sgx-ias-request");
//     let command = gramine
//         .arg("report")
//         .args(["-g", &ra_client_spid])
//         .args(["-k", &ias_api_key])
//         .args(["-q", &quote_file])
//         .args(["-r", &report_file])
//         .args(["-s", &report_sig_file]);

//     let output = command.output()?;
//     if !output.status.success() {
//         return Err(anyhow!("Couldn't run gramine. {:?}", output));
//     }

//     let report: ReportBody = serde_json::from_str(&fs::read_to_string(report_file).await?)?;
//     let report_sig_str = fs::read_to_string(report_sig_file).await?.replace('\r', "");
//     let report_sig: Binary = Binary::from_base64(report_sig_str.trim())?;

//     Ok(EpidAttestation::new(IASReport { report, report_sig }))
// }
