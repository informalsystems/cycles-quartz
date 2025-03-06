use std::{path::PathBuf, time::Duration};

use async_trait::async_trait;
use color_eyre::{
    eyre::{eyre, Context},
    owo_colors::OwoColorize,
    Report, Result,
};
use quartz_common::proto::core_client::CoreClient;
use tokio::{sync::mpsc, time::sleep};
use tracing::{debug, info};
use watchexec::Watchexec;
use watchexec_signals::Signal;

use crate::{
    handler::{utils::helpers::wasmaddr_to_id, Handler},
    request::{
        contract_build::ContractBuildRequest, contract_deploy::ContractDeployRequest,
        dev::DevRequest, enclave_build::EnclaveBuildRequest, enclave_start::EnclaveStartRequest,
        handshake::HandshakeRequest,
    },
    response::{dev::DevResponse, Response},
    Config,
};

#[async_trait]
impl Handler for DevRequest {
    type Response = Response;

    async fn handle<C: AsRef<Config> + Send>(self, config: C) -> Result<Self::Response, Report> {
        let config = config.as_ref();
        info!("\nPeforming Dev");

        let (tx, rx) = mpsc::channel::<DevRebuild>(32);
        let _res = tx.send(DevRebuild::Init).await;

        if self.watch {
            tokio::spawn(watcher(tx, config.build_log_dir()?));
        }

        dev_driver(rx, &self, config.clone()).await?;

        Ok(DevResponse.into())
    }
}

#[derive(Debug, Clone)]
enum DevRebuild {
    Init,
    Enclave,
    Contract,
}

async fn dev_driver(
    mut rx: mpsc::Receiver<DevRebuild>,
    args: &DevRequest,
    config: Config,
) -> Result<(), Report> {
    // State
    let mut first_enclave_message = true;
    let mut first_contract_message = true;
    let mut contract = String::from("");

    // Shutdown enclave upon interruption

    // Drive
    while let Some(dev) = rx.recv().await {
        match dev {
            DevRebuild::Init => {
                info!("{}", "Launching quartz app...".green().bold());

                // Build enclave
                let enclave_build = EnclaveBuildRequest {};
                enclave_build.handle(&config).await?;

                // Build contract
                let contract_build = ContractBuildRequest {
                    contract_manifest: args.contract_manifest.clone(),
                };
                contract_build
                    .handle(&config)
                    .await
                    .wrap_err("Could not run `contract build`")?;

                // Start enclave in background
                spawn_enclave_start(args, &config)?;

                // Deploy new contract and perform handshake
                let res = deploy_and_handshake(None, args, &config).await;

                // Save resulting contract address or shutdown and return error
                match res {
                    Ok(res_contract) => {
                        // Set state
                        contract = res_contract;

                        info!("{}", "Enclave is listening for requests...".green().bold());
                    }
                    Err(e) => {
                        return Err(e).wrap_err("Error initializing `quartz dev`");
                    }
                }
            }
            DevRebuild::Enclave => {
                if first_enclave_message {
                    first_enclave_message = false;

                    continue;
                }
                info!("{}", "Rebuilding Enclave...".green().bold());

                info!("Waiting 1 second for the enclave to shut down");
                sleep(Duration::from_secs(1)).await;

                // Start enclave in background
                spawn_enclave_start(args, &config)?;

                // todo: should not unconditionally deploy here
                let res = deploy_and_handshake(Some(&contract), args, &config).await;

                match res {
                    Ok(res_contract) => {
                        // Set state
                        contract = res_contract;

                        info!("{}", "Enclave is listening for requests...".green().bold());
                    }
                    Err(e) => {
                        return Err(e).wrap_err("Error restarting enclave after rebuild");
                    }
                }
            }
            DevRebuild::Contract => {
                if first_contract_message {
                    first_contract_message = false;
                    continue;
                }
                info!("{}", "Rebuilding Contract...".green().bold());

                let res = deploy_and_handshake(None, args, &config).await;

                match res {
                    Ok(res_contract) => contract = res_contract,
                    Err(e) => {
                        eprintln!("Error deploying contract and handshake:");

                        return Err(e).wrap_err("Error redeploying contract after rebuild");
                    }
                }

                info!("{}", "Enclave is listening for requests...".green().bold());
            }
        }
    }

    Ok(())
}

// Spawns enclve start in a separate task which runs in the background
fn spawn_enclave_start(args: &DevRequest, config: &Config) -> Result<()> {
    // In separate process, launch the enclave
    let enclave_start = EnclaveStartRequest {
        unsafe_trust_latest: args.unsafe_trust_latest,
        fmspc: args.fmspc.clone(),
        tcbinfo_contract: args.tcbinfo_contract.clone(),
        dcap_verifier_contract: args.dcap_verifier_contract.clone(),
    };

    let config_cpy = config.clone();

    tokio::spawn(async move {
        if let Err(e) = enclave_start.handle(config_cpy).await {
            eprintln!("Error running enclave start.\n {:?}", e);
        }
    });

    Ok(())
}

// TODO: do not shutdown if cli calls fail, just print
async fn deploy_and_handshake(
    contract: Option<&str>,
    args: &DevRequest,
    config: &Config,
) -> Result<String> {
    info!("Waiting for enclave start to deploy contract and handshake");

    // Wait at most 60 seconds to connect to enclave
    let mut i = 30;
    while CoreClient::connect(format!(
        "{}:{}",
        config.enclave_rpc_addr, config.enclave_rpc_port
    ))
    .await
    .is_err()
    {
        sleep(Duration::from_secs(2)).await;
        i -= 1;

        if i == 0 {
            return Err(eyre!("Could not connect to enclave"));
        }
    }
    // Calls which interact with enclave
    info!("Successfully pinged enclave, enclave is running");

    // Deploy contract IF existing contract wasn't pass into the function
    let contract = if let Some(contract) = contract {
        info!("Contract already deployed, reusing");
        contract.to_string()
    } else {
        info!("Deploying contract");
        // Deploy Contract request
        let contract_deploy = ContractDeployRequest {
            init_msg: args.init_msg.clone(),
            label: args.label.clone(),
            contract_manifest: args.contract_manifest.clone(),
        };
        // Call handler
        let cd_res = contract_deploy
            .handle(config)
            .await
            .wrap_err("Could not run `quartz contract deploy`")?;

        if let Response::ContractDeploy(res) = cd_res {
            res.contract_addr
        } else {
            unreachable!("Unexpected response variant")
        }
    };

    // Run handshake
    info!("Running handshake on contract `{}`", contract);
    let handshake = HandshakeRequest {
        contract: wasmaddr_to_id(&contract)?,
        unsafe_trust_latest: args.unsafe_trust_latest,
    };

    let h_res = handshake
        .handle(config)
        .await
        .wrap_err("Could not run `quartz handshake`")?;
    if let Response::Handshake(res) = h_res {
        info!("Handshake complete: {}", res.pub_key);
    }

    Ok(contract)
}

async fn watcher(tx: mpsc::Sender<DevRebuild>, log_dir: PathBuf) -> Result<()> {
    let wx = Watchexec::new_async(move |mut action| {
        let tx = tx.clone();

        Box::new(async move {
            if action.signals().any(|sig| sig == Signal::Interrupt) {
                eprintln!("[Quitting...]");
                action.quit();
                return action;
            }

            // Look for updates to filepaths
            if let Some((path, _)) = action.paths().next() {
                debug!("event triggered on path:\n {:?}", path);
                if path
                    .file_name()
                    .expect("events can't trigger on nonexistent files")
                    .eq("enclave")
                {
                    let _res = tx.send(DevRebuild::Enclave).await;
                } else if path
                    .file_name()
                    .expect("events can't trigger on nonexistent files")
                    .eq("contract")
                {
                    let _res = tx.send(DevRebuild::Contract).await;
                }
            }

            action
        })
    })?;

    // Start the engine
    let main = wx.main();

    // Watch all files in quartz app directory
    // TODO: should create_log_dir be called instead? Just enforce building log in all cases?
    wx.config.pathset([log_dir]);

    // Keep running until Watchexec quits
    let _ = main.await?;

    Ok(())
}
