use serde::Serialize;
use cosmwasm_std::HexBinary;

use crate::response::Response;

#[derive(Clone, Debug, Serialize, Default)]
pub struct ContractQueryResponse {
    pub account: HexBinary, // encrypted
    pub balance: HexBinary, // encrypted
}

impl From<ContractQueryResponse> for Response {
    fn from(response: ContractQueryResponse) -> Self {
        Self::ContractQuery(response)
    }
}