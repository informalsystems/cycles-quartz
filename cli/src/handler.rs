use async_trait::async_trait;

use crate::{error::Error, request::Request, response::Response, Config};

pub mod utils;
// commands
pub mod contract_deploy;
pub mod contract_build;
pub mod enclave_build;
pub mod handshake;
pub mod init;

#[async_trait]
pub trait Handler {
    type Error;
    type Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error>;
}

#[async_trait]
impl Handler for Request {
    type Error = Error;
    type Response = Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error> {
        match self {
            Request::Init(request) => request.handle(config).await,
            Request::Handshake(request) => request.handle(config).await,
            Request::ContractBuild(request) => request.handle(config).await,
            Request::ContractDeploy(request) => request.handle(config).await,
            Request::EnclaveBuild(request) => request.handle(config).await,
        }
        .map(Into::into)
    }
}
