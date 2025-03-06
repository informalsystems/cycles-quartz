use std::sync::Arc;

use quartz_contract_core::state::{Config, Nonce};
use tokio::sync::RwLock;

use crate::store::Store;

#[derive(Clone, Debug)]
pub struct SharedStore<S> {
    inner: Arc<RwLock<S>>,
}

impl<S> SharedStore<S> {
    pub fn wrapping(store: S) -> Self {
        Self {
            inner: Arc::new(RwLock::new(store)),
        }
    }
}

#[async_trait::async_trait]
impl<S> Store for SharedStore<S>
where
    S: Store,
{
    type Contract = S::Contract;
    type Error = S::Error;

    async fn get_config(&self) -> Result<Option<Config>, Self::Error> {
        self.inner.read().await.get_config().await
    }

    async fn set_config(&mut self, config: Config) -> Result<Option<Config>, Self::Error> {
        self.inner.write().await.set_config(config).await
    }

    async fn get_contract(&self) -> Result<Option<Self::Contract>, Self::Error> {
        self.inner.read().await.get_contract().await
    }

    async fn set_contract(
        &mut self,
        contract: Self::Contract,
    ) -> Result<Option<Self::Contract>, Self::Error> {
        self.inner.write().await.set_contract(contract).await
    }

    async fn get_nonce(&self) -> Result<Option<Nonce>, Self::Error> {
        self.inner.read().await.get_nonce().await
    }

    async fn set_nonce(&mut self, nonce: Nonce) -> Result<Option<Nonce>, Self::Error> {
        self.inner.write().await.set_nonce(nonce).await
    }

    async fn get_seq_num(&self) -> Result<u64, Self::Error> {
        self.inner.read().await.get_seq_num().await
    }

    async fn inc_seq_num(&mut self, count: usize) -> Result<u64, Self::Error> {
        self.inner.write().await.inc_seq_num(count).await
    }
}
