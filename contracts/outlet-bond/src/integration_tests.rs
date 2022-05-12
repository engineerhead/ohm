#![cfg(test)]

use cosmwasm_std::{Empty, Addr, Uint128, WasmMsg, to_binary, Coin};

use terra_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

use crate::contract::{execute, instantiate, query};
use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg};

fn mock_app() -> App {
    AppBuilder::new().build()    
}

pub fn bond_contract() -> Box<dyn Contract<Empty>>{
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn treasury_contract() -> Box<dyn Contract<Empty>>{
    let contract = ContractWrapper::new(
        outlet_treasury::contract::execute, 
        outlet_treasury::contract::instantiate, 
        outlet_treasury::contract::query
    );
    Box::new(contract)
}

#[test]
fn transfer_ust(){
    let mut router = mock_app();

    let init_funds = vec![Coin::new(5000u128 * 10u128.pow(6), "uusd")];
    
    let bond_id = router.store_code(bond_contract());
    let treasury_id = router.store_code(treasury_contract());

    let addr_string = "terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8".to_string();

    router
        .init_bank_balance(&Addr::unchecked(addr_string.clone()), init_funds)
        .unwrap();

    let treasury_init_msg = outlet_treasury::msg::InstantiateMsg{
        admin: addr_string.clone()
    };

    let treasury_addr = router.instantiate_contract(
                                                treasury_id, 
                                                Addr::unchecked(addr_string.clone()), 
                                                &treasury_init_msg, 
                                                &[], 
                                                "Treasury", 
                                                None)
                                                .unwrap();


    let bond_inst_msg = InstantiateMsg{
        admin: addr_string.clone(),
        treasury: treasury_addr.clone().into(),
        dao: addr_string.clone(),
        staking: addr_string.clone(),
        total_debt: 0,
        last_decay: 0,
    };

    let bond_addr = router.instantiate_contract(
                                                bond_id, 
                                                Addr::unchecked(addr_string.clone()), 
                                                &bond_inst_msg, 
                                                &vec![], 
                                                "Bond", 
                                                None)
                                                .unwrap();

    let terms_msg = ExecuteMsg::Init {
            control_variable: 369,
            vesting_term: 28800,
            minimum_price: 10000,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt: 0

    };

    let add_depositor_msg = outlet_treasury::msg::ExecuteMsg::AddRemoveDepositor{
        address: bond_addr.clone().into()
    };

    router
        .execute_contract(Addr::unchecked(addr_string.clone()), treasury_addr.clone(), &add_depositor_msg,&[])
        .unwrap();

    let deposit_msg = ExecuteMsg::Deposit{
        max_price: 50000
    };

    router
        .execute_contract(Addr::unchecked(addr_string.clone()), bond_addr.clone(), &terms_msg, &[])
        .unwrap();

    router
        .execute_contract(
            Addr::unchecked(addr_string.clone()), 
            bond_addr.clone(), 
            &deposit_msg, 
            &vec![Coin::new(1000u128 * 10u128.pow(6), "uusd")]
        )
        .unwrap();

        let token_info_msg = outlet_treasury::msg::QueryMsg::TokenInfo{};

        let val: cw20::TokenInfoResponse = router
            .wrap()
            .query_wasm_smart(&treasury_addr, &token_info_msg)
            .unwrap();
        assert_eq!(10, val.total_supply.u128() / 10u128.pow(9));

}