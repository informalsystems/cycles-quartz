use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response,
    StdResult,
};
use quartz_contract_core::handler::RawHandler;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
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

    Ok(Response::new()
        .add_event(Event::new("pingpong"))
        .add_attribute("action", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Quartz msgs
        ExecuteMsg::Quartz(msg) => msg.handle_raw(deps, &env, &info).map_err(Into::into),

        // User messages
        ExecuteMsg::Ping(ping) => execute::ping(deps, env, info, ping),
        ExecuteMsg::Pong(attested_msg) => {
            let _ = attested_msg
                .clone()
                .handle_raw(deps.branch(), &env, &info)?;

            execute::pong(deps, env, info, attested_msg.msg.0)
        }
    }
}

pub mod execute {
    use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Response};
    use serde_json::json;

    use crate::{
        error::ContractError,
        msg::execute::{Ping, Pong},
        state::PINGS,
    };

    pub fn ping(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        ping: Ping,
    ) -> Result<Response, ContractError> {
        // Instantiate map entry for ping
        PINGS.save(deps.storage, ping.pubkey.to_hex(), &ping.message)?;

        Ok(Response::new()
            .add_event(Event::new("pingpong"))
            .add_attribute("action", "ping")
            .add_attribute("ping_data", json!(ping).to_string()))
    }

    pub fn pong(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        pong: Pong,
    ) -> Result<Response, ContractError> {
        // Overwrite entry with key `ping.pubkey` with the enclave's response, encrypted to the pubkey
        PINGS.save(deps.storage, pong.pubkey.to_hex(), &pong.response)?;

        Ok(Response::new()
            .add_event(Event::new("pingpong"))
            .add_attribute("action", "pong")
            .add_attribute("response", pong.response.to_hex()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllMessages {} => to_json_binary(&query::get_all_messages(deps)?),
    }
}

mod query {
    use std::collections::BTreeMap;

    use cosmwasm_std::{Deps, HexBinary, StdResult};

    use crate::state::PINGS;

    pub fn get_all_messages(deps: Deps) -> StdResult<BTreeMap<String, HexBinary>> {
        let pings = PINGS
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<BTreeMap<_, _>>>();

        pings
    }
}
