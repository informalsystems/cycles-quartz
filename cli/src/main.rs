#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_import_braces,
    unused_qualifications
)]

pub mod cli;
pub mod config;
pub mod error;
pub mod handler;
pub mod request;
pub mod response;

use clap::Parser;
use cli::ToFigment;
use color_eyre::eyre::Result;
use config::Config;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

use crate::{cli::Cli, handler::Handler, request::Request};

const BANNER: &str = r"
 ________       ___  ___      ________      ________     __________    ________     
|\   __  \     |\  \|\  \    |\   __  \    |\   __  \   |\___   ___\  |\_____  \    
\ \  \|\  \    \ \  \\\  \   \ \  \|\  \   \ \  \|\  \  \|___ \  \_|   \|___/  /|   
 \ \  \\\  \    \ \  \\\  \   \ \   __  \   \ \   _  _\      \ \  \        /  / /   
  \ \  \\\  \    \ \  \\\  \   \ \  \ \  \   \ \  \\  \       \ \  \      /  /_/__  
   \ \_____  \    \ \_______\   \ \__\ \__\   \ \__\\ _\       \ \__\    |\________\
    \|___| \__\    \|_______|    \|__|\|__|    \|__|\|__|       \|__|     \|_______|
          \|__|                                                                     
                                                                                    
";

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    println!("{BANNER}");

    let args: Cli = Cli::parse();

    let config: Config = Figment::new()
        .merge(Toml::file("quartz.toml"))
        .merge(Env::prefixed("QUARTZ_"))
        .merge(Serialized::defaults(&args))
        .merge(args.command.to_figment())
        .extract()?;

    let env_filter = EnvFilter::builder()
        .with_default_directive(args.verbose.to_level_filter().into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_target(false)
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter)
        .finish()
        .init();

    // The idea is to parse the input args and convert them into `Requests` which are
    // correct-by-construction types that this tool can handle. All validation should happen during
    // this conversion.
    let request = Request::try_from(args.command)?;

    // Each `Request` defines an associated `Handler` (i.e. logic) and `Response`. All handlers are
    // free to log to the terminal and these logs are sent to `stderr`.
    let response = request.handle(config).await?;

    // `Handlers` must use `Responses` to output to `stdout`.
    println!(
        "{}",
        serde_json::to_string(&response).expect("infallible serializer")
    );

    Ok(())
}
