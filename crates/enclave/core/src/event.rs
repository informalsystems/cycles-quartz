use anyhow::{anyhow, Error as AnyhowError};
use cosmrs::AccountId;
use tendermint_rpc::event::Event as TmEvent;

use crate::{chain_client::ChainClient, handler::Handler};

#[derive(Clone, Debug)]
pub struct QuartzEvent<E> {
    pub contract: AccountId,
    inner: E,
}

impl<E> TryFrom<TmEvent> for QuartzEvent<E>
where
    E: TryFrom<TmEvent, Error = AnyhowError>,
{
    type Error = AnyhowError;

    fn try_from(event: TmEvent) -> Result<Self, Self::Error> {
        let Some(events) = &event.events else {
            return Err(anyhow!("no events in tx"));
        };

        let contract = events
            .get("execute._contract_address")
            .ok_or_else(|| anyhow!("missing execute._contract_address in events"))?
            .first()
            .ok_or_else(|| anyhow!("execute._contract_address is empty"))?
            .parse::<AccountId>()
            .map_err(|e| anyhow!("failed to parse contract address: {}", e))?;

        Ok(QuartzEvent {
            contract,
            inner: event.try_into()?,
        })
    }
}

#[async_trait::async_trait]
impl<C, E> Handler<C> for QuartzEvent<E>
where
    C: ChainClient<Contract = AccountId>,
    E: Handler<C, Error = AnyhowError>,
{
    type Error = AnyhowError;
    type Response = <E as Handler<C>>::Response;

    async fn handle(self, ctx: &C) -> Result<Self::Response, Self::Error> {
        self.inner.handle(ctx).await
    }
}
