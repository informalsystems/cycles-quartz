#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    missing_docs,
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

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use cli::Cli;
use mtcs_server::MtcsService;
use proto::clearing_server::ClearingServer as MtcsServer;
use quartz_cw::state::{Config, LightClientOpts};
use quartz_enclave::{
    attestor::{Attestor, DefaultAttestor},
    server::CoreService,
};
use quartz_proto::quartz::core_server::CoreServer;
use tonic::transport::Server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let light_client_opts = LightClientOpts::new(
        args.chain_id,
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

    let attestor = DefaultAttestor::default();

    let config = Config::new(
        attestor.mr_enclave()?,
        Duration::from_secs(30 * 24 * 60),
        light_client_opts,
    );

    let sk = Arc::new(Mutex::new(None));

    Server::builder()
        .add_service(CoreServer::new(CoreService::new(
            config,
            sk.clone(),
            attestor.clone(),
        )))
        .add_service(MtcsServer::new(MtcsService::new(sk, attestor)))
        .serve(args.rpc_addr)
        .await?;

    Ok(())
}
