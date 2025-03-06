use k256::ecdsa::{SigningKey, VerifyingKey};

use crate::key_manager::KeyManager;

#[derive(Clone, Default)]
pub struct DefaultKeyManager {
    sk: Option<SigningKey>,
}

#[async_trait::async_trait]
impl KeyManager for DefaultKeyManager {
    type PubKey = VerifyingKey;
    type PrivKey = SigningKey;

    async fn keygen(&mut self) {
        self.sk = Some(SigningKey::random(&mut rand::thread_rng()));
    }

    async fn pub_key(&self) -> Option<Self::PubKey> {
        self.sk.clone().map(|sk| VerifyingKey::from(&sk))
    }

    async fn priv_key(&self) -> Option<Self::PrivKey> {
        self.sk.clone()
    }
}
