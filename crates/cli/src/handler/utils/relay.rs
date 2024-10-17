use color_eyre::{eyre::eyre, Result};
use cosmrs::AccountId;
use quartz_common::proto::{
    core_client::CoreClient, InstantiateRequest, SessionCreateRequest, SessionSetPubKeyRequest,
};
use quartz_tm_prover::config::ProofOutput;
use serde_json::{json, Value as JsonValue};

#[derive(Debug)]
pub enum RelayMessage {
    Instantiate { init_msg: JsonValue },
    SessionCreate { contract: AccountId },
    SessionSetPubKey { proof: ProofOutput },
}

impl RelayMessage {
    pub async fn run_relay(self, enclave_rpc: String) -> Result<JsonValue> {
        // Query the gRPC quartz enclave service
        let mut qc_client = CoreClient::connect(enclave_rpc).await.map_err(|e| {
            eyre!(
                "Failed to connect to the gRPC quartz enclave service: {}",
                e
            )
        })?;

        let attested_msg = match self {
            RelayMessage::Instantiate { mut init_msg } => qc_client
                .instantiate(tonic::Request::new(InstantiateRequest {}))
                .await
                .map_err(|e| {
                    eyre!(
                        "Failed to instantiate via gRPC quartz enclave service: {}",
                        e
                    )
                })
                .map(|res| serde_json::from_str::<JsonValue>(&res.into_inner().message))?
                .map(|msg| {
                    init_msg["quartz"] = msg;
                    init_msg.to_string()
                })?,
            RelayMessage::SessionCreate { contract } => qc_client
                .session_create(tonic::Request::new(SessionCreateRequest {
                    message: serde_json::to_string(&contract)?,
                }))
                .await
                .map_err(|e| {
                    eyre!(
                        "Failed to create session via gRPC quartz enclave service: {}",
                        e
                    )
                })
                .map(|res| serde_json::from_str::<JsonValue>(&res.into_inner().message))?
                .map(|msg| json!({ "quartz": {"session_create": msg}}).to_string())?,
            RelayMessage::SessionSetPubKey { proof } => qc_client
                .session_set_pub_key(SessionSetPubKeyRequest {
                    message: serde_json::to_string(&proof)?,
                })
                .await
                .map_err(|e| {
                    eyre!(
                        "Failed to set public key via gRPC quartz enclave service: {}",
                        e
                    )
                })
                .map(|res| serde_json::from_str::<JsonValue>(&res.into_inner().message))?
                .map(|msg| json!({ "quartz":  {"session_set_pub_key": msg}}).to_string())?,
        };
        serde_json::from_str(&attested_msg).map_err(Into::into)
    }
}
