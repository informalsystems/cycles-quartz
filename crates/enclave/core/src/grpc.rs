use cosmrs::AccountId;
use k256::ecdsa::VerifyingKey;
use quartz_proto::quartz::{
    core_server::Core, InstantiateRequest, InstantiateResponse, SessionCreateRequest,
    SessionCreateResponse, SessionSetPubKeyRequest, SessionSetPubKeyResponse,
};
use tonic::{Request, Response, Status};

use crate::{
    attestor::Attestor,
    handler::Handler,
    key_manager::KeyManager,
    kv_store::{ConfigKey, ContractKey, NonceKey, TypedStore},
    DefaultEnclave,
};

#[async_trait::async_trait]
impl<T, C> Handler<C> for Request<T>
where
    T: Handler<C>,
    C: Send + Sync,
{
    type Error = T::Error;
    type Response = Response<T::Response>;

    async fn handle(self, ctx: &C) -> Result<Self::Response, Self::Error> {
        let request = self.into_inner();
        let response = request.handle(ctx).await?;
        Ok(Response::new(response))
    }
}

#[async_trait::async_trait]
impl<A, K, S> Core for DefaultEnclave<A, K, S>
where
    A: Attestor + Clone,
    K: KeyManager<PubKey = VerifyingKey> + Clone,
    S: TypedStore<ContractKey<AccountId>> + TypedStore<NonceKey> + TypedStore<ConfigKey> + Clone,
{
    async fn instantiate(
        &self,
        request: Request<InstantiateRequest>,
    ) -> Result<Response<InstantiateResponse>, Status> {
        request.handle(self).await
    }

    async fn session_create(
        &self,
        request: Request<SessionCreateRequest>,
    ) -> Result<Response<SessionCreateResponse>, Status> {
        request.handle(self).await
    }

    async fn session_set_pub_key(
        &self,
        request: Request<SessionSetPubKeyRequest>,
    ) -> Result<Response<SessionSetPubKeyResponse>, Status> {
        request.handle(self).await
    }
}
