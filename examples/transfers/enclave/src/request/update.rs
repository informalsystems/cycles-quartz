use std::collections::btree_map::Entry;

use cosmwasm_std::{Addr, HexBinary, Uint128};
use quartz_common::enclave::{
    handler::{ensure_seq_num_consistency, Handler},
    key_manager::KeyManager,
    proof_of_publication::ProofOfPublication,
    store::Store,
    DefaultSharedEnclave, Enclave,
};
use serde::{Deserialize, Serialize};
use tonic::Status;
use transfers_contract::{
    msg::execute::{ClearTextTransferRequestMsg, Request as TransferRequest, UpdateMsg},
    state::REQUESTS_KEY,
};

use crate::{
    proto::UpdateRequest,
    request::{decrypt_state, decrypt_transfer, encrypt_state},
    state::State,
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UpdateRequestMessage {
    pub state: HexBinary,
    pub requests: Vec<TransferRequest>,
    pub seq_num: u64,
}

#[async_trait::async_trait]
impl Handler<DefaultSharedEnclave<()>> for UpdateRequest {
    type Error = Status;
    type Response = UpdateMsg;

    async fn handle(self, ctx: &DefaultSharedEnclave<()>) -> Result<Self::Response, Self::Error> {
        // verify proof
        let proof: ProofOfPublication<UpdateRequestMessage> = {
            let message = self.message;
            serde_json::from_str(&message).map_err(|e| Status::invalid_argument(e.to_string()))?
        };
        let contract = ctx
            .store()
            .await
            .get_contract()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("contract not found"))?;
        let config = ctx
            .store()
            .await
            .get_config()
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("config not found"))?;
        let (proof_value, message) = proof
            .verify(
                config.light_client_opts(),
                contract,
                REQUESTS_KEY.to_string(),
                None,
            )
            .map_err(Status::failed_precondition)?;

        let proof_value_matches_msg =
            serde_json::to_string(&message.requests).is_ok_and(|s| s.as_bytes() == proof_value);
        if !proof_value_matches_msg {
            return Err(Status::failed_precondition("proof verification"));
        }

        // ensure sequence number consistency
        // TODO: move this into the core?
        let pending_sequenced_requests = message
            .requests
            .iter()
            .filter(|req| matches!(req, TransferRequest::Transfer(_)))
            .count();
        if pending_sequenced_requests > 0 {
            let seq_num = ctx
                .store()
                .await
                .get_seq_num()
                .await
                .map_err(|_| Status::internal("store read error"))?;
            ensure_seq_num_consistency(seq_num, message.seq_num, pending_sequenced_requests)?;
            ctx.store()
                .await
                .inc_seq_num(pending_sequenced_requests)
                .await
                .map_err(|_| Status::internal("store read error"))?;
        }

        // Decrypt and deserialize the state
        let mut state = match &message.state.to_vec()[..] {
            &[0] => State::default(),
            state_bytes => {
                let sk = ctx
                    .key_manager
                    .priv_key()
                    .await
                    .ok_or_else(|| Status::internal("failed to get private key"))?;
                decrypt_state(&sk, state_bytes)?
            }
        };

        let requests_len = message.requests.len() as u32;

        // Instantiate empty withdrawals map to include in response (Update message to smart contract)
        let mut withdrawals_response: Vec<(Addr, Uint128)> = Vec::<(Addr, Uint128)>::new();

        // Loop through requests, match on cases, and apply changes to state
        for req in message.requests {
            match req {
                TransferRequest::Transfer(ciphertext) => {
                    // Decrypt transfer ciphertext into cleartext struct (acquires lock on enclave sk to do so)
                    let transfer: ClearTextTransferRequestMsg = {
                        let sk = ctx
                            .key_manager
                            .priv_key()
                            .await
                            .ok_or_else(|| Status::internal("failed to get private key"))?;

                        decrypt_transfer(&sk, &ciphertext)?
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
                    }
                }
                TransferRequest::Withdraw(receiver) => {
                    // If a user with no balance requests withdraw, withdraw request for 0 coins gets processed
                    // TODO: A no-op seems like a bad design choice in a privacy system
                    if let Some(withdraw_bal) = state.state.remove(&receiver) {
                        withdrawals_response.push((receiver, withdraw_bal));
                    }
                }
                TransferRequest::Deposit(sender, amount) => {
                    state
                        .state
                        .entry(sender)
                        .and_modify(|bal| *bal += amount)
                        .or_insert(amount);
                }
            }
        }

        // Encrypt state
        let state_enc = {
            let pk = ctx
                .key_manager()
                .await
                .pub_key()
                .await
                .ok_or_else(|| Status::internal("failed to get public key"))?;

            encrypt_state(state, pk).map_err(|e| Status::invalid_argument(e.to_string()))?
        };

        // Prepare message to chain
        let msg = UpdateMsg {
            ciphertext: state_enc,
            quantity: requests_len,
            withdrawals: withdrawals_response,
        };

        Ok(msg)
    }
}
