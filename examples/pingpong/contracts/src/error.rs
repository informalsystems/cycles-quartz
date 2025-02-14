use cosmwasm_std::StdError;
use quartz_contract_core::error::Error as QuartzError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Quartz(#[from] QuartzError),
}
