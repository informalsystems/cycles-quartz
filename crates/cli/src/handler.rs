use async_trait::async_trait;
use color_eyre::{Report, Result};

use crate::{config::Config, request::Request, response::Response};

pub mod utils;
// commands
pub mod contract_build;
pub mod contract_deploy;
pub mod dev;
pub mod enclave_build;
pub mod enclave_start;
pub mod handshake;
pub mod init;
pub mod print_fmspc;

#[async_trait]
pub trait Handler {
    type Response;

    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report>;
}

#[async_trait]
impl Handler for Request {
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report> {
        match self {
            Request::Init(request) => request.handle(config).await,
            Request::Handshake(request) => request.handle(config).await,
            Request::ContractBuild(request) => request.handle(config).await,
            Request::ContractDeploy(request) => request.handle(config).await,
            Request::EnclaveBuild(request) => request.handle(config).await,
            Request::EnclaveStart(request) => request.handle(config).await,
            Request::Dev(request) => request.handle(config).await,
            Request::PrintFmspc(request) => request.handle(config).await,
        }
        .map(Into::into)
    }
}
