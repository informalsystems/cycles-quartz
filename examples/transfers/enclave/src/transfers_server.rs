use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{Arc, Mutex},
};

use cosmrs::AccountId;
use cosmwasm_std::{Addr, HexBinary, Uint128};
use ecies::{decrypt, encrypt};
use k256::ecdsa::{SigningKey, VerifyingKey};
use quartz_common::{
    contract::{
        msg::execute::attested::{HasUserData, RawAttested},
        state::{Config, UserData},
    },
    enclave::{
        attestor::Attestor,
        server::{AppService, CoreMsg, IntoServer, ProofOfPublication, WsListenerConfig},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Result as TonicResult, Status};
use transfers_contract::msg::execute::{ClearTextTransferRequestMsg, Request as TransfersRequest};

use crate::{
    proto::{
        settlement_server::{Settlement, SettlementServer},
        QueryRequest, QueryResponse, UpdateRequest, UpdateResponse,
    },
    state::{RawBalance, RawState, State},
};

impl<A: Attestor> IntoServer for TransfersService<A> {
    type Server = SettlementServer<TransfersService<A>>;

    fn into_server(self) -> Self::Server {
        SettlementServer::new(self)
    }
}

impl<A: Attestor> AppService for TransfersService<A> {
    fn accept_channel(&mut self, tx: Sender<CoreMsg>) {
        self.tx = Some(tx);
    }
}

pub type RawCipherText = HexBinary;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UpdateRequestMessage {
    pub state: HexBinary,
    pub requests: Vec<TransfersRequest>,
    pub seq_num: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRequestMessage {
    pub state: HexBinary,
    pub address: Addr,
    pub ephemeral_pubkey: HexBinary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResponseMessage {
    address: Addr,
    encrypted_bal: HexBinary,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct UpdateMsg {
    ciphertext: HexBinary,
    quantity: u32,
    withdrawals: Vec<(Addr, Uint128)>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusResponseMessage {
    address: Addr,
    encrypted_bal: HexBinary,
}

#[derive(Clone, Debug)]
pub enum TransfersOpEvent {
    Query {
        contract_address: AccountId,
        sender: String,
        ephemeral_pubkey: String,
    },
    Transfer {
        contract_address: AccountId,
    },
}

#[derive(Clone, Debug)]
pub struct TransfersOp<A: Attestor> {
    pub client: TransfersService<A>,
    pub event: TransfersOpEvent,
    pub config: WsListenerConfig,
}

#[derive(Clone, Debug)]
pub struct TransfersService<A: Attestor> {
    config: Config,
    sk: Arc<Mutex<Option<SigningKey>>>,
    attestor: A,
    pub queue_producer: Sender<TransfersOp<A>>,
    tx: Option<Sender<CoreMsg>>,
}

impl<A> TransfersService<A>
where
    A: Attestor,
{
    pub fn new(
        config: Config,
        sk: Arc<Mutex<Option<SigningKey>>>,
        attestor: A,
        queue_producer: Sender<TransfersOp<A>>,
    ) -> Self {
        Self {
            config,
            sk,
            attestor,
            queue_producer,
            tx: None,
        }
    }
}

#[tonic::async_trait]
impl<A> Settlement for TransfersService<A>
where
    A: Attestor + Send + Sync + 'static,
{
    async fn run(&self, request: Request<UpdateRequest>) -> TonicResult<Response<UpdateResponse>> {
        // Serialize request into struct containing State and the Requests vec
        let message: ProofOfPublication<UpdateRequestMessage> = {
            let message = request.into_inner().message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        let (proof_value, message) = message
            .verify(self.config.light_client_opts())
            .map_err(Status::failed_precondition)?;

        let proof_value_matches_msg =
            serde_json::to_string(&message.requests).is_ok_and(|s| s.as_bytes() == proof_value);
        if !proof_value_matches_msg {
            return Err(Status::failed_precondition("proof verification"));
        }

        // Decrypt and deserialize the state
        let mut state = {
            if message.state.len() == 1 && message.state[0] == 0 {
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
        let msg = UpdateMsg {
            ciphertext: state_enc,
            quantity: requests_len,
            withdrawals: withdrawals_response,
        };

        // Attest to message
        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;

        let attested_msg = RawAttested {
            msg,
            attestation: A::RawAttestation::from(attestation),
        };
        let message =
            serde_json::to_string(&attested_msg).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UpdateResponse { message }))
    }

    async fn query(&self, request: Request<QueryRequest>) -> TonicResult<Response<QueryResponse>> {
        // Serialize request into struct containing State and the Requests vec
        let message: QueryRequestMessage = {
            let message: String = request.into_inner().message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Decrypt and deserialize the state
        let state = {
            if message.state.len() == 1 && message.state[0] == 0 {
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

        let bal = match state.state.get(&message.address) {
            Some(balance) => RawBalance { balance: *balance },
            None => RawBalance {
                balance: Uint128::new(0),
            },
        };

        // Parse the ephemeral public key
        let ephemeral_pubkey =
            VerifyingKey::from_sec1_bytes(&message.ephemeral_pubkey).map_err(|e| {
                Status::invalid_argument(format!("Invalid ephemeral public key: {}", e))
            })?;

        // Encrypt the balance using the ephemeral public key
        let bal_enc = encrypt_balance(bal, ephemeral_pubkey)
            .map_err(|e| Status::internal(format!("Encryption error: {}", e)))?;

        // Prepare message to chain
        let msg = QueryResponseMessage {
            address: message.address,
            encrypted_bal: bal_enc,
        };

        // Attest to message
        let attestation = self
            .attestor
            .attestation(msg.clone())
            .map_err(|e| Status::internal(e.to_string()))?;

        let attested_msg = RawAttested {
            msg,
            attestation: A::RawAttestation::from(attestation),
        };
        let message =
            serde_json::to_string(&attested_msg).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(QueryResponse { message }))
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

fn encrypt_balance(balance: RawBalance, ephemeral_pk: VerifyingKey) -> TonicResult<RawCipherText> {
    let serialized_balance = serde_json::to_string(&balance).expect("infallible serializer");

    match encrypt(&ephemeral_pk.to_sec1_bytes(), serialized_balance.as_bytes()) {
        Ok(encrypted_balance) => Ok(encrypted_balance.into()),
        Err(e) => Err(Status::internal(format!("Encryption error: {}", e))),
    }
}
