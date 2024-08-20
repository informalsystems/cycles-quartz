use cosmrs::AccountId;

use crate::request::Request;

#[derive(Clone, Debug)]
pub struct ContractQueryRequest {
    pub contract: AccountId,
    pub query_msg: String,
}

impl From<ContractQueryRequest> for Request {
    fn from(request: ContractQueryRequest) -> Self {
        Self::ContractQuery(request)
    }
}