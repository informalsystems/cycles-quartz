use std::process::Command;

use async_trait::async_trait;
use tracing::{debug, trace};

use crate::{
    error::Error,
    handler::Handler,
    request::contract_build::ContractBuildRequest,
    response::{contract_build::ContractBuildResponse, Response},
    Config,
};

#[async_trait]
impl Handler for ContractBuildRequest {
    type Error = Error;
    type Response = Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error> {
        let mut cargo = Command::new("cargo");
        let command = cargo
            .arg("wasm")
            .args(["--manifest-path", &self.manifest_path.display().to_string()])
            .env("RUSTFLAGS", "-C link-arg=-s");

        if config.mock_sgx {
            debug!("Building with mock-sgx enabled");
            command.arg("--features=mock-sgx");
        }

        trace!("🚧 Building contract binary ...");
        let status = command
            .status()
            .map_err(|e| Error::GenericErr(e.to_string()))?;

        if !status.success() {
            return Err(Error::GenericErr(format!(
                "Couldn't build contract. \n{:?}",
                status
            )));
        }

        Ok(ContractBuildResponse.into())
    }
}