use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use cosmrs::AccountId;
use futures_util::StreamExt;
use k256::ecdsa::SigningKey;
use quartz_contract_core::{
    msg::{
        execute::{
            attested::Attested, session_create::SessionCreate,
            session_set_pub_key::SessionSetPubKey,
        },
        instantiate::CoreInstantiate,
    },
    state::{Config, LightClientOpts, Nonce, Session, SESSION_KEY},
};
use quartz_cw_proof::proof::{
    cw::{CwProof, RawCwProof},
    key::CwAbciKey,
    Proof,
};
use quartz_proto::quartz::{
    core_server::{Core, CoreServer},
    InstantiateRequest as RawInstantiateRequest, InstantiateResponse as RawInstantiateResponse,
    SessionCreateRequest as RawSessionCreateRequest,
    SessionCreateResponse as RawSessionCreateResponse,
    SessionSetPubKeyRequest as RawSessionSetPubKeyRequest,
    SessionSetPubKeyResponse as RawSessionSetPubKeyResponse,
};
use quartz_tm_stateless_verifier::make_provider;
use rand::Rng;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tendermint::{block::Height, Hash};
use tendermint_light_client::{
    light_client::Options,
    types::{LightBlock, TrustThreshold},
};
use tendermint_rpc::{
    event::Event,
    query::{EventType, Query},
    SubscriptionClient, WebSocketClient,
};
use tonic::{
    body::BoxBody,
    codegen::http,
    server::NamedService,
    transport::{server::Router, Server},
    Request, Response, Result as TonicResult, Status,
};
use tower::Service;

use crate::{
    attestor::Attestor,
    error::QuartzError,
    types::{InstantiateResponse, SessionCreateResponse, SessionSetPubKeyResponse},
};

/// Trait for Quartz enclaves to process on-chain events.
///
/// Implementors of this trait should define how to process incoming WebSocket events,
/// using the provided `event` and `ws_config` parameters.
///
/// # Arguments
///
/// * `event` - The WebSocket event received from the Tendermint RPC server.
/// * `ws_config` - Configuration values used for handling the WebSocket events,
///   such as node URL, a signer for transactions, and trusted block information.
///
/// # Returns
///
/// An `anyhow::Result<()>` indicating success or failure in handling the event.
#[tonic::async_trait]
pub trait WebSocketHandler: Send + Sync + 'static {
    async fn handle(&self, event: Event, ws_config: WsListenerConfig) -> anyhow::Result<()>; // TODO: replace anyhow
}

#[derive(Debug, Clone)]
pub struct WsListenerConfig {
    pub node_url: Url,
    pub ws_url: Url,
    pub grpc_url: Url,
    pub chain_id: String,
    pub tx_sender: String,
    pub trusted_hash: Hash,
    pub trusted_height: Height,
    pub admin_sk: String,
}

/// A trait for wrapping a tonic service with the gRPC server handler
pub trait IntoServer {
    type Server;

    fn into_server(self) -> Self::Server;
}

pub struct QuartzServer {
    pub router: Router,
    ws_handlers: Vec<Box<dyn WebSocketHandler>>,
    pub ws_config: WsListenerConfig,
}

impl QuartzServer {
    pub fn new<A>(
        config: Config,
        sk: Arc<Mutex<Option<SigningKey>>>,
        contract: Arc<Mutex<Option<AccountId>>>,
        attestor: A,
        ws_config: WsListenerConfig,
    ) -> Self
    where
        A: Attestor + Clone,
    {
        let core_service = CoreServer::new(CoreService::new(
            config,
            contract.clone(),
            sk.clone(),
            attestor.clone(),
        ));

        Self {
            router: Server::builder().add_service(core_service),
            ws_handlers: Vec::new(),
            ws_config,
        }
    }

    pub fn add_service<S>(mut self, service: S) -> Self
    where
        S: IntoServer + WebSocketHandler + Clone,
        S::Server: Service<
                http::request::Request<BoxBody>,
                Response = http::response::Response<BoxBody>,
                Error = Infallible,
            > + NamedService
            + Clone
            + Send
            + 'static,
        <S::Server as Service<http::request::Request<BoxBody>>>::Future: Send + 'static,
    {
        self.ws_handlers.push(Box::new(service.clone()));

        let tonic_server = service.into_server();
        self.router = self.router.add_service(tonic_server);

        self
    }

    pub async fn serve(self, addr: SocketAddr) -> Result<(), QuartzError> {
        // Launch all WebSocket handlers as separate Tokio tasks
        tokio::spawn(async move {
            if let Err(e) = Self::websocket_events_listener(&self.ws_handlers, self.ws_config).await
            {
                eprintln!("Error in WebSocket event handler: {:?}", e);
            }
        });

        Ok(self.router.serve(addr).await?)
    }

    async fn websocket_events_listener(
        ws_handlers: &Vec<Box<dyn WebSocketHandler>>,
        ws_config: WsListenerConfig,
    ) -> Result<(), QuartzError> {
        let (client, driver) = WebSocketClient::new(ws_config.ws_url.as_str())
            .await
            .unwrap();
        let driver_handle = tokio::spawn(async move { driver.run().await });
        let mut subs = client.subscribe(Query::from(EventType::Tx)).await.unwrap();

        while let Some(Ok(event)) = subs.next().await {
            for handler in ws_handlers {
                if let Err(e) = handler.handle(event.clone(), ws_config.clone()).await {
                    eprintln!("Error in event handler: {:?}", e);
                }
            }
        }

        // Close connection
        client.close()?;
        let _ = driver_handle.await;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CoreService<A> {
    config: Config,
    nonce: Arc<Mutex<Nonce>>,
    contract: Arc<Mutex<Option<AccountId>>>,
    sk: Arc<Mutex<Option<SigningKey>>>,
    attestor: A,
}

impl<A> CoreService<A>
where
    A: Attestor,
{
    pub fn new(
        config: Config,
        contract: Arc<Mutex<Option<AccountId>>>,
        sk: Arc<Mutex<Option<SigningKey>>>,
        attestor: A,
    ) -> Self {
        Self {
            config,
            nonce: Arc::new(Mutex::new([0u8; 32])),
            contract,
            sk,
            attestor,
        }
    }
}

#[tonic::async_trait]
impl<A> Core for CoreService<A>
where
    A: Attestor + Send + Sync + 'static,
{
    async fn instantiate(
        &self,
        _request: Request<RawInstantiateRequest>,
    ) -> TonicResult<Response<RawInstantiateResponse>> {
        let msg = CoreInstantiate::new(self.config.clone());

        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        let attested_msg = Attested::new(msg, attestation);

        let response: InstantiateResponse<A::Attestation, A::RawAttestation> =
            InstantiateResponse::new(attested_msg);
        Ok(Response::new(response.into()))
    }

    async fn session_create(
        &self,
        request: Request<RawSessionCreateRequest>,
    ) -> TonicResult<Response<RawSessionCreateResponse>> {
        // FIXME(hu55a1n1) - disallow calling more than once
        let deployed_contract: AccountId = serde_json::from_str(&request.into_inner().message)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let mut contract = self.contract.lock().unwrap();
        *contract = Some(deployed_contract.clone());

        let mut nonce = self.nonce.lock().unwrap();
        *nonce = rand::thread_rng().gen::<Nonce>();

        let msg = SessionCreate::new(*nonce, deployed_contract.to_string());

        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        let attested_msg = Attested::new(msg, attestation);

        let response: SessionCreateResponse<A::Attestation, A::RawAttestation> =
            SessionCreateResponse::new(attested_msg);
        Ok(Response::new(response.into()))
    }

    async fn session_set_pub_key(
        &self,
        request: Request<RawSessionSetPubKeyRequest>,
    ) -> TonicResult<Response<RawSessionSetPubKeyResponse>> {
        // FIXME(hu55a1n1) - disallow calling more than once
        let proof: ProofOfPublication<Option<()>> =
            serde_json::from_str(&request.into_inner().message)
                .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let contract = self.contract.lock().unwrap().clone();

        let (value, _msg) = proof
            .verify(
                self.config.light_client_opts(),
                contract.expect("contract not set"),
                SESSION_KEY.to_string(),
                None,
            )
            .map_err(Status::failed_precondition)?;

        let session: Session = serde_json::from_slice(&value).unwrap();
        let nonce = self.nonce.lock().unwrap();

        if session.nonce() != *nonce {
            return Err(Status::unauthenticated("nonce mismatch"));
        }

        let sk = SigningKey::random(&mut rand::thread_rng());
        *self.sk.lock().unwrap() = Some(sk.clone());
        let pk = sk.verifying_key();

        let msg = SessionSetPubKey::new(*nonce, *pk);

        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        let attested_msg = Attested::new(msg, attestation);

        let response: SessionSetPubKeyResponse<A::Attestation, A::RawAttestation> =
            SessionSetPubKeyResponse::new(attested_msg);
        Ok(Response::new(response.into()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofOfPublication<M> {
    light_client_proof: Vec<LightBlock>,
    merkle_proof: RawCwProof,
    msg: M,
}

impl<M> ProofOfPublication<M> {
    pub fn verify(
        self,
        light_client_opts: &LightClientOpts,
        contract_address: AccountId,
        storage_key: String,
        storage_namespace: Option<String>,
    ) -> Result<(Vec<u8>, M), String> {
        let config_trust_threshold = light_client_opts.trust_threshold();
        let trust_threshold =
            TrustThreshold::new(config_trust_threshold.0, config_trust_threshold.1).unwrap();

        let config_trusting_period = light_client_opts.trusting_period();
        let trusting_period = Duration::from_secs(config_trusting_period);

        let config_clock_drift = light_client_opts.max_clock_drift();
        let clock_drift = Duration::from_secs(config_clock_drift);
        let options = Options {
            trust_threshold,
            trusting_period,
            clock_drift,
        };

        let target_height = self.light_client_proof.last().unwrap().height();

        let primary_block = make_provider(
            light_client_opts.chain_id(),
            light_client_opts.trusted_height().try_into().unwrap(),
            light_client_opts
                .trusted_hash()
                .to_vec()
                .try_into()
                .unwrap(),
            self.light_client_proof,
            options,
        )
        .and_then(|mut primary| primary.verify_to_height(target_height))
        .map_err(|e| e.to_string())?;

        let key = CwAbciKey::new(contract_address, storage_key, storage_namespace);
        if key.into_vec() != self.merkle_proof.key() {
            return Err("Merkle proof key mismatch".to_string());
        }

        let proof = CwProof::from(self.merkle_proof);
        proof
            .verify(
                primary_block
                    .signed_header
                    .header
                    .app_hash
                    .as_bytes()
                    .to_vec(),
            )
            .map_err(|e| e.to_string())?;

        Ok((proof.value, self.msg))
    }
}
