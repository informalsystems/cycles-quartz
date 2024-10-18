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
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::oneshot,
};
use tracing::{debug, error, info};

use crate::{
    config::Config,
    handler::{utils::helpers::write_cache_hash_height, Handler},
    request::enclave_start::EnclaveStartRequest,
    response::{enclave_start::EnclaveStartResponse, Response},
};

#[async_trait]
impl Handler for EnclaveStartRequest {
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report> {
        let config = config.as_ref().clone();
        info!("{}", "\nPerforming Enclave Start".blue().bold());

        // Get trusted height and hash
        let (trusted_height, trusted_hash) = self
            .get_hash_height(&config)
            .wrap_err("Error getting trusted hash and height")?;
        write_cache_hash_height(trusted_height, trusted_hash, &config).await?;

        // Create a channel to signal when the enclave process is done
        let (tx, rx) = oneshot::channel();

        // Spawn the enclave process in the background
        tokio::spawn(async move {
            if let Err(e) = start_enclave(
                self,
                &config,
                trusted_height.to_string(),
                trusted_hash.to_string(),
                tx,
            )
            .await
            {
                error!("Error starting enclave: {:?}", e);
            }
        });

        // Wait for the enclave process to complete
        rx.await.expect("Failed to receive completion signal");

        Ok(EnclaveStartResponse.into())
    }
}

async fn start_enclave(
    request: EnclaveStartRequest,
    config: &Config,
    trusted_height: String,
    trusted_hash: String,
    tx: oneshot::Sender<()>,
) -> Result<()> {
    let result = if config.mock_sgx {
        start_mock_enclave(config, trusted_height, trusted_hash).await
    } else {
        start_real_enclave(&request, config, trusted_height, trusted_hash).await
    };

    // Signal that the enclave process has completed
    let _ = tx.send(());

    result
}

async fn start_mock_enclave(
    config: &Config,
    trusted_height: String,
    trusted_hash: String,
) -> Result<()> {
    let enclave_args = vec![
        "--chain-id".to_string(),
        config.chain_id.to_string(),
        "--trusted-height".to_string(),
        trusted_height,
        "--trusted-hash".to_string(),
        trusted_hash,
        "--node-url".to_string(),
        config.node_url.to_string(),
        "--ws-url".to_string(),
        config.ws_url.to_string(),
        "--grpc-url".to_string(),
        config.grpc_url.to_string(),
        "--tx-sender".to_string(),
        config.tx_sender.clone(),
    ];

    let mut child =
        create_mock_enclave_child(&config.app_dir, config.release, enclave_args).await?;
    handle_process(&mut child).await
}

async fn create_mock_enclave_child(
    app_dir: &Path,
    release: bool,
    enclave_args: Vec<String>,
) -> Result<Child> {
    let enclave_dir = app_dir.join("enclave");
    let target_dir = app_dir.join("target");

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

    command
        .args(enclave_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    debug!("Enclave Start Command: {:?}", command);

    info!("{}", "🚧 Spawning enclave process ...".green().bold());
    let child = command
        .spawn()
        .wrap_err("Failed to spawn enclave process")?;

    info!("Spawned process with PID: {:?}", child.id());

    Ok(child)
}

async fn handle_process(child: &mut Child) -> Result<()> {
    info!("Enclave process is running...");

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| eyre!("Failed to capture stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| eyre!("Failed to capture stderr"))?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    loop {
        tokio::select! {
            result = stdout_reader.next_line() => {
                match result {
                    Ok(Some(line)) => info!("Enclave stdout: {}", line),
                    Ok(None) => break,
                    Err(e) => error!("Error reading stdout: {:?}", e),
                }
            }
            result = stderr_reader.next_line() => {
                match result {
                    Ok(Some(line)) => error!("Enclave stderr: {}", line),
                    Ok(None) => break,
                    Err(e) => error!("Error reading stderr: {:?}", e),
                }
            }
            result = child.wait() => {
                match result {
                    Ok(status) => {
                        if status.success() {
                            info!("Enclave process terminated successfully.");
                        } else {
                            error!("Enclave process terminated with status: {:?}", status);
                        }
                        return Ok(());
                    }
                    Err(e) => return Err(eyre!("Error waiting for enclave process: {:?}", e)),
                }
            }
        }
    }

    Ok(())
}

async fn start_real_enclave(
    request: &EnclaveStartRequest,
    config: &Config,
    trusted_height: String,
    trusted_hash: String,
) -> Result<()> {
    let fmspc = request
        .fmspc
        .as_ref()
        .ok_or_else(|| eyre!("FMSPC is required if MOCK_SGX isn't set"))?;
    let tcbinfo_contract = request
        .tcbinfo_contract
        .as_ref()
        .ok_or_else(|| eyre!("tcbinfo_contract is required if MOCK_SGX isn't set"))?;
    let dcap_verifier_contract = request
        .dcap_verifier_contract
        .as_ref()
        .ok_or_else(|| eyre!("dcap_verifier_contract is required if MOCK_SGX isn't set"))?;

    if std::env::var("ADMIN_SK").is_err() {
        return Err(eyre!("ADMIN_SK environment variable is not set"));
    }

    let enclave_dir = fs::canonicalize(config.app_dir.join("enclave"))?;

    // Gramine setup
    gramine_sgx_gen_private_key(&enclave_dir).await?;
    let quartz_dir_canon = &enclave_dir.join("..");
    gramine_manifest(
        &trusted_height,
        &trusted_hash,
        &config.chain_id,
        quartz_dir_canon,
        &enclave_dir,
        fmspc.clone(),
        tcbinfo_contract.clone(),
        dcap_verifier_contract.clone(),
        &config.node_url,
        &config.ws_url,
        &config.grpc_url,
    )
    .await?;
    gramine_sgx_sign(&enclave_dir).await?;

    // Start Gramine SGX enclave
    let mut enclave_child = create_gramine_sgx_child(&enclave_dir).await?;
    handle_process(&mut enclave_child).await
}

async fn gramine_sgx_gen_private_key(enclave_dir: &Path) -> Result<()> {
    // Launch the gramine-sgx-gen-private-key command
    Command::new("gramine-sgx-gen-private-key")
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

    let status = Command::new("gramine-manifest")
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

async fn gramine_sgx_sign(enclave_dir: &Path) -> Result<()> {
    let status = Command::new("gramine-sgx-sign")
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

async fn create_gramine_sgx_child(enclave_dir: &Path) -> Result<Child> {
    info!("🚧 Spawning enclave process ...");

    let child = Command::new("gramine-sgx")
        .arg("./quartz")
        .kill_on_drop(true)
        .current_dir(enclave_dir)
        .spawn()
        .map_err(|e| eyre!("Failed to spawn gramine-sgx child process: {}", e))?;

    Ok(child)
}
