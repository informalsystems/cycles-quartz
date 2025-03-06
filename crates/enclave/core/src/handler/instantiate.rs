use quartz_contract_core::msg::{execute::attested::Attested, instantiate::CoreInstantiate};
use quartz_proto::quartz::{
    InstantiateRequest as RawInstantiateRequest, InstantiateResponse as RawInstantiateResponse,
};
use tonic::Status;

use crate::{
    attestor::Attestor,
    handler::{Handler, A, RA},
    store::Store,
    types::InstantiateResponse,
    Enclave,
};

#[async_trait::async_trait]
impl<E: Enclave> Handler<E> for RawInstantiateRequest {
    type Error = Status;
    type Response = RawInstantiateResponse;

    async fn handle(self, ctx: &E) -> Result<Self::Response, Self::Error> {
        // create `CoreInstantiate` msg and attest to it
        let config = ctx
            .store()
            .await
            .get_config()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("config not found"))?;
        let msg = CoreInstantiate::new(config);
        let attestation = ctx
            .attestor()
            .await
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        let attested_msg = Attested::new(msg, attestation);

        // return response with attested `CoreInstantiate` msg
        let response: InstantiateResponse<A<E>, RA<E>> = InstantiateResponse::new(attested_msg);
        Ok(response.into())
    }
}
