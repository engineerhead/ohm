use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub treasury: Addr,
    pub dao: Addr,
    pub staking: Addr,
    pub total_debt: u64,
    pub last_decay: u64
}

pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Terms {
    pub control_variable: u64,
    pub vesting_term: u64,
    pub minimum_price: u64,
    pub max_payout: u64,
    pub fee: u64,
    pub max_debt: u64
}

pub const TERMS: Item<Terms> = Item::new("terms");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bond {
    pub payout: u64,
    pub vesting: u64,
    pub last_block: u64,
    pub price_paid: u64
}
pub const BOND: Map<&Addr, Bond> = Map::new("bond");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Adjust {
    pub add: bool,
    pub rate: u64,
    pub target: u64,
    pub buffer: u64,
    pub last_block: u64
}

pub const ADJUST: Item<Adjust> = Item::new("adjust");