use std::{fs, path::Path};

use async_trait::async_trait;
use cargo_metadata::MetadataCommand;
use color_eyre::owo_colors::OwoColorize;
use cosmrs::AccountId;
use quartz_common::enclave::types::Fmspc;
use reqwest::Url;
use tendermint::chain::Id;
use tokio::process::{Child, Command};
use tracing::{debug, info};

use crate::{
    config::Config,
    error::Error,
    handler::{utils::helpers::write_cache_hash_height, Handler},
    request::enclave_start::EnclaveStartRequest,
    response::{enclave_start::EnclaveStartResponse, Response},
};

#[async_trait]
impl Handler for EnclaveStartRequest {
    type Error = Error;
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(
        self,
        config: C,
    ) -> Result<Self::Response, Self::Error> {
        let config = config.as_ref().clone();
        info!("{}", "\nPeforming Enclave Start".blue().bold());

        // Get trusted height and hash
        let (trusted_height, trusted_hash) = self.get_hash_height(&config)?;
        write_cache_hash_height(trusted_height, trusted_hash, &config).await?;

        if config.mock_sgx {
            let enclave_args: Vec<String> = vec![
                "--chain-id".to_string(),
                config.chain_id.to_string(),
                "--trusted-height".to_string(),
                trusted_height.to_string(),
                "--trusted-hash".to_string(),
                trusted_hash.to_string(),
                "--node-url".to_string(),
                config.node_url.to_string(),
                "--ws-url".to_string(),
                config.ws_url.to_string(),
                "--grpc-url".to_string(),
                config.grpc_url.to_string(),
                "--tx-sender".to_string(),
                config.tx_sender,
            ];

            // Run quartz enclave and block
            let enclave_child =
                create_mock_enclave_child(config.app_dir.as_path(), config.release, enclave_args)
                    .await?;
            handle_process(enclave_child).await?;
        } else {
            let Some(fmspc) = self.fmspc else {
                return Err(Error::GenericErr(
                    "FMSPC is required if MOCK_SGX isn't set".to_string(),
                ));
            };

            let Some(tcbinfo_contract) = self.tcbinfo_contract else {
                return Err(Error::GenericErr(
                    "tcbinfo_contract is required if MOCK_SGX isn't set".to_string(),
                ));
            };

            let Some(dcap_verifier_contract) = self.dcap_verifier_contract else {
                return Err(Error::GenericErr(
                    "dcap_verifier_contract is required if MOCK_SGX isn't set".to_string(),
                ));
            };

            if let Err(_) = std::env::var("ADMIN_SK") {
                return Err(Error::GenericErr(
                    "ADMIN_SK environment variable is not set".to_string(),
                ));
            };

            let enclave_dir = fs::canonicalize(config.app_dir.join("enclave"))?;

            // gramine private key
            gramine_sgx_gen_private_key(&enclave_dir).await?;

            // gramine manifest
            let quartz_dir_canon = &enclave_dir.join("..");

            debug!("quartz_dir_canon: {:?}", quartz_dir_canon);

            gramine_manifest(
                &trusted_height.to_string(),
                &trusted_hash.to_string(),
                &config.chain_id,
                quartz_dir_canon,
                &enclave_dir,
                fmspc,
                tcbinfo_contract,
                dcap_verifier_contract,
                &config.node_url,
                &config.ws_url,
                &config.grpc_url,
            )
            .await?;

            // gramine sign
            gramine_sgx_sign(&enclave_dir).await?;

            // Run quartz enclave and block
            let enclave_child = create_gramine_sgx_child(&enclave_dir).await?;
            handle_process(enclave_child).await?;
        }

        Ok(EnclaveStartResponse.into())
    }
}

async fn handle_process(mut child: Child) -> Result<(), Error> {
    let status = child
        .wait()
        .await
        .map_err(|e| Error::GenericErr(e.to_string()))?;

    if !status.success() {
        return Err(Error::GenericErr(format!(
            "Couldn't build enclave. {:?}",
            status
        )));
    }
    Ok(())
}

async fn create_mock_enclave_child(
    app_dir: &Path,
    release: bool,
    enclave_args: Vec<String>,
) -> Result<Child, Error> {
    let enclave_dir = app_dir.join("enclave");
    let target_dir = app_dir.join("target");

    // Use the enclave package metadata to get the path to the program binary
    let package_name = MetadataCommand::new()
        .manifest_path(enclave_dir.join("Cargo.toml"))
        .exec()
        .map_err(|e| Error::GenericErr(e.to_string()))?
        .root_package()
        .ok_or("No root package found in the metadata")
        .map_err(|e| Error::GenericErr(e.to_string()))?
        .name
        .clone();

    let executable = if release {
        target_dir.join("release").join(package_name)
    } else {
        target_dir.join("debug").join(package_name)
    };

    let mut command = Command::new(executable.display().to_string());

    command.args(enclave_args);

    debug!("Enclave Start Command: {:?}", command);

    info!("{}", "🚧 Spawning enclave process ...".green().bold());
    let child = command
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::GenericErr(e.to_string()))?;

    Ok(child)
}

async fn gramine_sgx_gen_private_key(enclave_dir: &Path) -> Result<(), Error> {
    // Launch the gramine-sgx-gen-private-key command
    Command::new("gramine-sgx-gen-private-key")
        .current_dir(enclave_dir)
        .output()
        .await
        .map_err(|e| {
            Error::GenericErr(format!(
                "Failed to execute gramine-sgx-gen-private-key: {}",
                e
            ))
        })?;

    // Continue regardless of error
    // > /dev/null 2>&1 || :  # may fail
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn gramine_manifest(
    trusted_height: &str,
    trusted_hash: &str,
    chain_id: &Id,
    quartz_dir: &Path,
    enclave_dir: &Path,
    fmspc: Fmspc,
    tcbinfo_contract: AccountId,
    dcap_verifier_contract: AccountId,
    node_url: &Url,
    ws_url: &Url,
    grpc_url: &Url,
) -> Result<(), Error> {
    let host = target_lexicon::HOST;
    let arch_libdir = format!(
        "/lib/{}-{}-{}",
        host.architecture, host.operating_system, host.environment
    );

    let ra_client_spid = "51CAF5A48B450D624AEFE3286D314894";
    let home_dir = dirs::home_dir()
        .ok_or(Error::GenericErr("home dir not set".to_string()))?
        .display()
        .to_string();

    let status = Command::new("gramine-manifest")
        .arg("-Dlog_level=error")
        .arg(format!("-Dhome={}", home_dir))
        .arg(format!("-Darch_libdir={}", arch_libdir))
        .arg("-Dra_type=dcap")
        .arg(format!("-Dra_client_spid={}", ra_client_spid))
        .arg("-Dra_client_linkable=1")
        .arg(format!("-Dchain_id={}", chain_id))
        .arg(format!("-Dquartz_dir={}", quartz_dir.display()))
        .arg(format!("-Dtrusted_height={}", trusted_height))
        .arg(format!("-Dtrusted_hash={}", trusted_hash))
        .arg(format!("-Dfmspc={}", hex::encode(fmspc)))
        .arg(format!("-Dnode_url={}", node_url))
        .arg(format!("-Dws_url={}", ws_url))
        .arg(format!("-Dgrpc_url={}", grpc_url))
        .arg(format!("-Dtcbinfo_contract={}", tcbinfo_contract))
        .arg(format!(
            "-Ddcap_verifier_contract={}",
            dcap_verifier_contract
        ))
        .arg("quartz.manifest.template")
        .arg("quartz.manifest")
        .current_dir(enclave_dir)
        .status()
        .await
        .map_err(|e| Error::GenericErr(e.to_string()))?;

    if !status.success() {
        return Err(Error::GenericErr(format!(
            "Couldn't run gramine manifest. {:?}",
            status
        )));
    }

    Ok(())
}

async fn gramine_sgx_sign(enclave_dir: &Path) -> Result<(), Error> {
    let status = Command::new("gramine-sgx-sign")
        .arg("--manifest")
        .arg("quartz.manifest")
        .arg("--output")
        .arg("quartz.manifest.sgx")
        .current_dir(enclave_dir)
        .status()
        .await
        .map_err(|e| Error::GenericErr(e.to_string()))?;

    if !status.success() {
        return Err(Error::GenericErr(format!(
            "gramine-sgx-sign command failed. {:?}",
            status
        )));
    }

    Ok(())
}

async fn create_gramine_sgx_child(enclave_dir: &Path) -> Result<Child, Error> {
    info!("🚧 Spawning enclave process ...");

    let child = Command::new("gramine-sgx")
        .arg("./quartz")
        .kill_on_drop(true)
        .current_dir(enclave_dir)
        .spawn()?;

    Ok(child)
}
