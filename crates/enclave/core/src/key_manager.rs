use serde::Serialize;

pub mod default;
pub mod shared;

#[async_trait::async_trait]
pub trait KeyManager: Send + Sync + 'static {
    type PubKey: Serialize;
    type PrivKey;

    async fn keygen(&mut self);
    async fn pub_key(&self) -> Option<Self::PubKey>;
    async fn priv_key(&self) -> Option<Self::PrivKey>;
}
