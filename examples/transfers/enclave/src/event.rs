use std::collections::BTreeMap;

use anyhow::{anyhow, Error as AnyhowError};
use cosmrs::AccountId;
use quartz_common::enclave::{
    chain_client::{default::Query, ChainClient},
    handler::Handler,
};
use tendermint_rpc::event::Event as TmEvent;

use crate::{
    event::{query::QueryEvent, transfer::TransferEvent},
    request::EnclaveRequest,
};

pub mod query;
pub mod transfer;

#[derive(Clone, Debug)]
pub enum EnclaveEvent {
    Transfer(TransferEvent),
    Query(QueryEvent),
}

impl TryFrom<TmEvent> for EnclaveEvent {
    type Error = AnyhowError;

    fn try_from(value: TmEvent) -> Result<Self, Self::Error> {
        if let Ok(event) = TransferEvent::try_from(value.clone()) {
            Ok(Self::Transfer(event))
        } else if let Ok(event) = QueryEvent::try_from(value) {
            Ok(Self::Query(event))
        } else {
            Err(anyhow::anyhow!("unsupported event"))
        }
    }
}

#[async_trait::async_trait]
impl<C> Handler<C> for EnclaveEvent
where
    C: ChainClient<Contract = AccountId, Query = Query>,
{
    type Error = AnyhowError;
    type Response = EnclaveRequest;

    async fn handle(self, ctx: &C) -> Result<Self::Response, Self::Error> {
        match self {
            EnclaveEvent::Transfer(event) => event.handle(ctx).await.map(EnclaveRequest::Update),
            EnclaveEvent::Query(event) => event.handle(ctx).await.map(EnclaveRequest::Query),
        }
    }
}

fn first_event_with_key<'a>(
    events: &'a BTreeMap<String, Vec<String>>,
    key: &str,
) -> Result<&'a String, AnyhowError> {
    events
        .get(key)
        .ok_or_else(|| anyhow!("missing execute._contract_address in events"))?
        .first()
        .ok_or_else(|| anyhow!("execute._contract_address is empty"))
}
