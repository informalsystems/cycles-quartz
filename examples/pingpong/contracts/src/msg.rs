use cosmwasm_schema::cw_serde;
use quartz_contract_core::{
    msg::execute::attested::{RawAttested, RawDefaultAttestation, RawNoop},
    prelude::*,
};

pub type AttestedMsg<M, RA = RawDefaultAttestation> = RawAttested<RawNoop<M>, RA>;

#[cw_serde]
pub struct InstantiateMsg<RA = RawDefaultAttestation> {
    pub quartz: QuartzInstantiateMsg<RA>,
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg<RA = RawDefaultAttestation> {
    // Quartz initialization
    Quartz(QuartzExecuteMsg<RA>),
    // User message
    Ping(execute::Ping),
    // Enclave message
    Pong(AttestedMsg<execute::Pong, RA>),
}

pub mod execute {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::HexBinary;
    use quartz_contract_core_derive::UserData;

    #[cw_serde]
    pub struct Ping {
        pub pubkey: HexBinary,
        pub message: HexBinary,
    }

    #[derive(UserData)]
    #[cw_serde]
    pub struct Pong {
        pub pubkey: HexBinary,
        pub response: HexBinary,
    }
}

#[cw_serde]
pub enum QueryMsg {
    GetAllMessages {},
}
