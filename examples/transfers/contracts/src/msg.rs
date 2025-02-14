use cosmwasm_schema::cw_serde;
use quartz_contract_core::{
    msg::execute::{
        attested::{RawAttested, RawDefaultAttestation, RawNoop},
        sequenced::RawSequenced,
    },
    prelude::*,
};

pub type AttestedMsg<M, RA = RawDefaultAttestation> = RawAttested<RawNoop<M>, RA>;
pub type SequencedMsg<M> = RawSequenced<RawNoop<M>>;

#[cw_serde]
pub struct InstantiateMsg<RA = RawDefaultAttestation> {
    pub quartz: QuartzInstantiateMsg<RA>,
    pub denom: String,
}

#[cw_serde]
pub enum QueryMsg {
    GetBalance { address: String },
    GetRequests {},
    GetState {},
}

#[cw_serde]
#[allow(clippy::large_enum_variant)]
pub enum ExecuteMsg<RA = RawDefaultAttestation> {
    // quartz initialization
    Quartz(QuartzExecuteMsg<RA>),

    // User msgs
    // clear text
    Deposit,
    Withdraw,
    ClearTextTransferRequest(execute::ClearTextTransferRequestMsg),
    // ciphertext
    TransferRequest(SequencedMsg<execute::TransferRequestMsg>),
    QueryRequest(execute::QueryRequestMsg),

    // Enclave msgs
    Update(AttestedMsg<execute::UpdateMsg, RA>),
    QueryResponse(AttestedMsg<execute::QueryResponseMsg, RA>),
}

pub mod execute {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, HexBinary, Uint128};
    use quartz_contract_core_derive::UserData;

    #[cw_serde]
    pub struct ClearTextTransferRequestMsg {
        pub sender: Addr,
        pub receiver: Addr,
        pub amount: Uint128,
        // pub proof: π
    }

    #[cw_serde]
    pub struct QueryRequestMsg {
        pub emphemeral_pubkey: HexBinary,
    }

    #[cw_serde]
    pub struct TransferRequestMsg {
        pub ciphertext: HexBinary,
        pub digest: HexBinary,
        // pub proof: π
    }

    #[cw_serde]
    pub enum Request {
        Transfer(HexBinary),
        Withdraw(Addr),
        Deposit(Addr, Uint128),
    }

    #[derive(UserData)]
    #[cw_serde]
    pub struct UpdateMsg {
        pub ciphertext: HexBinary,
        pub quantity: u32,
        pub withdrawals: Vec<(Addr, Uint128)>,
        // pub proof: π
    }

    #[derive(UserData)]
    #[cw_serde]
    pub struct QueryResponseMsg {
        pub address: Addr,
        pub encrypted_bal: HexBinary,
        // pub proof: π
    }
}
