use cosmwasm_std::HexBinary;
use cw_storage_plus::{Item, Map};

use crate::msg::execute::Request;

pub const STATE: Item<HexBinary> = Item::new("state");
pub const REQUESTS: Item<Vec<Request>> = Item::new("requests");
pub const DENOM: Item<String> = Item::new("donation_denom");
pub const BALANCES: Map<&str, HexBinary> = Map::new("balances");
