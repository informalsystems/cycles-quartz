use std::time::Duration;

use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use cosmrs::{AccountId, ErrorReport};
use cw_client::{CliClient, CwClient};
use regex::Regex;
use reqwest::Url;
use subtle_encoding::bech32::decode as bech32_decode;
use tendermint::{block::Height, Hash};
use tendermint_rpc::{
    endpoint::tx::Response as TmTxResponse, error::ErrorDetail, Client, HttpClient,
};
use tokio::fs::{self};
use tracing::debug;

use crate::config::Config;

pub fn wasmaddr_to_id(address_str: &str) -> Result<AccountId> {
    let _ = bech32_decode(address_str).map_err(|e| eyre!(e))?;
    address_str.parse().map_err(|e: ErrorReport| eyre!(e))
}

// Note: time until tx commit is empiraclly 800ms on DO wasmd chain.
pub async fn block_tx_commit(client: &HttpClient, tx: Hash) -> Result<TmTxResponse> {
    let re = Regex::new(r"tx \([A-F0-9]{64}\) not found")?;

    tokio::time::sleep(Duration::from_millis(400)).await;
    loop {
        match client.tx(tx, false).await {
            Ok(resp) => {
                return Ok(resp);
            }
            Err(e) => {
                // If error, make sure it is only because of a not yet committed tx
                match e.0 {
                    ErrorDetail::Response(subdetail) => {
                        if !re.is_match(subdetail.source.data().unwrap_or_default()) {
                            return Err(eyre!(
                                "Error querying for tx: {}",
                                ErrorDetail::Response(subdetail)
                            ));
                        } else {
                            debug!("🔗 Waiting for tx commit... (+400ms)");
                            tokio::time::sleep(Duration::from_millis(400)).await;
                            continue;
                        }
                    }
                    _ => {
                        return Err(eyre!("Error querying for tx: {}", e.0));
                    }
                }
            }
        }
    }
}

// Queries the chain for the latested height and hash
pub fn query_latest_height_hash(node_url: Url) -> Result<(Height, Hash)> {
    let cw_client = CliClient::neutrond(node_url);

    let (trusted_height, trusted_hash) = cw_client
        .trusted_height_hash()
        .map_err(|e| eyre!(e))
        .wrap_err("Could not query chain with cw client")?;

    Ok((
        trusted_height.try_into()?,
        trusted_hash.parse().expect("invalid hash from wasmd"),
    ))
}

pub async fn write_cache_hash_height(
    trusted_height: Height,
    trusted_hash: Hash,
    config: &Config,
) -> Result<()> {
    let height_path = config.cache_dir()?.join("trusted.height");
    fs::write(height_path.as_path(), trusted_height.to_string()).await?;
    let hash_path = config.cache_dir()?.join("trusted.hash");
    fs::write(hash_path.as_path(), trusted_hash.to_string()).await?;

    Ok(())
}

pub async fn read_cached_hash_height(config: &Config) -> Result<(Height, Hash)> {
    let height_path = config.cache_dir()?.join("trusted.height");
    let hash_path = config.cache_dir()?.join("trusted.hash");

    if !height_path.exists() {
        return Err(eyre!(
            "Could not read trusted height from cache: {}",
            height_path.display().to_string()
        ));
    }
    if !hash_path.exists() {
        return Err(eyre!(
            "Could not read trusted hash from cache: {}",
            hash_path.display().to_string()
        ));
    }

    let trusted_height: Height = fs::read_to_string(height_path.as_path()).await?.parse()?;
    let trusted_hash: Hash = fs::read_to_string(hash_path.as_path()).await?.parse()?;

    Ok((trusted_height, trusted_hash))
}
