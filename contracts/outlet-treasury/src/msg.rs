use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
   pub admin: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddRemoveDepositor{
        address: String
    },
    AddRemoveSpender{
        address: String
    },
    Deposit{
        amount: u64,
        profit: u64
    },
    Transfer{
        recipient: String,
        amount: Uint128
    }, 
    TransferFrom{
        owner: String,
        recipient: String,
        amount: Uint128
    },
    IncreaseAllowance{
        spender: String,
        amount: Uint128
    },
    Withdraw{
        recipient: String,
        amount: u64
    }

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TokenInfo{},
    Balance{ 
        address: String
    },
    QueryAllowance{
        owner: String,
        spender: String
    }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: i32,
}
