use std::{fmt::Display, marker::PhantomData};

use anyhow::anyhow;
use cosmrs::AccountId;
use futures_util::StreamExt;
use log::{error, info, trace};
use reqwest::Url;
use serde::Serialize;
use tendermint_rpc::{
    event::Event as TmEvent,
    query::{EventType, Query},
    SubscriptionClient, WebSocketClient,
};
use tonic::Status;

use crate::{
    chain_client::ChainClient, event::QuartzEvent, handler::Handler, store::Store, Enclave,
};

pub type Response<R, E> = <R as Handler<E>>::Response;

#[async_trait::async_trait]
pub trait Host: Send + Sync + 'static + Sized {
    type ChainClient: ChainClient;
    type Enclave: Enclave;
    type Error: Send + Sync + 'static;
    type Event: Handler<Self::ChainClient, Response = Self::Request>;
    type Request: Handler<Self::Enclave>;

    async fn enclave_call(
        &self,
        request: Self::Request,
    ) -> Result<Response<Self::Request, Self::Enclave>, Self::Error>;

    async fn serve(self, url: Url) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug)]
pub struct DefaultHost<E, C, R, EV, GF> {
    enclave: E,
    chain_client: C,
    gas_fn: GF,
    _phantom: PhantomData<(R, EV)>,
}

impl<E, C, R, EV, GF> DefaultHost<E, C, R, EV, GF>
where
    R: Handler<E>,
    C: ChainClient,
{
    pub fn new(enclave: E, chain_client: C, gas_fn: GF) -> Self {
        Self {
            enclave,
            chain_client,
            gas_fn,
            _phantom: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl<E, C, R, EV, GF> Host for DefaultHost<E, C, R, EV, GF>
where
    E: Enclave,
    <E as Enclave>::Store: Store<Contract = AccountId>,
    C: ChainClient<Contract = AccountId, Error = anyhow::Error>,
    <C as ChainClient>::TxOutput: Display,
    R: Handler<E, Error = Status>,
    <R as Handler<E>>::Response: Serialize + Send + Sync + 'static,
    EV: Handler<C, Response = R, Error = anyhow::Error>,
    EV: TryFrom<TmEvent, Error = anyhow::Error>,
    <EV as TryFrom<TmEvent>>::Error: Send + Sync + 'static,
    GF: Fn(&<R as Handler<E>>::Response) -> <C as ChainClient>::TxConfig + Send + Sync + 'static,
{
    type ChainClient = C;
    type Enclave = E;
    type Error = anyhow::Error;
    type Event = QuartzEvent<EV>;
    type Request = R;

    async fn enclave_call(
        &self,
        request: Self::Request,
    ) -> Result<Response<Self::Request, Self::Enclave>, Self::Error> {
        request
            .handle(&self.enclave)
            .await
            .map_err(|e| anyhow!("enclave call failed: {}", e))
    }

    async fn serve(self, url: Url) -> Result<(), Self::Error> {
        let (client, driver) = WebSocketClient::new(url.as_str()).await.unwrap();
        let driver_handle = tokio::spawn(async move { driver.run().await });

        let mut subs = client.subscribe(Query::from(EventType::Tx)).await.unwrap();
        while let Some(Ok(event)) = subs.next().await {
            trace!("Received event");

            let event = match Self::Event::try_from(event) {
                Ok(e) => e,
                Err(e) => {
                    trace!("Failed to decode event: {e}");
                    continue;
                }
            };

            // Make sure the contract in the event is the same as the paired contract.
            // This check is not really required since the proof-of-publication check will check
            // if there is a mismatch anyway, but it allows us to short-circuit here.
            let contract = event.contract.clone();
            let expected_contract = self
                .enclave
                .store()
                .await
                .get_contract()
                .await
                .map_err(|_| anyhow!("contract read failure"))?
                .expect("contract must be set");
            if contract != expected_contract {
                error!("contract != expected_contract");
                continue;
            }

            let request = event.handle(&self.chain_client).await?;
            let response = self.enclave_call(request).await?;
            let tx_config = (self.gas_fn)(&response);
            let output = self
                .chain_client
                .send_tx(&contract, response, tx_config)
                .await?;
            info!("tx output: {output}");
        }

        client.close().expect("Failed to close client");
        let _ = driver_handle.await;

        Ok(())
    }
}
