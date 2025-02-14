use std::sync::{Arc, Mutex};

use cosmrs::AccountId;
use cosmwasm_std::HexBinary;
use ecies::{decrypt, encrypt};
use k256::ecdsa::SigningKey;
use ping_pong_contract::{
    msg::execute::{Ping, Pong},
    state::PINGS_KEY,
};
use quartz_common::{
    contract::{msg::execute::attested::RawAttested, state::Config},
    enclave::{
        attestor::Attestor,
        server::{IntoServer, ProofOfPublication, WsListenerConfig},
    },
};
use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Result as TonicResult, Status};

use crate::proto::{
    ping_pong_server::{PingPong, PingPongServer},
    PingRequest, PongResponse,
};

impl<A: Attestor> IntoServer for PingPongService<A> {
    type Server = PingPongServer<PingPongService<A>>;

    fn into_server(self) -> Self::Server {
        PingPongServer::new(self)
    }
}

pub type RawCipherText = HexBinary;

#[derive(Clone, Debug)]
pub enum PingOpEvent {
    Ping { contract: AccountId, ping: Ping },
}

#[derive(Clone, Debug)]
pub struct PongOp<A: Attestor> {
    pub client: PingPongService<A>,
    pub event: PingOpEvent,
    pub config: WsListenerConfig,
}

#[derive(Clone, Debug)]
pub struct PingPongService<A: Attestor> {
    config: Config,
    contract: Arc<Mutex<Option<AccountId>>>,
    sk: Arc<Mutex<Option<SigningKey>>>,
    attestor: A,
    pub queue_producer: Sender<PongOp<A>>,
}

impl<A> PingPongService<A>
where
    A: Attestor,
{
    pub fn new(
        config: Config,
        contract: Arc<Mutex<Option<AccountId>>>,
        sk: Arc<Mutex<Option<SigningKey>>>,
        attestor: A,
        queue_producer: Sender<PongOp<A>>,
    ) -> Self {
        Self {
            config,
            contract,
            sk,
            attestor,
            queue_producer,
        }
    }
}

#[tonic::async_trait]
impl<A> PingPong for PingPongService<A>
where
    A: Attestor + Send + Sync + 'static,
{
    async fn run(&self, request: Request<PingRequest>) -> TonicResult<Response<PongResponse>> {
        // Serialize request into ProofOfPublication struct containing the `Ping` data and its storage proof
        let message: ProofOfPublication<Ping> = {
            let message = request.into_inner().message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        let contract = self
            .contract
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| Status::failed_precondition("Contract address is not set"))?;
        let (proof_value, ping) = message
            .verify(
                self.config.light_client_opts(),
                contract,
                PINGS_KEY.to_string(),
                None,
            )
            .map_err(Status::failed_precondition)?;

        // Verify that the ping.message contents match the value of the storage proof
        let proof_value_matches_msg =
            serde_json::to_string(&ping.message).is_ok_and(|s| s.as_bytes() == proof_value);
        if !proof_value_matches_msg {
            return Err(Status::failed_precondition("proof verification"));
        }

        // Perform enclave logic
        // Decrypt the ciphertext using enclave private key
        let decrypted_message: String = {
            let sk_lock = self
                .sk
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?;

            let sk = sk_lock
                .as_ref()
                .ok_or(Status::internal("SigningKey unavailable"))?;

            let msg_bytes = decrypt(&sk.to_bytes(), &ping.message)
                .map_err(|_| Status::invalid_argument("decryption failed"))?;

            String::from_utf8(msg_bytes)
                .map_err(|_| Status::invalid_argument("Byte conversion to string failed"))?
        };

        println!("\nDecryption Result: {}\n", decrypted_message);
        let response = format!("Enclave responded to {}", decrypted_message);

        // Encrypt enclave response to the user's provided pubkey
        let encrypted_response: Vec<u8> = {
            encrypt(&ping.pubkey, response.as_bytes())
                .map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Prepare message to chain
        let msg = Pong {
            pubkey: ping.pubkey,
            response: HexBinary::from(encrypted_response),
        };

        // Attest to message
        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;

        let attested_msg = RawAttested {
            msg,
            attestation: A::RawAttestation::from(attestation),
        };
        let message =
            serde_json::to_string(&attested_msg).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PongResponse { message }))
    }
}
