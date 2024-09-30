#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_import_braces,
    unused_qualifications
)]

mod cli;
mod mtcs_server;
mod proto;
mod types;
mod wslistener;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use cli::Cli;
use mtcs_server::MtcsService;
use quartz_common::{
    contract::state::{Config, LightClientOpts},
    enclave::{
        attestor::{self, Attestor},
        server::{QuartzServer, WsListenerConfig},
    },
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let light_client_opts = LightClientOpts::new(
        args.chain_id.clone(),
        args.trusted_height.into(),
        Vec::from(args.trusted_hash)
            .try_into()
            .expect("invalid trusted hash"),
        (
            args.trust_threshold.numerator(),
            args.trust_threshold.denominator(),
        ),
        args.trusting_period,
        args.max_clock_drift,
        args.max_block_lag,
    )?;

    #[cfg(not(feature = "mock-sgx"))]
    let attestor = attestor::DcapAttestor {
        fmspc: args.fmspc.expect("FMSPC is required for DCAP"),
    };

    #[cfg(feature = "mock-sgx")]
    let attestor = attestor::MockAttestor;

    let config: Config = Config::new(
        attestor.mr_enclave()?,
        Duration::from_secs(30 * 24 * 60),
        light_client_opts,
        args.tcbinfo_contract.map(|c| c.to_string()),
        args.dcap_verifier_contract.map(|c| c.to_string()),
    );

    let ws_config = WsListenerConfig {
        node_url: args.node_url,
        tx_sender: args.tx_sender,
        trusted_hash: args.trusted_hash,
        trusted_height: args.trusted_height,
        chain_id: args.chain_id,
        websocket_url: args.websocket_url,
    };

    let sk = Arc::new(Mutex::new(None));

    QuartzServer::new(config.clone(), sk.clone(), attestor.clone(), ws_config)
        .add_service(MtcsService::new(config, sk, attestor))
        .serve(args.rpc_addr)
        .await?;

    Ok(())
}
