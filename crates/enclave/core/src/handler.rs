use cosmrs::AccountId;
use k256::ecdsa::VerifyingKey;
use quartz_proto::quartz::{
    InstantiateRequest, InstantiateResponse, SessionCreateRequest, SessionCreateResponse,
    SessionSetPubKeyRequest, SessionSetPubKeyResponse,
};
use tonic::Status;

use crate::{attestor::Attestor, key_manager::KeyManager, store::Store, Enclave};

pub type A<E> = <<E as Enclave>::Attestor as Attestor>::Attestation;
pub type RA<E> = <<E as Enclave>::Attestor as Attestor>::RawAttestation;

pub mod instantiate;
pub mod session_create;
pub mod session_set_pubkey;

#[async_trait::async_trait]
pub trait Handler<Context>: Send + Sync + 'static {
    type Error;
    type Response;

    async fn handle(self, ctx: &Context) -> Result<Self::Response, Self::Error>;
}

#[derive(Clone, Debug)]
pub enum CoreEnclaveRequest {
    Instantiate(InstantiateRequest),
    SessionCreate(SessionCreateRequest),
    SessionSetPubKey(SessionSetPubKeyRequest),
}

#[derive(Clone, Debug)]
pub enum CoreEnclaveResponse {
    Instantiate(InstantiateResponse),
    SessionCreate(SessionCreateResponse),
    SessionSetPubKey(SessionSetPubKeyResponse),
}

#[async_trait::async_trait]
impl<E: Enclave> Handler<E> for CoreEnclaveRequest
where
    E: Enclave,
    E::KeyManager: KeyManager<PubKey = VerifyingKey>,
    E::Store: Store<Contract = AccountId>,
{
    type Error = Status;
    type Response = CoreEnclaveResponse;

    async fn handle(self, ctx: &E) -> Result<Self::Response, Self::Error> {
        match self {
            CoreEnclaveRequest::Instantiate(req) => {
                req.handle(ctx).await.map(CoreEnclaveResponse::Instantiate)
            }
            CoreEnclaveRequest::SessionCreate(req) => req
                .handle(ctx)
                .await
                .map(CoreEnclaveResponse::SessionCreate),
            CoreEnclaveRequest::SessionSetPubKey(req) => req
                .handle(ctx)
                .await
                .map(CoreEnclaveResponse::SessionSetPubKey),
        }
    }
}

pub fn ensure_seq_num_consistency(
    seq_num_in_store: u64,
    seq_num_on_chain: u64,
    pending_sequenced_requests: usize,
) -> Result<(), Status> {
    if seq_num_on_chain < seq_num_in_store {
        return Err(Status::failed_precondition("replay attempted"));
    }

    // make sure number of pending requests are equal to the diff b/w on-chain v/s in-mem seq num
    let seq_num_diff = seq_num_on_chain - seq_num_in_store;
    if seq_num_diff != pending_sequenced_requests as u64 {
        return Err(Status::failed_precondition(format!(
            "seq_num_diff mismatch: num({seq_num_diff}) v/s diff({pending_sequenced_requests})"
        )));
    }

    Ok(())
}
