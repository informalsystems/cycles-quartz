use std::sync::Arc;

use tokio::sync::RwLock;

use crate::key_manager::KeyManager;

#[derive(Clone, Debug)]
pub struct SharedKeyManager<K> {
    inner: Arc<RwLock<K>>,
}

impl<K> SharedKeyManager<K> {
    pub fn wrapping(key_manager: K) -> Self {
        Self {
            inner: Arc::new(RwLock::new(key_manager)),
        }
    }
}

#[async_trait::async_trait]
impl<K: KeyManager> KeyManager for SharedKeyManager<K> {
    type PubKey = K::PubKey;
    type PrivKey = K::PrivKey;

    async fn keygen(&mut self) {
        self.inner.write().await.keygen().await
    }

    async fn pub_key(&self) -> Option<Self::PubKey> {
        self.inner.read().await.pub_key().await
    }

    async fn priv_key(&self) -> Option<Self::PrivKey> {
        self.inner.read().await.priv_key().await
    }
}
