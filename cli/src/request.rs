use crate::{
    cli::{Command, ContractCommand, EnclaveCommand},
    error::Error,
    request::{
        contract_build::ContractBuildRequest, contract_deploy::ContractDeployRequest,
        enclave_build::EnclaveBuildRequest, enclave_start::EnclaveStartRequest,
        contract_query::ContractQueryRequest,
        handshake::HandshakeRequest, init::InitRequest,
    },
};

pub mod contract_build;
pub mod contract_deploy;
pub mod contract_query;
pub mod enclave_build;
pub mod enclave_start;
pub mod handshake;
pub mod init;

#[derive(Clone, Debug)]
pub enum Request {
    Init(InitRequest),
    Handshake(HandshakeRequest),
    ContractBuild(ContractBuildRequest),
    ContractDeploy(ContractDeployRequest),
    ContractQuery(ContractQueryRequest),
    EnclaveBuild(EnclaveBuildRequest),
    EnclaveStart(EnclaveStartRequest),
}

impl TryFrom<Command> for Request {
    type Error = Error;

    fn try_from(cmd: Command) -> Result<Self, Self::Error> {
        match cmd {
            Command::Init(args) => Ok(InitRequest { name: args.name }.try_into()?),
            Command::Handshake(args) => Ok(HandshakeRequest {
                contract: args.contract,
            }
            .into()),
            Command::Contract { contract_command } => contract_command.try_into(),
            Command::Enclave { enclave_command } => enclave_command.try_into(),
        }
    }
}

impl TryFrom<ContractCommand> for Request {
    type Error = Error;

    fn try_from(cmd: ContractCommand) -> Result<Request, Error> {
        match cmd {
            ContractCommand::Deploy(args) => {
                if !args.wasm_bin_path.exists() {
                    return Err(Error::PathNotFile(args.wasm_bin_path.display().to_string()));
                }

                Ok(ContractDeployRequest {
                    init_msg: serde_json::from_str(&args.init_msg)
                        .map_err(|e| Error::GenericErr(e.to_string()))?,
                    label: args.label,
                    wasm_bin_path: args.wasm_bin_path,
                }
                .into())
            }
            ContractCommand::Build(args) => {
                if !args.manifest_path.exists() {
                    return Err(Error::PathNotFile(args.manifest_path.display().to_string()));
                }

                Ok(ContractBuildRequest {
                    manifest_path: args.manifest_path,
                }
                .into())
            }
            ContractCommand::Query(args) => {
                Ok(ContractQueryRequest {
                    contract: args.contract,
                    query_msg: args.query_msg,
                }
                .into())
            },
        }
    }
}

impl TryFrom<EnclaveCommand> for Request {
    type Error = Error;

    fn try_from(cmd: EnclaveCommand) -> Result<Request, Error> {
        match cmd {
            EnclaveCommand::Build(args) => Ok(EnclaveBuildRequest {
                manifest_path: args.manifest_path,
            }
            .into()),
            EnclaveCommand::Start(args) => Ok(EnclaveStartRequest {
                use_latest_trusted: args.use_latest_trusted,
            }
            .into()),
        }
    }
}
