use anyhow::anyhow;
use cosmrs::{crypto::secp256k1::SigningKey, AccountId};
use cw_client::{CwClient, GrpcClient};
use futures_util::StreamExt;
use quartz_tm_prover::{
    config::{Config as TmProverConfig, ProofOutput},
    prover::prove,
};
use reqwest::Url;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use tendermint::{block::Height, chain::Id as TmChainId, Hash};
use tendermint_rpc::{query::EventType, SubscriptionClient, WebSocketClient};

use crate::chain_client::ChainClient;

pub struct DefaultChainClient {
    chain_id: TmChainId,
    grpc_client: GrpcClient,
    node_url: Url,
    ws_url: Url,
    trusted_height: Height,
    trusted_hash: Hash,
}

impl DefaultChainClient {
    pub fn new(
        chain_id: TmChainId,
        signer: SigningKey,
        grpc_url: Url,
        node_url: Url,
        ws_url: Url,
        trusted_height: Height,
        trusted_hash: Hash,
    ) -> Self {
        DefaultChainClient {
            chain_id,
            grpc_client: GrpcClient::new(signer, grpc_url),
            node_url,
            ws_url,
            trusted_height,
            trusted_hash,
        }
    }
}

pub enum Query {
    Json(Value),
    String(String),
}

impl From<Value> for Query {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}

impl From<String> for Query {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

#[async_trait::async_trait]
impl ChainClient for DefaultChainClient {
    type Contract = AccountId;
    type Error = anyhow::Error;
    type Proof = ProofOutput;
    type Query = Query;
    type TxConfig = DefaultTxConfig;
    type TxOutput = String;

    async fn query_contract<R: DeserializeOwned + Default + Send>(
        &self,
        contract: &Self::Contract,
        query: impl Into<Self::Query> + Send,
    ) -> Result<R, Self::Error> {
        match query.into() {
            Query::Json(q) => self.grpc_client.query_smart(contract, q).await,
            Query::String(q) => self.grpc_client.query_raw(contract, q).await,
        }
    }

    async fn existence_proof(
        &self,
        contract: &Self::Contract,
        storage_key: &str,
    ) -> Result<Self::Proof, Self::Error> {
        let prover_config = TmProverConfig {
            primary: self.node_url.as_str().parse()?,
            witnesses: self.node_url.as_str().parse()?,
            trusted_height: self.trusted_height,
            trusted_hash: self.trusted_hash,
            verbose: "1".parse()?,
            contract_address: contract.clone(),
            storage_key: storage_key.to_string(),
            chain_id: self.chain_id.to_string(),
            ..Default::default()
        };

        let proof_output = tokio::task::spawn_blocking(move || {
            // Create a new runtime inside the blocking thread.
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                prove(prover_config)
                    .await
                    .map_err(|report| anyhow!("Tendermint prover failed. Report: {}", report))
            })
        })
        .await??; // Handle both JoinError and your custom error
        Ok(proof_output)
    }

    async fn send_tx<T: Serialize + Send + Sync>(
        &self,
        contract: &Self::Contract,
        tx: T,
        config: Self::TxConfig,
    ) -> Result<Self::TxOutput, Self::Error> {
        self.grpc_client
            .tx_execute(
                contract,
                &self.chain_id,
                config.gas,
                "",
                json!(tx),
                &config.amount,
            )
            .await
    }

    async fn wait_for_blocks(&self, _blocks: u8) -> Result<(), Self::Error> {
        let (client, driver) = WebSocketClient::new(self.ws_url.to_string().as_str()).await?;

        let driver_handle = tokio::spawn(async move { driver.run().await });

        // Subscription functionality
        let mut subs = client.subscribe(EventType::NewBlock.into()).await?;

        // Wait 2 NewBlock events
        let mut ev_count = 2_i32;

        while let Some(res) = subs.next().await {
            let _ev = res?;
            ev_count -= 1;
            if ev_count == 0 {
                break;
            }
        }

        // Signal to the driver to terminate.
        client.close()?;
        // Await the driver's termination to ensure proper connection closure.
        let _ = driver_handle.await?;

        Ok(())
    }
}

pub struct DefaultTxConfig {
    pub gas: u64,
    pub amount: String,
}
