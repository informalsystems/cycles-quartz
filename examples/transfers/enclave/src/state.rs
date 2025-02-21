use std::collections::BTreeMap;

use cosmwasm_std::{Addr, Uint128};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct State {
    pub state: BTreeMap<Addr, Uint128>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Balance {
    pub balance: Uint128,
}
