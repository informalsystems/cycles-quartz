pub use cli::CliClient;
use cosmrs::tendermint::chain::Id;
pub use grpc::GrpcClient;
use hex::ToHex;
use serde::de::DeserializeOwned;

pub mod cli;

pub mod grpc;

#[async_trait::async_trait]
pub trait CwClient {
    type Address: AsRef<str>;
    type Query: ToString;
    type RawQuery: ToHex;
    type ChainId: AsRef<str>;
    type Error;

    async fn query_smart<R: DeserializeOwned + Send>(
        &self,
        contract: &Self::Address,
        query: Self::Query,
    ) -> Result<R, Self::Error>;

    async fn query_raw<R: DeserializeOwned + Default>(
        &self,
        contract: &Self::Address,
        query: Self::RawQuery,
    ) -> Result<R, Self::Error>;

    fn query_tx<R: DeserializeOwned + Default>(&self, txhash: &str) -> Result<R, Self::Error>;

    async fn tx_execute<M: ToString + Send>(
        &self,
        contract: &Self::Address,
        chain_id: &Id,
        gas: u64,
        sender: &str,
        msg: M,
        fees: &str,
    ) -> Result<String, Self::Error>;

    fn deploy<M: ToString>(
        &self,
        chain_id: &Id,
        sender: &str, // what should this type be
        wasm_path: M,
    ) -> Result<String, Self::Error>;

    fn init<M: ToString>(
        &self,
        chain_id: &Id,
        sender: &str,
        code_id: u64,
        init_msg: M,
        label: &str,
    ) -> Result<String, Self::Error>;

    fn trusted_height_hash(&self) -> Result<(u64, String), Self::Error>;
}
