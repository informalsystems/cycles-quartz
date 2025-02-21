use quartz_common::enclave::{handler::Handler, DefaultSharedEnclave};
use tonic::{Request, Response, Status};

use crate::proto::{
    settlement_server::Settlement, QueryRequest, QueryResponse, UpdateRequest, UpdateResponse,
};

#[tonic::async_trait]
impl Settlement for DefaultSharedEnclave<()> {
    async fn run(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        let response = request.handle(self).await?;
        Ok(response.map(|r| UpdateResponse {
            message: serde_json::to_string(&r).unwrap(),
        }))
    }

    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let response = request.handle(self).await?;
        Ok(response.map(|r| QueryResponse {
            message: serde_json::to_string(&r).unwrap(),
        }))
    }
}
