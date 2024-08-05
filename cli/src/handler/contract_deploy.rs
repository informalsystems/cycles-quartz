use std::env::current_dir;

use async_trait::async_trait;
use cycles_sync::wasmd_client::{CliWasmdClient, WasmdClient};
use quartz_common::contract::{
    msg::execute::attested::{EpidAttestation, MockAttestation},
    prelude::QuartzInstantiateMsg,
};
use reqwest::Url;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;
use tendermint_rpc::HttpClient;
use tracing::trace;

use super::utils::{
    helpers::{block_tx_commit, run_relay},
    types::{Log, WasmdTxResponse},
};
use crate::{
    error::Error,
    handler::Handler,
    request::contract_deploy::ContractDeployRequest,
    response::{contract_deploy::ContractDeployResponse, Response},
    Config,
};

#[async_trait]
impl Handler for ContractDeployRequest {
    type Error = Error;
    type Response = Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error> {
        trace!("initializing directory structure...");

        if config.mock_sgx {
            deploy::<MockAttestation>(self, config.mock_sgx)
                .await
                .map_err(|e| Error::GenericErr(e.to_string()))?;
        } else {
            deploy::<EpidAttestation>(self, config.mock_sgx)
                .await
                .map_err(|e| Error::GenericErr(e.to_string()))?;
        }

        Ok(ContractDeployResponse.into())
    }
}

async fn deploy<DA: Serialize + DeserializeOwned>(
    args: ContractDeployRequest,
    mock_sgx: bool,
) -> Result<(), anyhow::Error> {
    // TODO: Replace with call to Rust package
    let relay_path = current_dir()?.join("../");

    let httpurl = Url::parse(&format!("http://{}", args.node_url))?;
    let tmrpc_client = HttpClient::new(httpurl.as_str())?;
    let wasmd_client = CliWasmdClient::new(Url::parse(httpurl.as_str())?);

    println!("\n🚀 Deploying {} Contract\n", args.label);
    let contract_path = args
        .directory
        .join("contracts/cw-tee-mtcs/target/wasm32-unknown-unknown/release/cw_tee_mtcs.wasm");

    // TODO: uncertain about the path -> string conversion
    let deploy_output: WasmdTxResponse = serde_json::from_str(&wasmd_client.deploy(
        &args.chain_id,
        args.sender.clone(),
        contract_path.display().to_string(),
    )?)?;
    let res = block_tx_commit(&tmrpc_client, deploy_output.txhash).await?;

    let log: Vec<Log> = serde_json::from_str(&res.tx_result.log)?;
    let code_id: usize = log[0].events[1].attributes[1].value.parse()?;
    // let code_id: usize = 25;

    println!("\n🚀 Communicating with Relay to Instantiate...\n");
    let raw_init_msg: QuartzInstantiateMsg<DA> =
        run_relay(relay_path.as_path(), mock_sgx, "Instantiate", None)?;

    println!("\n🚀 Instantiating {} Contract\n", args.label);
    let mut init_msg = args.init_msg;
    init_msg.quartz = json!(raw_init_msg);

    let init_output: WasmdTxResponse = serde_json::from_str(&wasmd_client.init(
        &args.chain_id,
        args.sender.clone(),
        code_id,
        json!(init_msg),
        format!("{} Contract #{}", args.label, code_id),
    )?)?;
    let res = block_tx_commit(&tmrpc_client, init_output.txhash).await?;

    let log: Vec<Log> = serde_json::from_str(&res.tx_result.log)?;
    let contract_addr: &String = &log[0].events[1].attributes[0].value;

    println!("\n🚀 Successfully deployed and instantiated contract!");
    println!("🆔 Code ID: {}", code_id);
    println!("📌 Contract Address: {}", contract_addr);

    println!("{contract_addr}");
    Ok(())
}

//RES=$($CMD tx wasm instantiate "$CODE_ID" "$INSTANTIATE_MSG" --from "$USER_ADDR" --label $LABEL $TXFLAG -y --no-admin --output json)
