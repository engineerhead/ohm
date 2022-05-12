#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
                    QueryRequest, WasmQuery, Uint128, Coin, Addr, SubMsg, CosmosMsg, WasmMsg};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE, Terms, TERMS, Bond, BOND, Adjust, ADJUST};

use cw_controllers::Admin;
use cw20::{TokenInfoResponse, Cw20QueryMsg };
use terra_cosmwasm::TerraQuerier;
use cosmwasm_bignumber::{Uint256, Decimal256};
use outlet_treasury::msg::{ExecuteMsg as TreasuryExecuteMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:outlet-bond";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
static BOND_ADMIN: &Admin = &Admin::new("bond_admin");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin_addr = deps.api.addr_validate(&msg.admin)?;
    BOND_ADMIN.set(deps.branch(), Some(admin_addr))?;

    let state = State {
        treasury: deps.api.addr_validate(&msg.treasury)?,
        dao: deps.api.addr_validate(&msg.dao)?,
        staking: deps.api.addr_validate(&msg.staking)?,
        total_debt: 0,
        last_decay: 0
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Init { control_variable, vesting_term, minimum_price, max_payout,
                            fee, max_debt, initial_debt} =>
                            initialize_bond_terms(deps, env, info, control_variable,
                                                    vesting_term, minimum_price, max_payout, fee, max_debt, initial_debt),
        ExecuteMsg::SetStaking {staking} => set_staking(deps, info, staking),
        ExecuteMsg::Deposit{max_price} => deposit(deps, env, info, max_price),
        ExecuteMsg::SetAdjustment{ addition, increment, target, buffer } => set_adjustment( deps, info, env,addition, increment, target, buffer  ),
        ExecuteMsg::Redeem {stake} => redeem( deps, env, info, stake )
        // ExecuteMsg::Increment {} => try_increment(deps),
        // ExecuteMsg::Reset { count } => try_reset(deps, info, count),
    }
}

pub fn redeem(deps: DepsMut, env: Env, info: MessageInfo, stake: bool) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let treasury_address = String::from(state.treasury);
    let recipient = info.clone().sender;
    let bond = bond_info(deps.as_ref(), recipient.clone().to_string())?;
    let percent_vested = percent_vested_for(deps.as_ref(), env.clone(), recipient.clone().to_string());

    if percent_vested >= 10000u64 {
        BOND.remove(deps.storage, &recipient);
        return stake_or_send(
            info.clone(), 
            state.staking.to_string(), 
            treasury_address, 
            recipient,
            stake,
            bond.payout
        )
    }
    else{
        let payout = bond.payout * percent_vested / 10000;
        let bond_info_to_save = Bond{
            payout: bond.payout - payout,
            vesting: bond.vesting - (env.clone().block.height - bond.last_block),
            last_block: env.clone().block.height,
            price_paid: bond.price_paid
        };
        BOND.save(deps.storage, &recipient, &bond_info_to_save)?;
        return stake_or_send(
            info.clone(), 
            state.staking.to_string(), 
            treasury_address, 
            recipient, 
            stake, 
            payout
        );
    }
}

fn stake_or_send( 
    info: MessageInfo, 
    staking_address: String, 
    treasury_address: String, 
    recipient: Addr, 
    stake: bool, 
    payout: u64) -> Result<Response, ContractError>{

    let mut response = Response::new();


    // response
    // .messages
    // .push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
    //     funds: vec![],
    //     contract_addr: treasury_address,
    //     msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //         recipient: recipient.to_string(),
    //         amount: Uint128::from(payout)
    //     })?,
    // })));

    // if stake{
    //     response    
    //     .messages
    //     .push(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
    //         funds: vec![],
    //         contract_addr: staking_address.clone(),
    //         msg: to_binary(&StakingExecuteMsg::Stake {
    //             owner: info.sender.to_string(),
    //             recipient: staking_address,
    //             amount: payout
    //         })?,
    //     }))); 

    // }
  
    Ok(response)
}



pub fn set_adjustment(
    mut deps: DepsMut, 
    info: MessageInfo, 
    env: Env, 
    addition: bool, 
    increment: u64, 
    target: u64, 
    buffer: u64 
) -> Result<Response, ContractError> {
    only_admin(deps.branch(), info)?;
    let terms = TERMS.load(deps.branch().storage)?;
    let compare = terms.control_variable * 25 / 1000;
    if increment > compare {
        return Err(ContractError::LargeIncrement{})
    }
    ADJUST.update(deps.storage, |mut adjust| -> StdResult<_>{
        adjust.add = addition;
        adjust.rate = increment;
        adjust.target = target;
        adjust.buffer = buffer;
        adjust.last_block = env.block.height;
        Ok(adjust)
    })?;
    Ok(Response::default())
}

pub fn deposit(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_price: u64
) ->Result<Response, ContractError>{
    
    decay_debt(deps.branch(), env.clone());

    let mut state = STATE.load(deps.storage)?;
    let terms = TERMS.load(deps.storage)?;

    if state.total_debt >= terms.max_debt{
        return Err(ContractError:: MaxCapacity{})
    }

    let price_in_usd = bond_price_in_usd(deps.as_ref(), env.clone())?;
    let native_price = _bond_price(deps.branch(), env.clone())?;

    if max_price <= native_price {
        return Err(ContractError::SlippageLimit{})
    }
    
    let deposit_amount = info
                            .funds
                            .iter()
                            .find(|c| c.denom == "uusd")
                            .map(|c| c.amount)
                            .unwrap_or_else(Uint128::zero);

    let value = deposit_amount.u128() as u64 * 10u64.pow(3);

    let payout = payout_for(deps.branch(), env.clone(), value)? * 100;

    if payout <= 10000000u64 {
        return Err(ContractError::SmallBond{})
    }

    let max_payout = max_payout(deps.as_ref(), String::from(state.clone().treasury), terms.max_payout)?;

    if payout >= max_payout{
        return Err(ContractError::LargeBond{})
    }

    let fee = payout * terms.fee / 100000u64;
    let profit = value - payout - fee;

    state.total_debt = state.total_debt + value;
    STATE.save(deps.branch().storage, &state)?;

    let depositor = info.sender;
    let bond_info = bond_info(deps.as_ref(), depositor.clone().to_string())?;
    let bond_info_to_save = Bond{
        payout: bond_info.payout + payout,
        vesting: terms.vesting_term,
        last_block: env.block.height,
        price_paid: price_in_usd
    };
    BOND.save(deps.branch().storage, &depositor, &bond_info_to_save)?;

    adjust(deps.branch(), env.clone());

    //let coin = deduct_tax(deps.as_ref(), Coin::new(deposit_amount.u128(),"uusd"))?;
    let coin = Coin::new(deposit_amount.u128(), "uusd");
    

    Ok(Response::new()
                    .add_submessage(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute{
                        funds: vec![coin],
                        contract_addr: String::from(state.clone().treasury),
                        msg: to_binary(&TreasuryExecuteMsg::Deposit{
                            amount: value,
                            profit
                        })?
                    })))
    )
    //TODO: Send to treasury 
}

fn adjust(deps: DepsMut, env: Env){
    let adjust_info = ADJUST.load(deps.storage);
    let mut adjust = match adjust_info {
        Ok(_) => adjust_info.unwrap(),
        Err(_) => Adjust{add: false, rate:0, target: 0, buffer:0, last_block:0 }
    };
    if adjust.last_block != 0 {
        let block_can_adjust = adjust.last_block + adjust.buffer;
        if adjust.rate != 0 && env.block.height >= block_can_adjust{
            let mut terms = TERMS.load(deps.storage).unwrap();
            let initial = terms.control_variable;
            if adjust.add {
                terms.control_variable = initial + adjust.target;
                if terms.control_variable >= adjust.target {
                    adjust.rate = 0;
                }
            }else{
                terms.control_variable = initial - adjust.target;
                if terms.control_variable <= adjust.target {
                    adjust.rate = 0;
                } 
            }
            adjust.last_block = env.block.height;
            let _terms = TERMS.save(deps.storage, &terms);
            let _adjust = ADJUST.save(deps.storage, &adjust);
        }
    }

}

pub fn compute_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(amount * tax_rate, tax_cap))
}

// https://github.com/Anchor-Protocol/money-market-contracts/blob/230ccf7f41fb04fff66536c48a9d397225813544/packages/moneymarket/src/querier.rs#L70
fn compute_deducted_tax(deps: Deps, coin: &Coin) -> StdResult<Uint256> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    let tax_rate = Decimal256::from((terra_querier.query_tax_rate()?).rate);
    let tax_cap = Uint256::from((terra_querier.query_tax_cap(coin.denom.to_string())?).cap);
    let amount = Uint256::from(coin.amount);
    Ok(std::cmp::min(
        amount * Decimal256::one() - amount / (Decimal256::one() + tax_rate),
        tax_cap,
    ))
}

pub fn deduct_tax(deps: Deps, coin: Coin) -> StdResult<Coin> {
    let tax_amount = compute_deducted_tax(deps, &coin)?;
    Ok(Coin {
        denom: coin.denom,
        amount: (Uint256::from(coin.amount) - tax_amount).into(),
    })
}

fn max_payout(deps: Deps, addr: String, max_pay: u64) -> StdResult<u64>{
    let total_supply = total_supply(deps, addr)?;
    Ok(total_supply * max_pay / 100u64)
}

fn payout_for(deps: DepsMut, env: Env, value: u64) -> StdResult<u64> {
    Ok( value / bond_price(deps.as_ref(), env)?)
}

fn _bond_price(deps: DepsMut, env: Env) -> StdResult<u64>{
    let state = STATE.load(deps.storage)?;
    let mut terms = TERMS.load(deps.storage)?;
    let debt_ratio = debt_ratio(deps.as_ref(), env, String::from(state.treasury))?;
    let mut price = (terms.control_variable *  debt_ratio + 1000000000u64) / 10u64.pow(7);
    if price < terms.minimum_price {
        price = terms.minimum_price;
    }else if terms.minimum_price != 0 {
        terms.minimum_price = 500; //TODO: Figure out minimum price
    }
    let _res = TERMS.save(deps.storage, &terms);
    Ok(price)
}

fn bond_price_in_usd(deps: Deps, env: Env) -> StdResult<u64> {
    Ok(bond_price(deps,env)? * 10u64.pow(6) / 100)
}

fn bond_price(deps: Deps, env: Env) -> StdResult<u64> {
    let state = STATE.load(deps.storage)?;
    let terms = TERMS.load(deps.storage)?;
    let debt_ratio = debt_ratio(deps, env, String::from(state.treasury))?;
    
    let mut price = (terms.control_variable * debt_ratio + 1000000000u64) / 10u64.pow(7);
    
    if price < terms.minimum_price {
        price = terms.minimum_price;
    }
    Ok(price)
}

fn debt_ratio(deps: Deps, env: Env, addr:String) -> StdResult<u64> {
    let total_supply = total_supply(deps, addr)?;
    let current_debt = current_debt(deps, env)?;
    Ok(current_debt * 10u64.pow(9) / total_supply)
}

fn current_debt(deps: Deps, env: Env) -> StdResult<u64>{
    let state = STATE.load(deps.storage)?;
    Ok(state.total_debt - debt_decay(deps, env)?)
}

fn total_supply(deps: Deps, addr: String) -> StdResult<u64> {
    // let res: TokenInfoResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
    //     contract_addr: addr,
    //     msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    // }))?;
    // Ok(res.total_supply.u128() as u64)
    Ok(60000000000)
}

fn decay_debt(deps: DepsMut, env: Env){
    let mut state = STATE.load(deps.storage).unwrap();
    state.total_debt = state.total_debt - debt_decay(deps.as_ref(), env.clone()).unwrap();
    state.last_decay = env.block.height;
    let _res = STATE.save(deps.storage, &state);
}

fn debt_decay(deps: Deps, env: Env) -> StdResult<u64> {
    let state = STATE.load(deps.storage)?;
    let last_decay = state.last_decay;
    let height_since_last = env.block.height - last_decay;
    let total_debt = state.total_debt;
    let mut decay = total_debt *height_since_last;
    if decay > total_debt {
        decay = total_debt;
    }
    Ok(decay)
}

pub fn set_staking(
    mut deps: DepsMut,
    info: MessageInfo,
    staking: String
) -> Result<Response, ContractError>{
    only_admin(deps.branch(), info)?;
    let mut state = STATE.load(deps.storage)?;
    state.staking =  deps.api.addr_validate(&staking)?;
    STATE.save(deps.storage, &state)?; 
    Ok(Response::default())
}

pub fn initialize_bond_terms(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    control_variable: u64,
    vesting_term: u64,
    minimum_price: u64,
    max_payout: u64,
    fee: u64,
    max_debt: u64,
    initial_debt: u64
) -> Result<Response, ContractError> {
    only_admin(deps.branch(), info)?;

    let terms_data = Terms{
        control_variable,
        vesting_term,
        minimum_price,
        max_payout,
        fee,
        max_debt
    };    
    TERMS.save(deps.storage, &terms_data)?;

    STATE.update(deps.storage, |mut state| -> StdResult<_>{
        state.total_debt = initial_debt;
        state.last_decay = env.block.height;
        Ok(state)
    })?;

    Ok(Response::default())
}

fn only_admin(deps: DepsMut, info: MessageInfo) -> Result< (),ContractError>{
    let res = BOND_ADMIN.assert_admin(deps.as_ref(), &info.sender);
    match res{
        Ok(()) => return Ok(()),
        Err(_not_admin) => return Err(ContractError::Unauthorized{})
    }; 
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::BondInfo{address} => to_binary(&bond_info(deps, address)?),
        QueryMsg::BondPriceInUsd{} => to_binary(&bond_price_in_usd(deps, _env)?),
        QueryMsg::MaxPayout{address, max_pay} => to_binary(&max_payout(deps, address, max_pay)?),
        QueryMsg::PendingPayout{address} => to_binary(&pending_payout(deps, _env, address)?)
    
    }
}

fn bond_info(deps: Deps, depositor: String) -> StdResult<Bond> {
    let mut bond_info = BOND.load(deps.storage, &deps.api.addr_validate(&depositor).unwrap());
    match bond_info.as_mut(){
        Ok(_bond) => Ok(bond_info.unwrap()),
        Err(_) => return Ok(Bond{payout: 0,vesting: 0, last_block:0, price_paid:0 })
    }
}

pub fn percent_vested_for(deps: Deps, env: Env, depositor: String) -> u64 {
    let bond = bond_info(deps, depositor).unwrap();
    let blocks_since_last = env.block.height - bond.last_block;
    let vesting = bond.vesting;
    return blocks_since_last * 10000 / vesting;
    
}

pub fn pending_payout(deps: Deps, env: Env, depositor: String) -> StdResult<u64>{
    let percent_vested = percent_vested_for(deps, env, depositor.clone());
    let payout = bond_info(deps, depositor).unwrap().payout;
    let pending_payout: u64;
    if percent_vested >= 10000{
        pending_payout = payout;
    }else{
        pending_payout = payout * percent_vested / 10000;
    }
    Ok (pending_payout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    fn init_msg() -> InstantiateMsg {
        let addr_string = "terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8".to_string();
        
        // let addr = Addr::unchecked(addr_string);
        return InstantiateMsg {
            admin: addr_string.clone(),
            treasury: addr_string.clone(),
            dao: addr_string.clone(),
            staking: addr_string.clone(),
            total_debt: 0,
            last_decay: 0,
       }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let msg = init_msg();
       
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(1000, "uusd"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn minimum_price() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(2000000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 369,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt: 0

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);

        //While initial debt is 0, price should be minimum price
        assert_eq!(minimum_price, bond_price(deps.as_ref(), mock_env()).unwrap());

        assert_eq!(minimum_price * 10000, bond_price_in_usd(deps.as_ref(), mock_env()).unwrap());
        
    }

    #[test]
    fn normal_price() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(2000000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;
        let initial_debt = 100000000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 300000 ,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);
        
        // As block height is staying same in testing env, initial debt should not decay
        assert_eq!(initial_debt, current_debt(deps.as_ref(), mock_env()).unwrap());
       
        //Further check debt ratio and price accroding to formula
        assert_eq!(1666666, debt_ratio(deps.as_ref(), mock_env(), "terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8".to_string()).unwrap());
        // assert_eq!()
        assert_eq!(50099, bond_price(deps.as_ref(), mock_env()).unwrap());
        
        
    }

    #[test]
    fn slippage() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(2000000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;
        let initial_debt = 100000000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 300000 ,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);
        
        let deposit_msg = ExecuteMsg::Deposit{
            max_price: 50
        };

        let res2 = execute(deps.as_mut(), mock_env(), info.clone(), deposit_msg); 
        match res2 {
            Err(ContractError::SlippageLimit{}) => {},
            _ => panic!("Must return slippage limit error")
        }
        
        
    }

    #[test]
    fn small_bond() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(20000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;
        let initial_debt = 100000000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 300000 ,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);
        
        let deposit_msg = ExecuteMsg::Deposit{
            max_price: 500000
        };

        let res2 = execute(deps.as_mut(), mock_env(), info.clone(), deposit_msg); 
        
        match res2 {
            Err(ContractError::SmallBond{}) => {},
            _ => panic!("Must return small bond error")
        }
        
        
    }

    #[test]
    fn large_bond() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(20000000000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;
        let initial_debt = 100000000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 300000 ,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);
        
        let deposit_msg = ExecuteMsg::Deposit{
            max_price: 500000
        };

        let res2 = execute(deps.as_mut(), mock_env(), info.clone(), deposit_msg); 
        
        match res2 {
            Err(ContractError::LargeBond{}) => {},
            _ => panic!("Must return large bonds error")
        }
        
        
    }

    #[test]
    fn payout() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let msg = init_msg();
        let info = mock_info("terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8", &coins(20000000, "uusd"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg);

        let minimum_price = 10000;
        let initial_debt = 100000000;

        let terms_msg = ExecuteMsg::Init {
            control_variable: 300000 ,
            vesting_term: 28800,
            minimum_price,
            max_payout: 50,
            fee: 1000,
            max_debt: 1000000000000000,
            initial_debt

        };

        
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), terms_msg);
        
        let deposit_msg = ExecuteMsg::Deposit{
            max_price: 500000
        };

        let _res2 = execute(deps.as_mut(), mock_env(), info.clone(), deposit_msg); 
        
        let res3 = query(deps.as_ref(), mock_env(), QueryMsg::BondInfo{address:"terra1dcegyrekltswvyy0xy69ydgxn9x8x32zdtapd8".to_string() }).unwrap();
        
        let value: Bond = from_binary(&res3).unwrap();
        assert_eq!(39920900, value.payout);
    }
}
