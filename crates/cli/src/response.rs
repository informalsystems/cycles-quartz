use serde::Serialize;

use crate::response::{
    contract_build::ContractBuildResponse, contract_deploy::ContractDeployResponse,
    dev::DevResponse, enclave_build::EnclaveBuildResponse, enclave_start::EnclaveStartResponse,
    handshake::HandshakeResponse, init::InitResponse, print_fmspc::PrintFmspcResponse,
};

pub mod contract_build;
pub mod contract_deploy;
pub mod dev;
pub mod enclave_build;
pub mod enclave_start;
pub mod handshake;
pub mod init;

pub mod print_fmspc;

#[derive(Clone, Debug, Serialize)]
pub enum Response {
    Init(InitResponse),
    Handshake(HandshakeResponse),
    ContractBuild(ContractBuildResponse),
    ContractDeploy(ContractDeployResponse),
    EnclaveBuild(EnclaveBuildResponse),
    EnclaveStart(EnclaveStartResponse),
    Dev(DevResponse),
    PrintFmspc(PrintFmspcResponse),
}
