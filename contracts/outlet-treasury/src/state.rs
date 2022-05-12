use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_reserves: u64,
    pub total_debt: u64
}

pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveDepositors {
    pub depositors: Vec<Addr>
}
pub const RESERVE_DEPOSITORS: Item<ReserveDepositors> = Item::new("reserve_depositors");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReserveSpenders {
    pub spenders: Vec<Addr>
}
pub const RESERVE_SPENDERS: Item<ReserveSpenders> = Item::new("reserve_spenders");


