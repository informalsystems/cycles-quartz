use std::process::Command;

use async_trait::async_trait;
use tracing::{debug, info};

use crate::{
    config::Config,
    error::Error,
    handler::Handler,
    request::enclave_build::EnclaveBuildRequest,
    response::{enclave_build::EnclaveBuildResponse, Response},
};

#[async_trait]
impl Handler for EnclaveBuildRequest {
    type Error = Error;
    type Response = Response;

    async fn handle(self, config: Config) -> Result<Self::Response, Self::Error> {
        let mut cargo = Command::new("cargo");
        let command = cargo
            .args(["build", "--release"])
            .args(["--manifest-path", &self.manifest_path.display().to_string()]);

        if config.mock_sgx {
            debug!("Building with mock-sgx enabled");
            command.arg("--features=mock-sgx");
        }

        info!("🚧 Building enclave ...");
        let status = command
            .status()
            .map_err(|e| Error::GenericErr(e.to_string()))?;

        if !status.success() {
            return Err(Error::GenericErr(format!(
                "Couldn't build enclave. {:?}",
                status
            )));
        }

        Ok(EnclaveBuildResponse.into())
    }
}
