use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cosmrs::{tendermint::chain::Id as ChainId, AccountId};
use figment::{providers::Serialized, Figment};
use serde::{Deserialize, Serialize};
use tracing::metadata::LevelFilter;

use crate::handler::utils::helpers::wasmaddr_to_id;

#[derive(clap::Args, Debug, Clone, Serialize)]
pub struct Verbosity {
    /// Increase verbosity, can be repeated up to 2 times
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl Verbosity {
    pub fn to_level_filter(&self) -> LevelFilter {
        match self.verbose {
            0 => LevelFilter::INFO,
            1 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    }
}

#[derive(Debug, Parser, Serialize)]
#[command(version, long_about = None)]
pub struct Cli {
    /// Increase log verbosity
    #[command(flatten)]
    pub verbose: Verbosity,

    /// Enable mock SGX mode for testing purposes.
    /// This flag disables the use of an Intel SGX processor and allows the system to run without remote attestations.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock_sgx: Option<bool>,

    /// Path to Quartz app directory
    /// Defaults to current working dir
    /// For quartz init, root serves as the parent directory of the directory in which the quartz app is generated
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_dir: Option<PathBuf>,

    /// Main command
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, Serialize, Clone)]
pub enum Command {
    /// Create an empty Quartz app from a template
    Init(InitArgs),

    /// Perform handshake
    Handshake(HandshakeArgs),

    /// Subcommands for handling the Quartz app contract
    Contract {
        #[command(subcommand)]
        contract_command: ContractCommand,
    },

    /// Subcommands for handling the Quartz app enclave
    Enclave {
        #[command(subcommand)]
        enclave_command: EnclaveCommand,
    },
}

#[derive(Debug, Clone, Subcommand, Serialize)]
pub enum ContractCommand {
    Build(ContractBuildArgs),
    Deploy(ContractDeployArgs),
    Query(ContractQueryArgs),
}

#[derive(Debug, Clone, Subcommand, Serialize)]
pub enum EnclaveCommand {
    /// Build the Quartz app's enclave
    Build(EnclaveBuildArgs),
    /// Run the Quartz app's enclave
    Start(EnclaveStartArgs),
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct InitArgs {
    /// The name of your Quartz app directory, defaults to quartz_app
    #[arg(default_value = "quartz_app")]
    pub name: PathBuf,
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct HandshakeArgs {
    /// Path to create & init a Quartz app, defaults to current path if unspecified
    #[arg(short, long, value_parser = wasmaddr_to_id)]
    pub contract: AccountId,

    /// Name or address of private key with which to sign
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_sender: Option<String>,

    /// The network chain ID
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,

    /// <host>:<port> to tendermint rpc interface for this chain
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_url: Option<String>,

    /// RPC interface for the Quartz enclave
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enclave_rpc_addr: Option<String>,

    /// Port enclave is listening on
    #[arg(long)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub enclave_rpc_port: Option<u16>,
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct ContractBuildArgs {
    #[arg(long)]
    pub manifest_path: PathBuf,
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct ContractDeployArgs {
    /// Json-formatted cosmwasm contract initialization message
    #[arg(long, default_value = "{}")]
    pub init_msg: String,

    /// <host>:<port> to tendermint rpc interface for this chain
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_url: Option<String>,

    /// Name or address of private key with which to sign
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_sender: Option<String>,

    /// The network chain ID
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,

    /// A human-readable name for this contract in lists
    #[arg(long, default_value = "Quartz App Contract")]
    pub label: String,

    /// Path to contract wasm binary for deployment
    #[arg(long)]
    pub wasm_bin_path: PathBuf,
}


#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct ContractQueryArgs {
    /// <host>:<port> to tendermint rpc interface for this chain
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_url: Option<String>,

    /// Path to create & init a Quartz app, defaults to current path if unspecified
    #[arg(short, long, value_parser = wasmaddr_to_id)]
    pub contract: AccountId,

    /// The network chain ID
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,
    
    /// Name or address of private key with which to query balance of
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_sender: Option<String>,

    /// Query message
    #[arg(long)]
    pub query_msg: String,
}


#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct EnclaveBuildArgs {
    /// Path to Cargo.toml file of the Quartz app's enclave package, defaults to './enclave/Cargo.toml' if unspecified
    #[arg(long, default_value = "./enclave/Cargo.toml")]
    pub manifest_path: PathBuf,
}

#[derive(Debug, Parser, Clone, Serialize, Deserialize)]
pub struct EnclaveStartArgs {
    /// The network chain ID
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<ChainId>,

    /// Fetch latest trusted hash and height from the chain instead of existing configuration
    #[arg(long)]
    pub use_latest_trusted: bool,
}

pub trait ToFigment {
    fn to_figment(&self) -> Figment;
}

impl ToFigment for Command {
    fn to_figment(&self) -> Figment {
        match self {
            Command::Init(args) => Figment::from(Serialized::defaults(args)),
            Command::Handshake(args) => Figment::from(Serialized::defaults(args)),
            Command::Contract { contract_command } => match contract_command {
                ContractCommand::Build(args) => Figment::from(Serialized::defaults(args)),
                ContractCommand::Deploy(args) => Figment::from(Serialized::defaults(args)),
                ContractCommand::Query(args) => Figment::from(Serialized::defaults(args)),
            },
            Command::Enclave { enclave_command } => match enclave_command {
                EnclaveCommand::Build(args) => Figment::from(Serialized::defaults(args)),
                EnclaveCommand::Start(args) => Figment::from(Serialized::defaults(args)),
            },
        }
    }
}
