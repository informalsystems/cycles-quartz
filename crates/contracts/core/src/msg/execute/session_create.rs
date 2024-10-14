use cosmrs::AccountId;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, StdError};
use sha2::{Digest, Sha256};

use crate::{
    msg::{execute::attested::HasUserData, HasDomainType},
    state::{Nonce, UserData},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SessionCreate {
    nonce: Nonce,
    contract: AccountId,
}

impl SessionCreate {
    pub fn new(nonce: Nonce, contract: AccountId) -> Self {
        Self { nonce, contract }
    }

    pub fn into_nonce(self) -> Nonce {
        self.nonce
    }
}

#[cw_serde]
pub struct RawSessionCreate {
    nonce: HexBinary,
    contract: String,
}

impl TryFrom<RawSessionCreate> for SessionCreate {
    type Error = StdError;

    fn try_from(value: RawSessionCreate) -> Result<Self, Self::Error> {
        let nonce = value.nonce.to_array()?;
        let contract = value
            .contract
            .parse()
            .map_err(|e| StdError::parse_err("AccountId", e))?;
        Ok(Self { nonce, contract })
    }
}

impl From<SessionCreate> for RawSessionCreate {
    fn from(value: SessionCreate) -> Self {
        Self {
            nonce: value.nonce.into(),
            contract: value.contract.to_string(),
        }
    }
}

impl HasDomainType for RawSessionCreate {
    type DomainType = SessionCreate;
}

impl HasUserData for SessionCreate {
    fn user_data(&self) -> UserData {
        let mut hasher = Sha256::new();
        hasher.update(
            serde_json::to_string(&RawSessionCreate::from(self.clone()))
                .expect("infallible serializer"),
        );
        let digest: [u8; 32] = hasher.finalize().into();

        let mut user_data = [0u8; 64];
        user_data[0..32].copy_from_slice(&digest);
        user_data
    }
}
