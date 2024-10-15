use cosmwasm_schema::cw_serde;
use quartz_common::contract::{
    msg::execute::{
        attested::{RawAttested, RawAttestedMsgSansHandler, RawDefaultAttestation},
        sequenced::RawSequencedMsg,
    },
    prelude::*,
};

pub type AttestedMsg<M, RA = RawDefaultAttestation> = RawAttested<RawAttestedMsgSansHandler<M>, RA>;
pub type SequencedMsgSansHandler<M> = RawSequencedMsg<RawAttestedMsgSansHandler<M>>;

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
    TransferRequest(SequencedMsgSansHandler<execute::TransferRequestMsg>),
    QueryRequest(SequencedMsgSansHandler<execute::QueryRequestMsg>),

    // Enclave msgs
    Update(AttestedMsg<execute::UpdateMsg, RA>),
    QueryResponse(AttestedMsg<execute::QueryResponseMsg, RA>),
}

pub mod execute {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, HexBinary, Uint128};
    use quartz_common::contract::{msg::execute::attested::HasUserData, state::UserData};
    use sha2::{Digest, Sha256};

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

    #[cw_serde]
    pub struct UpdateMsg {
        pub ciphertext: HexBinary,
        pub quantity: u32,
        pub withdrawals: Vec<(Addr, Uint128)>,
        // pub proof: π
    }

    impl HasUserData for UpdateMsg {
        fn user_data(&self) -> UserData {
            let mut hasher = Sha256::new();
            hasher.update(serde_json::to_string(&self).expect("infallible serializer"));
            let digest: [u8; 32] = hasher.finalize().into();

            let mut user_data = [0u8; 64];
            user_data[0..32].copy_from_slice(&digest);
            user_data
        }
    }

    #[cw_serde]
    pub struct QueryResponseMsg {
        pub address: Addr,
        pub encrypted_bal: HexBinary,
        // pub proof: π
    }

    impl HasUserData for QueryResponseMsg {
        fn user_data(&self) -> UserData {
            let mut hasher = Sha256::new();
            hasher.update(serde_json::to_string(&self).expect("infallible serializer"));
            let digest: [u8; 32] = hasher.finalize().into();

            let mut user_data = [0u8; 64];
            user_data[0..32].copy_from_slice(&digest);
            user_data
        }
    }
}
