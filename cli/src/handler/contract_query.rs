// use async_trait::async_trait;
// use cycles_sync::wasmd_client::{CliWasmdClient, WasmdClient};
// use reqwest::Url;
// use serde_json::Value;
// use tokio::time::{sleep, Duration};
// use tracing::info;

// use super::utils::types::WasmdTxResponse;
// use crate::{
//     error::Error,
//     handler::Handler,
//     request::contract_query::ContractQueryRequest,
//     response::{contract_query::ContractQueryResponse, Response},
//     Config,
// };

// #[async_trait]
// impl Handler for ContractQueryRequest {
//     type Error = Error;
//     type Response = Response;

//     async fn handle(self, _: Config) -> Result<Self::Response, Self::Error> {
//         let tx_hash = tx(self)
//             .await
//             .map_err(|e| Error::GenericErr(e.to_string()))?;

//         Ok(ContractQueryRequest { tx_hash }.into())
//     }
// }

// async fn query(args: ContractQueryRequest) -> Result<ContractQueryResponse, anyhow::Error> {

// }

// TODODODODODODO!!!!!!!!!!!!!!!!!!!!!!! ALL OTHER FILES ARE SETUP, JUST NEED TO IMPLEMENT THIS ONE