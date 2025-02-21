use anyhow::{anyhow, Error as AnyhowError};
use cosmrs::AccountId;
use cosmwasm_std::{HexBinary, Uint64};
use quartz_common::{
    contract::state::SEQUENCE_NUM_KEY,
    enclave::{
        chain_client::{default::Query, ChainClient},
        handler::Handler,
    },
};
use serde_json::json;
use tendermint_rpc::event::Event as TmEvent;
use tracing::info;
use transfers_contract::msg::{
    execute::Request as TransferRequest,
    QueryMsg::{GetRequests, GetState},
};

use crate::{
    event::first_event_with_key, proto::UpdateRequest, request::update::UpdateRequestMessage,
};

#[derive(Clone, Debug)]
pub struct TransferEvent {
    pub contract: AccountId,
}

impl TryFrom<TmEvent> for TransferEvent {
    type Error = AnyhowError;

    fn try_from(event: TmEvent) -> Result<Self, Self::Error> {
        let Some(events) = &event.events else {
            return Err(anyhow!("no events in tx"));
        };

        if !events.keys().any(|k| k.starts_with("wasm-transfer.action")) {
            return Err(anyhow!("irrelevant event"));
        };

        let contract = first_event_with_key(events, "execute._contract_address")?
            .parse::<AccountId>()
            .map_err(|e| anyhow!("failed to parse contract address: {}", e))?;

        Ok(TransferEvent { contract })
    }
}

#[async_trait::async_trait]
impl<C> Handler<C> for TransferEvent
where
    C: ChainClient<Contract = AccountId, Query = Query>,
{
    type Error = AnyhowError;
    type Response = UpdateRequest;

    async fn handle(self, ctx: &C) -> Result<Self::Response, Self::Error> {
        let contract = self.contract;

        // Query contract state
        let requests: Vec<TransferRequest> = ctx
            .query_contract(&contract, json!(GetRequests {}))
            .await
            .map_err(|e| anyhow!("Problem querying contract state: {}", e))?;
        let state: HexBinary = ctx
            .query_contract(&contract, json!(GetState {}))
            .await
            .map_err(|e| anyhow!("Problem querying contract state: {}", e))?;
        let seq_num: Uint64 = ctx
            .query_contract(&contract, SEQUENCE_NUM_KEY.to_string())
            .await
            .map_err(|e| anyhow!("Problem querying contract state: {}", e))?;

        // Request body contents
        let update_contents = UpdateRequestMessage {
            state,
            requests,
            seq_num: seq_num.into(),
        };

        // Wait 2 blocks
        info!("Waiting 2 blocks for light client proof");
        ctx.wait_for_blocks(2)
            .await
            .map_err(|e| anyhow!("Problem waiting for proof: {}", e))?;

        // Call tm prover with trusted hash and height
        let proof = ctx
            .existence_proof(&contract, "requests")
            .await
            .map_err(|e| anyhow!("Problem getting existence proof: {}", e))?;

        // Merge the UpdateRequestMessage with the proof
        let mut proof_json = serde_json::to_value(proof)?;
        proof_json["msg"] = serde_json::to_value(&update_contents)?;

        // Build final request object
        let request = UpdateRequest {
            message: json!(proof_json).to_string(),
        };

        Ok(request)
    }
}
