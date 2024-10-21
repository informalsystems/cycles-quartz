use std::{fs, path::Path};

use async_trait::async_trait;
use cargo_metadata::MetadataCommand;
use color_eyre::{
    eyre::{eyre, Context},
    owo_colors::OwoColorize,
    Report, Result,
};
use cosmrs::AccountId;
use quartz_common::enclave::types::Fmspc;
use reqwest::Url;
use tendermint::chain::Id;
use tokio::process::{Child, Command};
use tracing::{debug, info};

use crate::{
    config::{Config, GramineBinPrefix},
    handler::{utils::helpers::write_cache_hash_height, Handler},
    request::enclave_start::EnclaveStartRequest,
    response::{enclave_start::EnclaveStartResponse, Response},
};

#[async_trait]
impl Handler for EnclaveStartRequest {
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report> {
        let config = config.as_ref().clone();
        info!("{}", "\nPeforming Enclave Start".blue().bold());

        // Get trusted height and hash
        let (trusted_height, trusted_hash) = self
            .get_hash_height(&config)
            .wrap_err("Error getting trusted hash and height")?;
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
                return Err(eyre!("FMSPC is required if MOCK_SGX isn't set"));
            };

            let Some(tcbinfo_contract) = self.tcbinfo_contract else {
                return Err(eyre!("tcbinfo_contract is required if MOCK_SGX isn't set"));
            };

            let Some(dcap_verifier_contract) = self.dcap_verifier_contract else {
                return Err(eyre!(
                    "dcap_verifier_contract is required if MOCK_SGX isn't set"
                ));
            };

            if std::env::var("ADMIN_SK").is_err() {
                return Err(eyre!("ADMIN_SK environment variable is not set"));
            };

            let enclave_dir = fs::canonicalize(config.app_dir.join("enclave"))?;

            // gramine private key
            gramine_sgx_gen_private_key(&config.gramine_bin_prefix, &enclave_dir).await?;

            // gramine manifest
            let quartz_dir_canon = &enclave_dir.join("..");

            debug!("quartz_dir_canon: {:?}", quartz_dir_canon);

            gramine_manifest(
                &config.gramine_bin_prefix,
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
            gramine_sgx_sign(&config.gramine_bin_prefix, &enclave_dir).await?;

            // Run quartz enclave and block
            let enclave_child =
                create_gramine_sgx_child(&config.gramine_bin_prefix, &enclave_dir).await?;
            handle_process(enclave_child).await?;
        }

        Ok(EnclaveStartResponse.into())
    }
}

async fn handle_process(mut child: Child) -> Result<()> {
    let status = child.wait().await?;

    if !status.success() {
        return Err(eyre!("Couldn't build enclave. {:?}", status));
    }
    Ok(())
}

async fn create_mock_enclave_child(
    app_dir: &Path,
    release: bool,
    enclave_args: Vec<String>,
) -> Result<Child> {
    let enclave_dir = app_dir.join("enclave");
    let target_dir = app_dir.join("target");

    // Use the enclave package metadata to get the path to the program binary
    let package_name = MetadataCommand::new()
        .manifest_path(enclave_dir.join("Cargo.toml"))
        .exec()?
        .root_package()
        .ok_or("No root package found in the metadata")
        .map_err(|e| eyre!(e))?
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
    let child = command.kill_on_drop(true).spawn()?;

    Ok(child)
}

async fn gramine_sgx_gen_private_key(
    gramine_bin_prefix: &GramineBinPrefix,
    enclave_dir: &Path,
) -> Result<()> {
    // Launch the gramine-sgx-gen-private-key command
    Command::new_prefixed(gramine_bin_prefix.as_ref(), "gramine-sgx-gen-private-key")
        .current_dir(enclave_dir)
        .output()
        .await
        .map_err(|e| eyre!("Failed to execute gramine-sgx-gen-private-key: {}", e))?;

    // Continue regardless of error
    // > /dev/null 2>&1 || :  # may fail
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn gramine_manifest(
    gramine_bin_prefix: &GramineBinPrefix,
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
) -> Result<()> {
    let host = target_lexicon::HOST;
    let arch_libdir = format!(
        "/lib/{}-{}-{}",
        host.architecture, host.operating_system, host.environment
    );

    let home_dir = dirs::home_dir()
        .ok_or_else(|| eyre!("Home directory not set"))?
        .display()
        .to_string();

    let status = Command::new_prefixed(gramine_bin_prefix.as_ref(), "gramine-manifest")
        .arg("-Dlog_level=error")
        .arg(format!("-Dhome={}", home_dir))
        .arg(format!("-Darch_libdir={}", arch_libdir))
        .arg("-Dra_type=dcap")
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
        .map_err(|e| eyre!("Failed to execute gramine-manifest: {}", e))?;

    if !status.success() {
        return Err(eyre!(
            "gramine-manifest command failed with status: {:?}",
            status
        ));
    }

    Ok(())
}

async fn gramine_sgx_sign(gramine_bin_prefix: &GramineBinPrefix, enclave_dir: &Path) -> Result<()> {
    let status = Command::new_prefixed(gramine_bin_prefix.as_ref(), "gramine-sgx-sign")
        .arg("--manifest")
        .arg("quartz.manifest")
        .arg("--output")
        .arg("quartz.manifest.sgx")
        .current_dir(enclave_dir)
        .status()
        .await
        .map_err(|e| eyre!("Failed to execute gramine-sgx-sign: {}", e))?;

    if !status.success() {
        return Err(eyre!(
            "gramine-sgx-sign command failed with status: {:?}",
            status
        ));
    }

    Ok(())
}

async fn create_gramine_sgx_child(
    gramine_bin_prefix: &GramineBinPrefix,
    enclave_dir: &Path,
) -> Result<Child> {
    info!("🚧 Spawning enclave process ...");

    let child = Command::new_prefixed(gramine_bin_prefix.as_ref(), "gramine-sgx")
        .arg("./quartz")
        .kill_on_drop(true)
        .current_dir(enclave_dir)
        .spawn()
        .map_err(|e| eyre!("Failed to spawn gramine-sgx child process: {}", e))?;

    Ok(child)
}

pub trait PrefixedCommandEx: Sized {
    fn new_prefixed(prefix: &str, program: &str) -> Self;
}

impl PrefixedCommandEx for Command {
    fn new_prefixed(prefix: &str, program: &str) -> Self {
        if !prefix.is_empty() {
            let mut parts = prefix.split_whitespace();
            let prefix_command = parts.next().unwrap();
            let prefix_args: Vec<&str> = parts.collect();

            let mut command = Command::new(prefix_command);
            command.args(&prefix_args);
            command.arg(program);
            command
        } else {
            Command::new(program)
        }
    }
}
