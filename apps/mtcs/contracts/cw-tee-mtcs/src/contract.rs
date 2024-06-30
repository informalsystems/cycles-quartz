use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw20_base::{
    contract::query_balance as cw20_query_balance,
    state::{MinterData, TokenInfo, TOKEN_INFO},
};
use quartz_cw::{handler::RawHandler, state::EPOCH_COUNTER};

use crate::{
    error::ContractError,
    msg::{
        execute::{
            Cw20Transfer, FaucetMintMsg, SubmitObligationMsg, SubmitObligationsMsg,
            SubmitSetoffsMsg,
        },
        ExecuteMsg, InstantiateMsg, QueryMsg,
    },
    state::{
        current_epoch_key, LiquiditySourcesItem, ObligationsItem, State, LIQUIDITY_SOURCES_KEY,
        OBLIGATIONS_KEY, STATE,
    },
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-tee-mtcs";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // must be the handled first!
    msg.0.handle_raw(deps.branch(), &env, &info)?;

    let state = State {
        owner: info.sender.to_string(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    EPOCH_COUNTER.save(deps.storage, &1)?;

    ObligationsItem::new(&current_epoch_key(OBLIGATIONS_KEY, deps.storage)?)
        .save(deps.storage, &Default::default())?;

    LiquiditySourcesItem::new(&current_epoch_key(LIQUIDITY_SOURCES_KEY, deps.storage)?)
        .save(deps.storage, &Default::default())?;

    // store token info using cw20-base format
    let data = TokenInfo {
        name: "USD".to_string(),
        symbol: "!$".to_string(),
        decimals: 0,
        total_supply: Uint128::zero(),
        // set self as minter, so we can properly execute mint and burn
        mint: Some(MinterData {
            minter: env.contract.address.clone(),
            cap: None,
        }),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

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
    match msg {
        ExecuteMsg::Quartz(msg) => msg.handle_raw(deps, &env, &info).map_err(Into::into),
        ExecuteMsg::FaucetMint(FaucetMintMsg { recipient, amount }) => {
            execute::faucet_mint(deps, env, recipient, amount)
        }
        ExecuteMsg::Transfer(Cw20Transfer { recipient, amount }) => Ok(
            cw20_base::contract::execute_transfer(deps, env, info, recipient, amount.into())?,
        ),
        ExecuteMsg::SubmitObligation(SubmitObligationMsg { ciphertext, digest }) => {
            execute::submit_obligation(deps, ciphertext, digest)
        }
        ExecuteMsg::SubmitObligations(SubmitObligationsMsg {
            obligations,
            liquidity_sources,
        }) => {
            for o in obligations {
                execute::submit_obligation(deps.branch(), o.ciphertext, o.digest)?;
            }
            execute::append_liquidity_sources(deps, liquidity_sources)?;
            Ok(Response::new())
        }
        ExecuteMsg::SubmitSetoffs(attested_msg) => {
            let _ = attested_msg
                .clone()
                .handle_raw(deps.branch(), &env, &info)?;
            let SubmitSetoffsMsg { setoffs_enc } = attested_msg.msg.0;
            execute::submit_setoffs(deps, env, setoffs_enc)
        }
        ExecuteMsg::InitClearing => execute::init_clearing(deps),
    }
}

pub mod execute {
    use std::collections::BTreeMap;

    use cosmwasm_std::{DepsMut, Env, HexBinary, MessageInfo, Response, StdResult};
    use cw20_base::contract::{execute_burn, execute_mint};
    use k256::ecdsa::VerifyingKey;
    use quartz_cw::state::{Hash, EPOCH_COUNTER};

    use crate::{
        state::{
            current_epoch_key, previous_epoch_key, LiquiditySourcesItem, ObligationsItem, RawHash,
            SetoffsItem, SettleOff, LIQUIDITY_SOURCES_KEY, OBLIGATIONS_KEY, SETOFFS_KEY,
        },
        ContractError,
    };

    pub fn faucet_mint(
        mut deps: DepsMut,
        env: Env,
        recipient: String,
        amount: u64,
    ) -> Result<Response, ContractError> {
        let info = MessageInfo {
            sender: env.contract.address.clone(),
            funds: vec![],
        };

        execute_mint(
            deps.branch(),
            env.clone(),
            info.clone(),
            recipient.to_string(),
            amount.into(),
        )?;

        Ok(Response::new().add_attribute("action", "faucet_mint"))
    }

    pub fn submit_obligation(
        deps: DepsMut,
        ciphertext: HexBinary,
        digest: HexBinary,
    ) -> Result<Response, ContractError> {
        let _: Hash = digest.to_array()?;

        // store the `(digest, ciphertext)` tuple
        ObligationsItem::new(&current_epoch_key(OBLIGATIONS_KEY, deps.storage)?).update(
            deps.storage,
            |mut obligations| {
                if let Some(_duplicate) = obligations.insert(digest.clone(), ciphertext.clone()) {
                    return Err(ContractError::DuplicateEntry);
                }
                Ok(obligations)
            },
        )?;

        Ok(Response::new()
            .add_attribute("action", "submit_obligation")
            .add_attribute("digest", digest.to_string())
            .add_attribute("ciphertext", ciphertext.to_string()))
    }

    pub fn append_liquidity_sources(
        deps: DepsMut,
        liquidity_sources: Vec<HexBinary>,
    ) -> Result<(), ContractError> {
        // validate liquidity sources as public keys
        liquidity_sources
            .iter()
            .try_for_each(|ls| VerifyingKey::from_sec1_bytes(ls).map(|_| ()))?;

        // store the liquidity sources
        LiquiditySourcesItem::new(&current_epoch_key(LIQUIDITY_SOURCES_KEY, deps.storage)?)
            .update(deps.storage, |mut ls| {
                ls.extend(liquidity_sources);
                Ok::<_, ContractError>(ls)
            })?;

        Ok(())
    }

    pub fn submit_setoffs(
        mut deps: DepsMut,
        env: Env,
        setoffs_enc: BTreeMap<RawHash, SettleOff>,
    ) -> Result<Response, ContractError> {
        // store the `BTreeMap<RawHash, RawCipherText>`
        SetoffsItem::new(&previous_epoch_key(SETOFFS_KEY, deps.storage)?)
            .save(deps.storage, &setoffs_enc)?;

        for (_, so) in setoffs_enc {
            if let SettleOff::Transfer(t) = so {
                let info = MessageInfo {
                    sender: env.contract.address.clone(),
                    funds: vec![],
                };

                execute_mint(
                    deps.branch(),
                    env.clone(),
                    info.clone(),
                    t.payee.to_string(),
                    t.amount.into(),
                )?;

                let payer = deps.api.addr_validate(&t.payer.to_string())?;
                let info = MessageInfo {
                    sender: payer,
                    funds: vec![],
                };

                execute_burn(deps.branch(), env.clone(), info, t.amount.into())?;
            }
        }

        Ok(Response::new().add_attribute("action", "submit_setoffs"))
    }

    pub fn init_clearing(deps: DepsMut) -> Result<Response, ContractError> {
        EPOCH_COUNTER.update(deps.storage, |mut counter| -> StdResult<_> {
            counter += 1;
            Ok(counter)
        })?;
        Ok(Response::new().add_attribute("action", "init_clearing"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllSetoffs => to_json_binary(&query::get_all_setoffs(deps)?),
        QueryMsg::GetLiquiditySources { epoch } => {
            to_json_binary(&query::get_liquidity_sources(deps, epoch)?)
        }
        QueryMsg::Balance { address } => to_json_binary(&cw20_query_balance(deps, address)?),
    }
}

pub mod query {
    use cosmwasm_std::{Deps, StdResult};

    use crate::{
        msg::{GetAllSetoffsResponse, GetLiquiditySourcesResponse},
        state::{
            current_epoch_key, epoch_key, previous_epoch_key, LiquiditySourcesItem, SetoffsItem,
            LIQUIDITY_SOURCES_KEY, SETOFFS_KEY,
        },
    };

    pub fn get_all_setoffs(deps: Deps) -> StdResult<GetAllSetoffsResponse> {
        let setoffs = SetoffsItem::new(&previous_epoch_key(SETOFFS_KEY, deps.storage)?)
            .load(deps.storage)?
            .into_iter()
            .collect();
        Ok(GetAllSetoffsResponse { setoffs })
    }

    pub fn get_liquidity_sources(
        deps: Deps,
        epoch: Option<usize>,
    ) -> StdResult<GetLiquiditySourcesResponse> {
        let epoch_key = match epoch {
            None => current_epoch_key(LIQUIDITY_SOURCES_KEY, deps.storage)?,
            Some(e) => epoch_key(LIQUIDITY_SOURCES_KEY, e)?,
        };

        let liquidity_sources = LiquiditySourcesItem::new(&epoch_key)
            .load(deps.storage)?
            .into_iter()
            .collect();
        Ok(GetLiquiditySourcesResponse { liquidity_sources })
    }
}
