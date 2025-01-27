use cosmrs::AccountId;
use k256::ecdsa::VerifyingKey;
use quartz_contract_core::{
    msg::execute::{attested::Attested, session_set_pub_key::SessionSetPubKey},
    state::{Session, SESSION_KEY},
};
use quartz_proto::quartz::{
    SessionSetPubKeyRequest as RawSessionSetPubKeyRequest,
    SessionSetPubKeyResponse as RawSessionSetPubKeyResponse,
};
use tonic::Status;

use crate::{
    attestor::Attestor,
    handler::{Handler, A, RA},
    key_manager::KeyManager,
    proof_of_publication::ProofOfPublication,
    store::Store,
    types::SessionSetPubKeyResponse,
    Enclave,
};

#[async_trait::async_trait]
impl<E> Handler<E> for RawSessionSetPubKeyRequest
where
    E: Enclave,
    E::KeyManager: KeyManager<PubKey = VerifyingKey>,
    E::Store: Store<Contract = AccountId>,
{
    type Error = Status;
    type Response = RawSessionSetPubKeyResponse;

    async fn handle(self, ctx: &E) -> Result<Self::Response, Self::Error> {
        // verify proof of publication
        let proof: ProofOfPublication<Option<()>> = serde_json::from_str(&self.message)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let contract = ctx
            .store()
            .await
            .get_contract()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("contract not found"))?;
        let config = ctx
            .store()
            .await
            .get_config()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("config not found"))?;
        let (value, _msg) = proof
            .verify(
                config.light_client_opts(),
                contract,
                SESSION_KEY.to_string(),
                None,
            )
            .map_err(Status::failed_precondition)?;

        // make sure session nonce matches what we have locally
        let session: Session = serde_json::from_slice(&value).unwrap();
        let nonce = ctx
            .store()
            .await
            .get_nonce()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("nonce not found"))?;
        if session.nonce() != nonce {
            return Err(Status::unauthenticated("nonce mismatch"));
        }

        // generate enclave key
        ctx.key_manager().await.keygen().await;
        let pk = ctx
            .key_manager()
            .await
            .pub_key()
            .await
            .ok_or_else(|| Status::internal("failed to get public key"))?;

        // create `SessionSetPubKey` msg and attest to it
        let msg = SessionSetPubKey::new(nonce, pk);
        let attestation = ctx
            .attestor()
            .await
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        let attested_msg = Attested::new(msg, attestation);

        // return response with attested `SessionCreate` msg
        let response: SessionSetPubKeyResponse<A<E>, RA<E>> =
            SessionSetPubKeyResponse::new(attested_msg);
        Ok(response.into())
    }
}
