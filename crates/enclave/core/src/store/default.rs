use cosmrs::AccountId;
use displaydoc::Display;
use quartz_contract_core::state::{Config, Nonce};

use crate::store::Store;

#[derive(Clone, Debug, Default)]
pub struct DefaultStore {
    config: Option<Config>,
    contract: Option<AccountId>,
    nonce: Option<Nonce>,
    seq_num: u64,
}

impl DefaultStore {
    pub fn new(config: Config) -> Self {
        DefaultStore {
            config: Some(config),
            contract: None,
            nonce: None,
            seq_num: 0,
        }
    }
}

#[derive(Debug, Display)]
pub enum StoreError {}

#[async_trait::async_trait]
impl Store for DefaultStore {
    type Contract = AccountId;
    type Error = StoreError;

    async fn get_config(&self) -> Result<Option<Config>, Self::Error> {
        Ok(self.config.clone())
    }

    async fn set_config(&mut self, config: Config) -> Result<Option<Config>, Self::Error> {
        Ok(self.config.replace(config))
    }

    async fn get_contract(&self) -> Result<Option<Self::Contract>, Self::Error> {
        Ok(self.contract.clone())
    }

    async fn set_contract(
        &mut self,
        contract: Self::Contract,
    ) -> Result<Option<Self::Contract>, Self::Error> {
        Ok(self.contract.replace(contract))
    }

    async fn get_nonce(&self) -> Result<Option<Nonce>, Self::Error> {
        Ok(self.nonce)
    }

    async fn set_nonce(&mut self, nonce: Nonce) -> Result<Option<Nonce>, Self::Error> {
        Ok(self.nonce.replace(nonce))
    }

    async fn get_seq_num(&self) -> Result<u64, Self::Error> {
        Ok(self.seq_num)
    }

    async fn inc_seq_num(&mut self, count: usize) -> Result<u64, Self::Error> {
        let prev_seq_num = self.seq_num;
        self.seq_num += count as u64;
        Ok(prev_seq_num)
    }
}
