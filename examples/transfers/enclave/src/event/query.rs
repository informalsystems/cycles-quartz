use anyhow::{anyhow, Error as AnyhowError};
use cosmrs::AccountId;
use cosmwasm_std::{Addr, HexBinary};
use quartz_common::enclave::{chain_client::ChainClient, handler::Handler};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tendermint_rpc::event::Event as TmEvent;
use transfers_contract::msg::QueryMsg::GetState;

use crate::proto::QueryRequest;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRequestMessage {
    pub state: HexBinary,
    pub address: Addr,
    pub ephemeral_pubkey: HexBinary,
}

#[derive(Clone, Debug)]
pub struct QueryEvent {
    pub contract: AccountId,
    pub sender: String,
    pub ephemeral_pubkey: String,
}

impl TryFrom<TmEvent> for QueryEvent {
    type Error = AnyhowError;

    fn try_from(event: TmEvent) -> Result<Self, Self::Error> {
        let Some(events) = &event.events else {
            return Err(anyhow!("no events in tx"));
        };

        if !events.keys().any(|k| k.starts_with("wasm-transfer.action")) {
            return Err(anyhow!("irrelevant event"));
        };

        let contract = events
            .get("execute._contract_address")
            .ok_or_else(|| anyhow!("missing execute._contract_address in events"))?
            .first()
            .ok_or_else(|| anyhow!("execute._contract_address is empty"))?
            .parse::<AccountId>()
            .map_err(|e| anyhow!("failed to parse contract address: {}", e))?;

        let sender = events
            .get("message.sender")
            .ok_or_else(|| anyhow!("Missing message.sender in events"))?
            .first()
            .ok_or_else(|| anyhow!("execute.sender is empty"))?
            .to_owned();

        let ephemeral_pubkey = events
            .get("wasm-query_balance.emphemeral_pubkey")
            .ok_or_else(|| anyhow!("Missing wasm-query_balance.emphemeral_pubkey in events"))?
            .first()
            .ok_or_else(|| anyhow!("execute.query_balance.emphemeral_pubkey is empty"))?
            .to_owned();

        Ok(QueryEvent {
            contract,
            sender,
            ephemeral_pubkey,
        })
    }
}

#[async_trait::async_trait]
impl<C> Handler<C> for QueryEvent
where
    C: ChainClient<Contract = AccountId, Query = String>,
{
    type Error = AnyhowError;
    type Response = QueryRequest;

    async fn handle(self, ctx: &C) -> Result<Self::Response, Self::Error> {
        let QueryEvent {
            contract,
            sender,
            ephemeral_pubkey,
        } = self;

        // Query contract state
        let state: HexBinary = ctx
            .query_contract(&contract, json!(GetState {}).to_string())
            .await
            .map_err(|e| anyhow!("Problem querying contract state: {}", e))?;

        // Build request
        let update_contents = QueryRequestMessage {
            state,
            address: Addr::unchecked(sender), // sender comes from TX event, therefore is checked
            ephemeral_pubkey: HexBinary::from_hex(&ephemeral_pubkey)?,
        };

        // Send QueryRequestMessage to enclave over tonic gRPC client
        let request = QueryRequest {
            message: json!(update_contents).to_string(),
        };

        Ok(request)
    }
}
