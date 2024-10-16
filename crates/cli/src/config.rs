use std::{path::PathBuf, str::FromStr};

use cosmrs::tendermint::chain::Id as ChainId;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
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
    #[serde_as(as = "DisplayFromStr")]
    pub node_url: Url,

    /// websocket URL
    #[serde(default = "default_ws_url")]
    #[serde_as(as = "DisplayFromStr")]
    pub ws_url: Url,

    /// gRPC URL
    #[serde(default = "default_grpc_url")]
    #[serde_as(as = "DisplayFromStr")]
    pub grpc_url: Url,

    /// RPC interface for the Quartz enclave
    #[serde(default = "default_rpc_addr")]
    pub enclave_rpc_addr: String,

    /// Port enclave is listening on
    #[serde(default = "default_port")]
    pub enclave_rpc_port: u16,

    /// Path to Quartz app directory.
    /// Defaults to current working dir
    #[serde(default = "default_app_dir", skip_serializing)]
    pub app_dir: PathBuf,

    /// Trusted height for light client proofs
    #[serde(default)]
    pub trusted_height: u64,

    /// Trusted hash for block at trusted_height for light client proofs
    #[serde(default)]
    pub trusted_hash: String,

    /// Whether to build for release or debug
    #[serde(default)]
    pub release: bool,

    /// Prefix for gramine commands (to be used if docker is preferred)
    #[serde(default)]
    pub gramine_bin_prefix: GramineBinPrefix,
}

fn default_rpc_addr() -> String {
    "http://127.0.0.1".to_string()
}

fn default_node_url() -> Url {
    "http://127.0.0.1:26657"
        .parse()
        .expect("valid hardcoded URL")
}

fn default_ws_url() -> Url {
    "ws://127.0.0.1/websocket"
        .parse()
        .expect("valid hardcoded URL")
}

fn default_grpc_url() -> Url {
    "http://127.0.0.1:9090"
        .parse()
        .expect("valid hardcoded URL")
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct GramineBinPrefix(String);

impl Default for GramineBinPrefix {
    fn default() -> Self {
        Self::docker_stable_jammy()
    }
}

impl GramineBinPrefix {
    pub fn empty() -> Self {
        Self("".to_string())
    }

    pub fn docker_stable_jammy() -> Self {
        GramineBinPrefix("docker run --device /dev/sgx_enclave --device /dev/sgx_provision  gramineproject/gramine:stable-jammy".to_string())
    }

    pub fn docker_stable_focal() -> Self {
        GramineBinPrefix("docker run --device /dev/sgx_enclave --device /dev/sgx_provision  gramineproject/gramine:stable-focal".to_string())
    }
}

impl FromStr for GramineBinPrefix {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO(hu55a1n1): validation
        Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for GramineBinPrefix {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mock_sgx: false,
            tx_sender: default_tx_sender(),
            chain_id: default_chain_id(),
            node_url: default_node_url(),
            ws_url: default_ws_url(),
            grpc_url: default_grpc_url(),
            enclave_rpc_addr: default_rpc_addr(),
            enclave_rpc_port: default_port(),
            app_dir: default_app_dir(),
            trusted_height: u64::default(),
            trusted_hash: String::default(),
            release: false,
            gramine_bin_prefix: Default::default(),
        }
    }
}

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}

impl Config {
    pub fn enclave_rpc(&self) -> String {
        format!("{}:{}", self.enclave_rpc_addr, self.enclave_rpc_port)
    }
}
