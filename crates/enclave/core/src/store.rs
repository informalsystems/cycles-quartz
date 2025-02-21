use quartz_contract_core::state::{Config, Nonce};

pub mod default;
pub mod shared;

#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
    type Contract: Send + Sync;
    type Error: ToString + Send + Sync;

    async fn get_config(&self) -> Result<Option<Config>, Self::Error>;
    async fn set_config(&mut self, config: Config) -> Result<Option<Config>, Self::Error>;
    async fn get_contract(&self) -> Result<Option<Self::Contract>, Self::Error>;
    async fn set_contract(
        &mut self,
        contract: Self::Contract,
    ) -> Result<Option<Self::Contract>, Self::Error>;
    async fn get_nonce(&self) -> Result<Option<Nonce>, Self::Error>;
    async fn set_nonce(&mut self, nonce: Nonce) -> Result<Option<Nonce>, Self::Error>;
    async fn get_seq_num(&self) -> Result<u64, Self::Error>;
    async fn inc_seq_num(&mut self, count: usize) -> Result<u64, Self::Error>;
}
