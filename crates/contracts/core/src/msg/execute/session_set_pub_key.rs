use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, StdError};
use sha2::{Digest, Sha256};

use crate::{
    msg::{execute::attested::HasUserData, HasDomainType},
    state::{Nonce, UserData},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SessionSetPubKey {
    nonce: Nonce,
    pub_key: Vec<u8>,
}

impl SessionSetPubKey {
    pub fn new(nonce: Nonce, pub_key: Vec<u8>) -> Self {
        Self { nonce, pub_key }
    }

    pub fn into_tuple(self) -> (Nonce, Vec<u8>) {
        (self.nonce, self.pub_key)
    }
}

#[cw_serde]
pub struct RawSessionSetPubKey {
    nonce: HexBinary,
    pub_key: HexBinary,
}

impl RawSessionSetPubKey {
    pub fn pub_key(&self) -> &HexBinary {
        &self.pub_key
    }
}

impl TryFrom<RawSessionSetPubKey> for SessionSetPubKey {
    type Error = StdError;

    fn try_from(value: RawSessionSetPubKey) -> Result<Self, Self::Error> {
        let nonce = value.nonce.to_array()?;
        Ok(Self {
            nonce,
            pub_key: value.pub_key.into(),
        })
    }
}

impl From<SessionSetPubKey> for RawSessionSetPubKey {
    fn from(value: SessionSetPubKey) -> Self {
        Self {
            nonce: value.nonce.into(),
            pub_key: value.pub_key.into(),
        }
    }
}

impl HasDomainType for RawSessionSetPubKey {
    type DomainType = SessionSetPubKey;
}

impl HasUserData for SessionSetPubKey {
    fn user_data(&self) -> UserData {
        let mut hasher = Sha256::new();
        hasher.update(
            serde_json::to_string(&RawSessionSetPubKey::from(self.clone()))
                .expect("infallible serializer"),
        );
        let digest: [u8; 32] = hasher.finalize().into();

        let mut user_data = [0u8; 64];
        user_data[0..32].copy_from_slice(&digest);
        user_data
    }
}
