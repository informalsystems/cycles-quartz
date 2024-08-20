use async_trait::async_trait;

use crate::{config::Config, error::Error, request::Request, response::Response};

pub mod utils;
// commands
pub mod contract_build;
pub mod contract_deploy;
pub mod contract_query;
pub mod enclave_build;
pub mod enclave_start;
pub mod handshake;
pub mod init;

#[async_trait]
pub trait Handler {
    type Error;
    type Response;

    async fn handle<C: AsRef<Config> + Send>(
        self,
        config: C,
    ) -> Result<Self::Response, Self::Error>;
}

#[async_trait]
impl Handler for Request {
    type Error = Error;
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(
        self,
        config: C,
    ) -> Result<Self::Response, Self::Error> {
        match self {
            Request::Init(request) => request.handle(config).await,
            Request::Handshake(request) => request.handle(config).await,
            Request::ContractBuild(request) => request.handle(config).await,
            Request::ContractDeploy(request) => request.handle(config).await,
            Request::ContractQuery(request) => request.handle(config).await,
            Request::EnclaveBuild(request) => request.handle(config).await,
            Request::EnclaveStart(request) => request.handle(config).await,
        }
        .map(Into::into)
    }
}
