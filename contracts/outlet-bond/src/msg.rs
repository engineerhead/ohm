use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: String,
    pub treasury: String,
    pub dao: String,
    pub staking: String,
    pub total_debt: u64,
    pub last_decay: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Init{
        control_variable: u64,
        vesting_term: u64,
        minimum_price: u64,
        max_payout: u64,
        fee: u64,
        max_debt: u64,
        initial_debt: u64
    },
    SetStaking{
        staking: String
    },
    Deposit {
        max_price: u64
    },
    SetAdjustment {
        addition: bool, 
        increment: u64, 
        target: u64, 
        buffer: u64 
    },
    Redeem {
        stake: bool
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    BondInfo{address: String},
    BondPriceInUsd{},
    MaxPayout{address: String, max_pay: u64},
    PendingPayout{address: String}
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}
