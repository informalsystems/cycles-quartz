use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{Arc, Mutex},
};

use cosmwasm_std::{Addr, HexBinary, Uint128};

pub type RawCipherText = HexBinary;

use ecies::{decrypt, encrypt};
use k256::ecdsa::{SigningKey, VerifyingKey};
use quartz_cw::{
    msg::execute::attested::{HasUserData, RawAttested},
    state::UserData,
};
use quartz_enclave::attestor::Attestor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Result as TonicResult, Status};
use transfers_contracts::msg::execute::{ClearTextTransferRequestMsg, Request as TransfersRequest};

use crate::{
    proto::{
        settlement_server::Settlement, QueryRequest, QueryResponse, RunTransfersRequest,
        RunTransfersResponse,
    },
    state::{RawBalance, RawState, State},
};

#[derive(Clone, Debug)]
pub struct TransfersService<A> {
    sk: Arc<Mutex<Option<SigningKey>>>,
    attestor: A,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunTransfersRequestMessage {
    state: HexBinary,
    requests: Vec<TransfersRequest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunTransfersResponseMessage {
    ciphertext: HexBinary,
    quantity: u32,
    withdrawals: Vec<(Addr, Uint128)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]

pub struct QueryRequestMessage {
    state: HexBinary,
    address: Addr,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResponseMessage {
    encrypted_bal: HexBinary,
}

impl HasUserData for RunTransfersResponseMessage {
    fn user_data(&self) -> UserData {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(&self).expect("infallible serializer"));
        let digest: [u8; 32] = hasher.finalize().into();

        let mut user_data = [0u8; 64];
        user_data[0..32].copy_from_slice(&digest);
        user_data
    }
}

impl HasUserData for QueryResponseMessage {
    fn user_data(&self) -> UserData {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(&self).expect("infallible serializer"));
        let digest: [u8; 32] = hasher.finalize().into();

        let mut user_data = [0u8; 64];
        user_data[0..32].copy_from_slice(&digest);
        user_data
    }
}

// TODO: this should probably just be an import from quartz
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AttestedMsg<M> {
    msg: M,
    quote: Vec<u8>,
}

impl<A> TransfersService<A>
where
    A: Attestor,
{
    pub fn new(sk: Arc<Mutex<Option<SigningKey>>>, attestor: A) -> Self {
        Self { sk, attestor }
    }
}

#[tonic::async_trait]
impl<A> Settlement for TransfersService<A>
where
    A: Attestor + Send + Sync + 'static,
{
    async fn run(
        &self,
        request: Request<RunTransfersRequest>,
    ) -> TonicResult<Response<RunTransfersResponse>> {
        // Request contains a serialized json string

        // Serialize request into struct containing State and the Requests vec
        let message: RunTransfersRequestMessage = {
            let message = request.into_inner().message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Decrypt and deserialize the state
        let mut state = {
            if message.state.len() == 1 && message.state[0] == 0 {
                println!("{}", message.state);

                State {
                    state: BTreeMap::<Addr, Uint128>::new(),
                }
            } else {
                let sk_lock = self
                    .sk
                    .lock()
                    .map_err(|e| Status::internal(e.to_string()))?;
                let sk = sk_lock
                    .as_ref()
                    .ok_or(Status::internal("SigningKey unavailable"))?;

                decrypt_state(sk, &message.state)?
            }
        };

        let requests_len = message.requests.len() as u32;
        // Instantiate empty withdrawals map to include in response (Update message to smart contract)
        let mut withdrawals_response: Vec<(Addr, Uint128)> = Vec::<(Addr, Uint128)>::new();

        // Loop through requests, match on cases, and apply changes to state
        for req in message.requests {
            match req {
                TransfersRequest::Transfer(ciphertext) => {
                    // Decrypt transfer ciphertext into cleartext struct (acquires lock on enclave sk to do so)
                    let transfer: ClearTextTransferRequestMsg = {
                        let sk_lock = self
                            .sk
                            .lock()
                            .map_err(|e| Status::internal(e.to_string()))?;
                        let sk = sk_lock
                            .as_ref()
                            .ok_or(Status::internal("SigningKey unavailable"))?;

                        decrypt_transfer(sk, &ciphertext)?
                    };
                    if let Entry::Occupied(mut entry) = state.state.entry(transfer.sender) {
                        let balance = entry.get();
                        if balance >= &transfer.amount {
                            entry.insert(balance - transfer.amount);

                            state
                                .state
                                .entry(transfer.receiver)
                                .and_modify(|bal| *bal += transfer.amount)
                                .or_insert(transfer.amount);
                        }
                        // TODO: handle errors
                    }
                }
                TransfersRequest::Withdraw(receiver) => {
                    // If a user with no balance requests withdraw, withdraw request for 0 coins gets processed
                    // TODO: A no-op seems like a bad design choice in a privacy system
                    if let Some(withdraw_bal) = state.state.remove(&receiver) {
                        withdrawals_response.push((receiver, withdraw_bal));
                    }
                }
                TransfersRequest::Deposit(sender, amount) => {
                    state
                        .state
                        .entry(sender)
                        .and_modify(|bal| *bal += amount)
                        .or_insert(amount);
                }
            }
        }

        // Encrypt state
        // Gets lock on PrivKey, generates PubKey to encrypt with
        let state_enc = {
            let sk_lock = self
                .sk
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?;
            let pk = VerifyingKey::from(
                sk_lock
                    .as_ref()
                    .ok_or(Status::internal("SigningKey unavailable"))?,
            );

            encrypt_state(RawState::from(state), pk)
                .map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Prepare message to chain
        let msg = RunTransfersResponseMessage {
            ciphertext: state_enc,
            quantity: requests_len,
            withdrawals: withdrawals_response,
        };

        // Attest to message
        let attestation = self
            .attestor
            .quote(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;

        let attested_msg = RawAttested { msg, attestation };
        let message =
            serde_json::to_string(&attested_msg).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RunTransfersResponse { message }))
    }

    // TODO - serialize the request into the account address (sender)
    async fn query(&self, request: Request<QueryRequest>) -> TonicResult<Response<QueryResponse>> {
        // Request contains a serialized json string

        // Serialize request into struct containing State and the Requests vec
        let message: QueryRequestMessage = {
            let message = request.into_inner().message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        println!("query request message: {:?}", message);

        // Decrypt and deserialize the state
        let mut state = {
            if message.state.len() == 1 && message.state[0] == 0 {
                println!("{}", message.state);

                State {
                    state: BTreeMap::<Addr, Uint128>::new(),
                }
            } else {
                let sk_lock = self
                    .sk
                    .lock()
                    .map_err(|e| Status::internal(e.to_string()))?;
                let sk = sk_lock
                    .as_ref()
                    .ok_or(Status::internal("SigningKey unavailable"))?;

                decrypt_state(sk, &message.state)?
            }
        };

        println!("state gotten {:?}", state.state);

        let mut bal: RawBalance;

        match state.state.entry(message.address.clone()) {
            Entry::Occupied(entry) => {
                bal = RawBalance {
                    balance: *entry.get(),
                };
            }
            Entry::Vacant(entry) => {
                entry.insert(Uint128::new(0));
                bal = RawBalance {
                    balance: Uint128::new(0),
                };
            }
        }

        println!("bal gotten {:?}", bal);

        // Encrypt the balance
        // Gets lock on PrivKey, generates PubKey to encrypt with
        let bal_enc = {
            let sk_lock = self
                .sk
                .lock()
                .map_err(|e| Status::internal(e.to_string()))?;
            let pk = VerifyingKey::from(
                sk_lock
                    .as_ref()
                    .ok_or(Status::internal("SigningKey unavailable"))?,
            );

            encrypt_balance(RawBalance::from(bal), pk) // TODO update this
                .map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Prepare message to chain
        let msg = QueryResponseMessage {
            encrypted_bal: bal_enc,
        };

        println!("query RESPONSE message: {:?}", msg);

        // Attest to message
        let quote = self
            .attestor
            .quote(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;

        let attested_msg = AttestedMsg { msg, quote };
        println!("attested mesg {:?}", attested_msg);
        let message =
            serde_json::to_string(&attested_msg).map_err(|e| Status::internal(e.to_string()))?;
            println!("final  mesg {:?}", message);

        Ok(Response::new(QueryResponse { message }))
            // Ok(Response::new(QueryResponse { message }))

        // Ok(Response::new(QueryResponse { message }))
        // Ok(Response::new(QueryResponse {
        //     message: "".to_string(),
        // }))
    }
}

//TODO: consider using generics for these decrypt functions
fn decrypt_transfer(
    sk: &SigningKey,
    ciphertext: &HexBinary,
) -> TonicResult<ClearTextTransferRequestMsg> {
    let o =
        decrypt(&sk.to_bytes(), ciphertext).map_err(|e| Status::invalid_argument(e.to_string()))?;

    serde_json::from_slice(&o)
        .map_err(|e| Status::internal(format!("Could not deserialize transfer {}", e)))
}

fn decrypt_state(sk: &SigningKey, ciphertext: &HexBinary) -> TonicResult<State> {
    let o: RawState = {
        let o = decrypt(&sk.to_bytes(), ciphertext)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        serde_json::from_slice(&o).map_err(|e| Status::invalid_argument(e.to_string()))?
    };

    State::try_from(o).map_err(|e| Status::internal(format!("Could not deserialize state {}", e)))
}

fn encrypt_state(state: RawState, enclave_pk: VerifyingKey) -> TonicResult<RawCipherText> {
    let serialized_state = serde_json::to_string(&state).expect("infallible serializer");

    match encrypt(&enclave_pk.to_sec1_bytes(), serialized_state.as_bytes()) {
        Ok(encrypted_state) => Ok(encrypted_state.into()),
        Err(e) => Err(Status::internal(format!("Encryption error: {}", e))),
    }
}

fn encrypt_balance(state: RawBalance, enclave_pk: VerifyingKey) -> TonicResult<RawCipherText> {
    let serialized_state = serde_json::to_string(&state).expect("infallible serializer");

    match encrypt(&enclave_pk.to_sec1_bytes(), serialized_state.as_bytes()) {
        Ok(encrypted_state) => Ok(encrypted_state.into()),
        Err(e) => Err(Status::internal(format!("Encryption error: {}", e))),
    }
}
