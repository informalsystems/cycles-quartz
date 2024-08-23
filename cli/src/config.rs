use std::{env, path::PathBuf};

use cosmrs::tendermint::chain::Id as ChainId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Enable mock SGX mode for testing purposes.
    /// This flag disables the use of an Intel SGX processor and allows the system to run without remote attestations.
    #[serde(default)]
    pub mock_sgx: bool,

    /// Name or address of private key with which to sign
    #[serde(default = "default_tx_sender")]
    pub tx_sender: String,

    /// The network chain ID
    #[serde(default = "default_chain_id")]
    pub chain_id: ChainId,

    /// <host>:<port> to tendermint rpc interface for this chain
    #[serde(default = "default_node_url")]
    pub node_url: String,

    /// RPC interface for the Quartz enclave
    #[serde(default = "default_rpc_addr")]
    pub enclave_rpc_addr: String,

    /// Port enclave is listening on
    #[serde(default = "default_port")]
    pub enclave_rpc_port: u16,

    /// Path to Quartz app directory
    /// Defaults to current working dir
    #[serde(default = "default_app_dir")]
    pub app_dir: PathBuf,

    /// Trusted height for light client proofs
    #[serde(default)]
    pub trusted_height: u64,

    /// Trusted hash for block at trusted_height for light client proofs
    #[serde(default)]
    pub trusted_hash: String,
}

fn default_rpc_addr() -> String {
    env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1".to_string())
}

fn default_node_url() -> String {
    env::var("NODE_URL").unwrap_or_else(|_| "http://127.0.0.1:26657".to_string())
}

fn default_tx_sender() -> String {
    String::from("admin")
}

fn default_chain_id() -> ChainId {
    "testing".parse().expect("default chain_id failed")
}

fn default_port() -> u16 {
    11090
}

fn default_app_dir() -> PathBuf {
    ".".parse().expect("default app_dir pathbuf failed")
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mock_sgx: false,
            tx_sender: default_tx_sender(),
            chain_id: default_chain_id(),
            node_url: default_node_url(),
            enclave_rpc_addr: default_rpc_addr(),
            enclave_rpc_port: default_port(),
            app_dir: default_app_dir(),
            trusted_height: u64::default(),
            trusted_hash: String::default(),
        }
    }
}

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}
