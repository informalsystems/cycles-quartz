use std::{env, path::PathBuf};

use clap::{Parser, Subcommand};
use tracing::metadata::LevelFilter;

#[derive(clap::Args, Debug, Clone)]
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

#[derive(Debug, Parser)]
#[command(version, long_about = None)]
pub struct Cli {
    /// Increase log verbosity
    #[clap(flatten)]
    pub verbose: Verbosity,

    /// Enable mock SGX mode for testing purposes.
    /// This flag disables the use of an Intel SGX processor and allows the system to run without remote attestations.
    #[clap(long, default_value_t = default_mocksgx_flag())]
    pub mock_sgx: bool,

    /// Main command
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create an empty Quartz app from a template
    Init {
        /// path to create & init a quartz app, defaults to current path if unspecified
        #[clap(long)]
        path: Option<PathBuf>,
    },
    /// Subcommands for handling the quartz app contract
    Contract {
        #[command(subcommand)]
        contract_command: ContractCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ContractCommand {
    /// Build the Quartz app's smart contract
    Build {
        /// path to Cargo.toml file of the Quartz app's contract package, defaults to './contracts/Cargo.toml' if unspecified
        #[arg(long, default_value = "./contracts/Cargo.toml")]
        manifest_path: PathBuf,
    },
    /// Deploy the Quartz app's smart contract
    Deploy {
        #[clap(long)]
        path: Option<PathBuf>,
    },
}

fn default_mocksgx_flag() -> bool {
    let flag = env::var("MOCK_SGX").unwrap_or_else(|_| "0".to_string());

    !matches!(flag.as_str(), "0")
}
