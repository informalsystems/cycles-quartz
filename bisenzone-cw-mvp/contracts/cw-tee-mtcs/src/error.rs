use cosmwasm_std::StdError;
use libsecp256k1::Error as SecpError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid pubkey")]
    InvalidPubKey(SecpError),
}

impl From<SecpError> for ContractError {
    fn from(e: SecpError) -> Self {
        Self::InvalidPubKey(e)
    }
}
