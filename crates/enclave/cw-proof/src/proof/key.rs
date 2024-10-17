use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::marker::PhantomData;

use cosmrs::AccountId;

use crate::proof::prefix::ConstPrefix;

const CONTRACT_STORE_PREFIX: u8 = 0x03;

pub trait IntoKeys {
    fn into_keys(self) -> Vec<Vec<u8>>;
}

impl<K> IntoKeys for K
where
    K: Into<Vec<u8>>,
{
    fn into_keys(self) -> Vec<Vec<u8>> {
        vec![self.into()]
    }
}

#[derive(Clone, Debug)]
pub struct PrefixedKey<P, K> {
    key: K,
    prefix: PhantomData<P>,
}

impl<P, K> PrefixedKey<P, K> {
    pub fn new(key: K) -> Self {
        Self {
            key,
            prefix: PhantomData,
        }
    }
}

impl<P, K> IntoKeys for PrefixedKey<P, K>
where
    K: Into<Vec<u8>>,
    P: ConstPrefix,
{
    fn into_keys(self) -> Vec<Vec<u8>> {
        vec![P::PREFIX.to_string().into_bytes(), self.key.into()]
    }
}

#[derive(Clone, Debug)]
pub enum CwAbciKey {
    Item {
        contract_address: AccountId,
        storage_key: String,
    },
    Map {
        contract_address: AccountId,
        storage_key: String,
        storage_namespace: String,
    },
}

impl CwAbciKey {
    pub fn new(
        contract_address: AccountId,
        storage_key: String,
        storage_namespace: Option<String>,
    ) -> Self {
        if let Some(storage_namespace) = storage_namespace {
            Self::Map {
                contract_address,
                storage_key,
                storage_namespace,
            }
        } else {
            Self::Item {
                contract_address,
                storage_key,
            }
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }

    fn into_tuple(self) -> (AccountId, String, Option<String>) {
        match self {
            CwAbciKey::Item {
                contract_address,
                storage_key,
            } => (contract_address, storage_key, None),
            CwAbciKey::Map {
                contract_address,
                storage_key,
                storage_namespace,
            } => (contract_address, storage_key, Some(storage_namespace)),
        }
    }

    // Copied from cw-storage-plus
    fn encode_length(namespace: &[u8]) -> [u8; 2] {
        assert!(
            namespace.len() <= 0xFFFF,
            "only supports namespaces up to length 0xFFFF"
        );

        let length_bytes = (namespace.len() as u32).to_be_bytes();
        [length_bytes[2], length_bytes[3]]
    }
}

impl From<CwAbciKey> for Vec<u8> {
    fn from(value: CwAbciKey) -> Self {
        let (contract_address, storage_key, storage_namespace) = value.into_tuple();

        let mut data = vec![CONTRACT_STORE_PREFIX];
        data.append(&mut contract_address.to_bytes());
        if let Some(namespace) = storage_namespace {
            data.extend_from_slice(&CwAbciKey::encode_length(namespace.as_bytes()));
            data.append(&mut namespace.into_bytes());
        }
        data.append(&mut storage_key.into_bytes());

        data
    }
}
