use std::process::Command;

use color_eyre::{eyre::eyre, Help, Report, Result};
use cosmrs::{tendermint::chain::Id, AccountId};
use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::CwClient;

#[derive(Clone, Debug)]
pub enum CliClientType {
    Wasmd,
    Neutrond,
}

impl CliClientType {
    fn bin(&self) -> String {
        match self {
            CliClientType::Wasmd => "wasmd",
            CliClientType::Neutrond => "neutrond",
        }
        .to_string()
    }
}

#[derive(Clone, Debug)]
pub struct CliClient {
    kind: CliClientType,
    url: Url,
    gas_price: String,
}

impl CliClient {
    pub fn new(kind: CliClientType, url: Url, gas_price: String) -> Self {
        Self {
            kind,
            url,
            gas_price,
        }
    }

    pub fn wasmd(url: Url) -> Self {
        Self {
            kind: CliClientType::Wasmd,
            url,
            gas_price: "0.0025ucosm".to_string(),
        }
    }

    pub fn neutrond(url: Url) -> Self {
        Self {
            kind: CliClientType::Neutrond,
            url,
            gas_price: "0.0053untrn".to_string(),
        }
    }

    fn new_command(&self) -> Result<Command> {
        let bin = self.kind.bin();
        if !self.is_bin_available(&bin) {
            return Err(eyre!("Binary '{}' not found in PATH", bin)).suggestion(format!(
                "Have you installed {}? If so, check that it's in your PATH.",
                bin
            ));
        }

        Ok(Command::new(self.kind.bin()))
    }
    fn is_bin_available(&self, bin: &str) -> bool {
        Command::new("which")
            .arg(bin)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[async_trait::async_trait]
impl CwClient for CliClient {
    type Address = AccountId;
    type Query = serde_json::Value;
    type RawQuery = String;
    type ChainId = Id;
    type Error = Report;

    async fn query_smart<R: DeserializeOwned + Send>(
        &self,
        contract: &Self::Address,
        query: Self::Query,
    ) -> Result<R, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["query", "wasm"])
            .args(["contract-state", "smart", contract.as_ref()])
            .arg(query.to_string())
            .args(["--output", "json"]);

        let output = command.output()?;
        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        let query_result: R = serde_json::from_slice(&output.stdout)
            .map_err(|e| eyre!("Error deserializing: {}", e))?;
        Ok(query_result)
    }

    async fn query_raw<R: DeserializeOwned + Default>(
        &self,
        contract: &Self::Address,
        query: Self::RawQuery,
    ) -> Result<R, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["query", "wasm"])
            .args(["contract-state", "raw", contract.as_ref()])
            .arg(&query)
            .args(["--output", "json"]);

        let output = command.output()?;
        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        let query_result: R = serde_json::from_slice(&output.stdout).unwrap_or_default();
        Ok(query_result)
    }

    fn query_tx<R: DeserializeOwned + Default>(&self, txhash: &str) -> Result<R, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["query", "tx"])
            .arg(txhash)
            .args(["--output", "json"]);

        let output = command.output()?;
        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        let query_result: R = serde_json::from_slice(&output.stdout).unwrap_or_default();
        Ok(query_result)
    }

    async fn tx_execute<M: ToString + Send>(
        &self,
        contract: &Self::Address,
        chain_id: &Id,
        gas: u64,
        sender: &str,
        msg: M,
        fees: &str,
    ) -> Result<String, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["--chain-id", chain_id.as_ref()])
            .args(["tx", "wasm"])
            .args(["execute", contract.as_ref(), &msg.to_string()])
            .args(["--gas", &gas.to_string()])
            .args(["--fees", fees])
            .args(["--from", sender])
            .args(["--output", "json"])
            .arg("-y");

        let output = command.output()?;

        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        // TODO: find the rust type for the tx output and return that
        Ok((String::from_utf8(output.stdout)?).to_string())
    }

    fn deploy<M: ToString>(
        &self,
        chain_id: &Id,
        sender: &str,
        wasm_path: M,
    ) -> Result<String, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["tx", "wasm", "store", &wasm_path.to_string()])
            .args(["--from", sender])
            .args(["--chain-id", chain_id.as_ref()])
            .args(["--gas-prices", &self.gas_price])
            .args(["--gas", "auto"])
            .args(["--gas-adjustment", "1.3"])
            .args(["-o", "json"])
            .arg("-y");

        let output = command.output()?;

        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        // TODO: find the rust type for the tx output and return that
        Ok((String::from_utf8(output.stdout)?).to_string())
    }

    fn init<M: ToString>(
        &self,
        chain_id: &Id,
        sender: &str,
        code_id: u64,
        init_msg: M,
        label: &str,
    ) -> Result<String, Self::Error> {
        let mut command = self.new_command()?;
        let command = command
            .args(["--node", self.url.as_str()])
            .args(["tx", "wasm", "instantiate"])
            .args([&code_id.to_string(), &init_msg.to_string()])
            .args(["--label", label])
            .args(["--from", sender])
            .arg("--no-admin")
            .args(["--chain-id", chain_id.as_ref()])
            .args(["--gas-prices", &self.gas_price])
            .args(["--gas", "auto"])
            .args(["--gas-adjustment", "1.3"])
            .args(["-o", "json"])
            .arg("-y");

        let output = command.output()?;

        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        // TODO: find the rust type for the tx output and return that
        Ok((String::from_utf8(output.stdout)?).to_string())
    }

    fn trusted_height_hash(&self) -> Result<(u64, String), Self::Error> {
        let mut command = self.new_command()?;
        let command = command.args(["--node", self.url.as_str()]).arg("status");

        let output = command.output()?;

        if !output.status.success() {
            return Err(eyre!("{:?}", output));
        }

        let query_result: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_default();

        let sync_info = match self.kind {
            CliClientType::Wasmd => "SyncInfo",
            CliClientType::Neutrond => "sync_info",
        };
        let trusted_height = query_result[sync_info]["latest_block_height"]
            .as_str()
            .ok_or(eyre!("Could not query height"))?;

        let trusted_height = trusted_height.parse::<u64>()?;

        let trusted_hash = query_result[sync_info]["latest_block_hash"]
            .as_str()
            .ok_or(eyre!("Could not query height"))?
            .to_string();

        Ok((trusted_height, trusted_hash))
    }
}
