#![doc = include_str!("../README.md")]
// #![forbid(unsafe_code)]
#![warn(
    clippy::checked_conversions,
    clippy::panic,
    clippy::panic_in_result_fn,
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_import_braces,
    unused_qualifications
)]

use cosmrs::AccountId;
use quartz_contract_core::state::Config;

use crate::{
    attestor::{Attestor, DefaultAttestor},
    key_manager::{default::DefaultKeyManager, shared::SharedKeyManager, KeyManager},
    store::{default::DefaultStore, Store},
};

pub mod attestor;
pub mod chain_client;
pub mod event;
pub mod grpc;
pub mod handler;
pub mod host;
pub mod key_manager;
pub mod proof_of_publication;
pub mod store;
pub mod types;

pub type DefaultSharedEnclave<C, K = DefaultKeyManager> =
    DefaultEnclave<C, DefaultAttestor, SharedKeyManager<K>, DefaultStore>;

#[async_trait::async_trait]
pub trait Enclave: Send + Sync + 'static {
    type Attestor: Attestor;
    type KeyManager: KeyManager;
    type Store: Store;

    async fn attestor(&self) -> Self::Attestor;
    async fn key_manager(&self) -> Self::KeyManager;
    async fn store(&self) -> &Self::Store;
}

#[derive(Clone, Debug)]
pub struct DefaultEnclave<C, A = DefaultAttestor, K = DefaultKeyManager, S = DefaultStore> {
    pub attestor: A,
    pub key_manager: K,
    pub store: S,
    pub ctx: C,
}

impl<C: Send + Sync + 'static> DefaultSharedEnclave<C> {
    pub fn shared(attestor: DefaultAttestor, config: Config, ctx: C) -> DefaultSharedEnclave<C> {
        DefaultSharedEnclave {
            attestor,
            key_manager: SharedKeyManager::wrapping(DefaultKeyManager::default()),
            store: DefaultStore::new(config),
            ctx,
        }
    }

    pub fn with_key_manager<K: KeyManager>(
        self,
        key_manager: K,
    ) -> DefaultEnclave<C, <Self as Enclave>::Attestor, K, <Self as Enclave>::Store> {
        DefaultEnclave {
            attestor: self.attestor,
            key_manager,
            store: self.store,
            ctx: self.ctx,
        }
    }
}

#[async_trait::async_trait]
impl<C, A, K, S> Enclave for DefaultEnclave<C, A, K, S>
where
    C: Send + Sync + 'static,
    A: Attestor + Clone,
    K: KeyManager + Clone,
    S: Store<Contract = AccountId> + Clone,
{
    type Attestor = A;
    type KeyManager = K;
    type Store = S;

    async fn attestor(&self) -> Self::Attestor {
        self.attestor.clone()
    }

    async fn key_manager(&self) -> Self::KeyManager {
        self.key_manager.clone()
    }

    async fn store(&self) -> &Self::Store {
        &self.store
    }
}
