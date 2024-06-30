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

pub mod cli;
pub mod proto;
pub mod state;
pub mod transfers_server;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use cli::Cli;
use proto::settlement_server::SettlementServer as TransfersServer;
use quartz_cw::state::{Config, LightClientOpts};
use quartz_enclave::{
    attestor::{Attestor, EpidAttestor},
    server::CoreService,
};
use quartz_proto::quartz::core_server::CoreServer;
use tonic::transport::Server;
use transfers_server::TransfersService;

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

    let config = Config::new(
        EpidAttestor.mr_enclave()?,
        Duration::from_secs(30 * 24 * 60),
        light_client_opts,
    );

    let sk = Arc::new(Mutex::new(None));

    Server::builder()
        .add_service(CoreServer::new(CoreService::new(
            config,
            sk.clone(),
            EpidAttestor,
        )))
        .add_service(TransfersServer::new(TransfersService::<EpidAttestor>::new(
            sk.clone(),
            EpidAttestor,
        )))
        .serve(args.rpc_addr)
        .await?;

    Ok(())
}
