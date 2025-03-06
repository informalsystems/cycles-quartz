use anyhow::{anyhow, Error as AnyhowError};
use cosmrs::AccountId;
use cosmwasm_std::{Addr, HexBinary};
use quartz_common::enclave::{
    chain_client::{default::Query, ChainClient},
    handler::Handler,
};
use serde_json::json;
use tendermint_rpc::event::Event as TmEvent;
use transfers_contract::msg::QueryMsg::GetState;

use crate::{
    event::first_event_with_key, proto::QueryRequest, request::query::QueryRequestMessage,
};

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

        let contract = first_event_with_key(events, "execute._contract_address")?
            .parse::<AccountId>()
            .map_err(|e| anyhow!("failed to parse contract address: {}", e))?;
        let sender = first_event_with_key(events, "message.sender")?.to_owned();
        let ephemeral_pubkey =
            first_event_with_key(events, "wasm-query_balance.emphemeral_pubkey")?.to_owned();

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
    C: ChainClient<Contract = AccountId, Query = Query>,
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
            .query_contract(&contract, json!(GetState {}))
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
