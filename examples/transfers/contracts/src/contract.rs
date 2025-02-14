use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, HexBinary, MessageInfo, Response,
    StdResult,
};
use quartz_contract_core::handler::RawHandler;

use crate::{
    error::ContractError,
    msg::{
        execute::{QueryResponseMsg, Request, UpdateMsg},
        ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    state::{BALANCES, DENOM, REQUESTS, STATE},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // must be handled first!
    msg.quartz.handle_raw(deps.branch(), &env, &info)?;

    DENOM.save(deps.storage, &msg.denom)?;

    let requests: Vec<Request> = Vec::new();
    REQUESTS.save(deps.storage, &requests)?;

    let state: HexBinary = HexBinary::from(&[0x00]);
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use execute::*;

    match msg {
        // Quartz msgs
        ExecuteMsg::Quartz(msg) => msg.handle_raw(deps, &env, &info).map_err(Into::into),

        // Clear user msgs
        ExecuteMsg::Deposit => deposit(deps, env, info),
        ExecuteMsg::Withdraw => withdraw(deps, env, info),
        ExecuteMsg::ClearTextTransferRequest(_) => unimplemented!(),
        ExecuteMsg::QueryRequest(msg) => query_balance(deps, env, info, msg),

        // Cipher user msgs
        ExecuteMsg::TransferRequest(msg) => {
            let _ = msg.clone().handle_raw(deps.branch(), &env, &info)?;
            transfer_request(deps, env, info, msg.0 .0)
        }

        // Enclave msgs
        ExecuteMsg::Update(attested_msg) => {
            let _ = attested_msg
                .clone()
                .handle_raw(deps.branch(), &env, &info)?;
            let UpdateMsg {
                ciphertext,
                quantity,
                withdrawals,
            } = attested_msg.msg.0;
            update(
                deps,
                env,
                info,
                UpdateMsg {
                    ciphertext,
                    quantity,
                    withdrawals,
                },
            )
        }

        ExecuteMsg::QueryResponse(attested_msg) => {
            let _ = attested_msg
                .clone()
                .handle_raw(deps.branch(), &env, &info)?;
            let QueryResponseMsg {
                address,
                encrypted_bal,
            } = attested_msg.msg.0;
            store_balance(
                deps,
                env,
                info,
                QueryResponseMsg {
                    address,
                    encrypted_bal,
                },
            )
        }
    }
}

pub mod execute {
    use cosmwasm_std::{coins, BankMsg, DepsMut, Env, Event, MessageInfo, Response};
    use cw_utils::must_pay;

    use crate::{
        error::ContractError,
        msg::execute::{QueryRequestMsg, QueryResponseMsg, Request, TransferRequestMsg, UpdateMsg},
        state::{BALANCES, DENOM, REQUESTS, STATE},
    };

    pub fn deposit(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let denom: String = DENOM.load(deps.storage)?;
        let quantity = must_pay(&info, &denom)?;

        let mut requests = REQUESTS.load(deps.storage)?;

        requests.push(Request::Deposit(info.sender, quantity));

        REQUESTS.save(deps.storage, &requests)?;

        let event = Event::new("transfer").add_attribute("action", "user");
        let resp = Response::new().add_event(event);

        Ok(resp)
    }

    pub fn withdraw(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        let mut requests = REQUESTS.load(deps.storage)?;

        requests.push(Request::Withdraw(info.sender));

        REQUESTS.save(deps.storage, &requests)?;

        let event = Event::new("transfer").add_attribute("action", "user");
        let resp = Response::new().add_event(event);

        Ok(resp)
    }

    pub fn query_balance(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: QueryRequestMsg,
    ) -> Result<Response, ContractError> {
        let event = Event::new("query_balance")
            .add_attribute("query", "user")
            .add_attribute("emphemeral_pubkey", msg.emphemeral_pubkey.to_string());
        let resp = Response::new().add_event(event);
        Ok(resp)
    }

    pub fn transfer_request(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: TransferRequestMsg,
    ) -> Result<Response, ContractError> {
        let mut requests = REQUESTS.load(deps.storage)?;

        requests.push(Request::Transfer(msg.ciphertext));

        REQUESTS.save(deps.storage, &requests)?;

        let event = Event::new("transfer").add_attribute("action", "user");
        let resp = Response::new().add_event(event);

        Ok(resp)
    }

    pub fn update(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: UpdateMsg,
    ) -> Result<Response, ContractError> {
        // Store state
        STATE.save(deps.storage, &msg.ciphertext)?;

        // Clear queue
        let mut requests: Vec<Request> = REQUESTS.load(deps.storage)?;

        requests.drain(0..msg.quantity as usize);

        REQUESTS.save(deps.storage, &requests)?;

        // Process withdrawals
        let denom = DENOM.load(deps.storage)?;

        let messages = msg
            .withdrawals
            .into_iter()
            .map(|(user, funds)| BankMsg::Send {
                to_address: user.to_string(),
                amount: coins(funds.into(), &denom),
            });

        let resp = Response::new().add_messages(messages);

        Ok(resp)
    }

    pub fn store_balance(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: QueryResponseMsg,
    ) -> Result<Response, ContractError> {
        // Store state
        BALANCES.save(deps.storage, msg.address.as_ref(), &msg.encrypted_bal)?;

        // Emit event
        let event = Event::new("store_balance")
            .add_attribute("query", "enclave") // TODO Weird to name it enclave?
            .add_attribute("address", msg.address.to_string())
            .add_attribute("encrypted_balance", msg.encrypted_bal.to_string());
        let resp = Response::new().add_event(event);
        Ok(resp)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance { address } => to_json_binary(&query::get_balance(deps, address)?),
        QueryMsg::GetRequests {} => to_json_binary(&query::get_requests(deps)?),
        QueryMsg::GetState {} => to_json_binary(&query::get_state(deps)?),
    }
}
mod query {
    use super::*;

    pub fn get_balance(deps: Deps, address: String) -> StdResult<HexBinary> {
        let balance = BALANCES.may_load(deps.storage, &address)?;
        Ok(balance.unwrap_or_default())
    }

    pub fn get_requests(deps: Deps) -> StdResult<Vec<Request>> {
        Ok(REQUESTS.load(deps.storage)?)
    }

    pub fn get_state(deps: Deps) -> StdResult<HexBinary> {
        Ok(STATE.load(deps.storage)?)
    }
}
