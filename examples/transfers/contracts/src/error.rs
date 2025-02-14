use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use quartz_contract_core::error::Error as QuartzError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Quartz(#[from] QuartzError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Duplicate entry found")]
    DuplicateEntry,

    #[error("Invalid length")]
    BadLength,

    #[error("Payment error: {0}")]
    CwUtil(PaymentError),
}

impl From<PaymentError> for ContractError {
    fn from(e: PaymentError) -> Self {
        Self::CwUtil(e)
    }
}
