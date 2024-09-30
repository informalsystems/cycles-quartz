use std::{env, net::SocketAddr};

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use cosmrs::AccountId;
use quartz_common::enclave::types::Fmspc;
use tendermint::Hash;
use tendermint_light_client::types::{Height, TrustThreshold};

fn parse_trust_threshold(s: &str) -> Result<TrustThreshold> {
    if let Some((l, r)) = s.split_once('/') {
        TrustThreshold::new(l.parse()?, r.parse()?).map_err(Into::into)
    } else {
        Err(eyre!(
            "invalid trust threshold: {s}, format must be X/Y where X and Y are integers"
        ))
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// RPC server address
    #[clap(long, default_value_t = default_rpc_addr())]
    pub rpc_addr: SocketAddr,

    /// Identifier of the chain
    #[clap(long)]
    pub chain_id: String,

    /// FMSPC (Family-Model-Stepping-Platform-Custom SKU)
    #[clap(long)]
    pub fmspc: Option<Fmspc>,

    /// TcbInfo contract address
    #[clap(long)]
    pub tcbinfo_contract: Option<AccountId>,

    /// DCAP verifier contract address
    #[clap(long)]
    pub dcap_verifier_contract: Option<AccountId>,

    /// Height of the trusted header (AKA root-of-trust)
    #[clap(long)]
    pub trusted_height: Height,

    /// Hash of the trusted header (AKA root-of-trust)
    #[clap(long)]
    pub trusted_hash: Hash,

    /// Trust threshold
    #[clap(long, value_parser = parse_trust_threshold, default_value_t = TrustThreshold::TWO_THIRDS)]
    pub trust_threshold: TrustThreshold,

    /// Trusting period, in seconds (default: two weeks)
    #[clap(long, default_value = "1209600")]
    pub trusting_period: u64,

    /// Maximum clock drift, in seconds
    #[clap(long, default_value = "5")]
    pub max_clock_drift: u64,

    /// Maximum block lag, in seconds
    #[clap(long, default_value = "5")]
    pub max_block_lag: u64,


    #[clap(long, default_value = "https://rpc-falcron.pion-1.ntrn.tech")]
    pub node_url: String,

    #[clap(long, default_value = "wss://rpc-falcron.pion-1.ntrn.tech/websocket")]
    pub websocket_url: String,

    #[clap(long, default_value = "val1")]
    pub tx_sender: String,
}

fn default_rpc_addr() -> SocketAddr {
    let port = env::var("QUARTZ_ENCLAVE_RPC_PORT").unwrap_or_else(|_| "11090".to_string());
    format!("127.0.0.1:{}", port)
        .parse()
        .expect("Invalid socket address")
}
