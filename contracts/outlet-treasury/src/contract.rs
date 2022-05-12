#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
                    StdError, Attribute, BankMsg, coin};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE, ReserveDepositors, RESERVE_DEPOSITORS, ReserveSpenders, RESERVE_SPENDERS};

use cw_controllers::Admin;
use cw20_base::state::{TokenInfo, TOKEN_INFO, MinterData};
use cw20_base::contract::{execute_burn, execute_mint, execute_transfer, query_balance, query_token_info};
use cw20_base::allowances::{execute_transfer_from, execute_increase_allowance, query_allowance};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:outlet-treasury";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

static TREASURY_ADMIN: &Admin = &Admin::new("treasury_admin");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    let admin_addr = deps.api.addr_validate(&msg.admin)?;
    TREASURY_ADMIN.set(deps.branch(), Some(admin_addr))?;

    let state = State{
        total_reserves: 0,
        total_debt: 0
    };
    STATE.save(deps.storage, &state)?; 

    let token_data = TokenInfo{
        name: "Phase".to_string(),
        symbol: "PHS".to_string(),
        decimals: 9,
        total_supply: Uint128::zero(),
        mint: Some(MinterData{
            minter: _env.contract.address,
            cap: None
        })
    };

    TOKEN_INFO.save(deps.storage, &token_data)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
    )
}

fn only_admin(deps: DepsMut, info: MessageInfo) -> Result< (),ContractError>{
    let res = TREASURY_ADMIN.assert_admin(deps.as_ref(), &info.sender);
    match res{
        Ok(()) => return Ok(()),
        Err(_not_admin) => return Err(ContractError::Unauthorized{})
    }; 
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRemoveDepositor{ address} => add_remove_depositor(deps, info, address ),
        ExecuteMsg::AddRemoveSpender{ address } => add_remove_spender(deps, info, address),
        ExecuteMsg::Deposit{ amount, profit} => deposit(deps, _env, info, amount, profit),
        ExecuteMsg::IncreaseAllowance{spender,amount} => Ok(execute_increase_allowance(deps, _env, info, spender, amount, None).unwrap()),
        ExecuteMsg::Transfer{recipient, amount} => Ok(execute_transfer(deps, _env, info, recipient, amount).unwrap()),
        ExecuteMsg::TransferFrom{owner, recipient, amount} => Ok(execute_transfer_from(deps, _env, info, owner, recipient, amount).unwrap()),
        ExecuteMsg::Withdraw{ recipient, amount } => withdraw(deps, _env, info, recipient, amount)

        // ExecuteMsg::Increment {} => try_increment(deps),
        // ExecuteMsg::Reset { count } => try_reset(deps, info, count),
    }
}

pub fn withdraw(mut deps: DepsMut, env: Env, info: MessageInfo, recipient: String, amount: u64) -> Result<Response, ContractError> {
    let rsd = RESERVE_SPENDERS.load(deps.storage)?;
    let sender = info.clone().sender;
    let value = amount * 10u64.pow(3);
    let is_spender = rsd.spenders.iter().any(|x| x== &sender.clone());

    if is_spender{
        let _res = execute_burn(deps.branch(), env, info.clone(), Uint128::from(value));
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError>{
            state.total_reserves = state.total_reserves - value;
            Ok(state)
        })?;
        let msg = BankMsg::Send{
            to_address: recipient,
            amount: vec![coin(amount as u128, "uusd")]
        };
        Ok(Response::new().add_message(msg))

    }else{
        return Err(ContractError::Unauthorized{});
    }
}

pub fn deposit(mut deps: DepsMut, env: Env, info: MessageInfo, amount: u64, profit: u64) -> Result<Response, ContractError>{
    let rds = RESERVE_DEPOSITORS.load(deps.storage)?;
    let is_depositor = rds.depositors.iter().any(|x| x == &info.clone().sender);
    
    if is_depositor{
        let send = amount - profit;
        let sub_info = MessageInfo{
            sender: env.contract.address.clone(),
            funds: vec![]
        };

        let _res = execute_mint(deps.branch(), env, sub_info, info.sender.to_string(), Uint128::from(send));
        
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError>{
            let reserve = state.total_reserves;
            state.total_reserves = reserve + amount;
            Ok(state)
        })?;

    }else{
        return Err(ContractError::Unauthorized{})
    }
    Ok(Response::new())
}

pub fn add_remove_depositor(mut deps: DepsMut, info: MessageInfo, address: String) -> Result<Response, ContractError> {
    only_admin(deps.branch(), info)?;

    let mut response = Response::new();
    
    let err_string = "outlet_treasury::state::ReserveDepositors".to_string();
    let addr = deps.api.addr_validate(&address)?;
    
    let mut res = RESERVE_DEPOSITORS.load(deps.storage);
    
    if res == Err(StdError::NotFound{ kind: err_string}) {
        res = Ok(ReserveDepositors{ depositors: vec![]});
    }

    let in_list = res.as_ref().unwrap().depositors.clone().iter().position(|x| x == &addr);


    if in_list == None {
        res.as_mut().unwrap().depositors.push(addr.clone());
        response.attributes.push(Attribute::new("added", address));
    }else{
        res.as_mut().unwrap().depositors.remove(in_list.unwrap());
        response.attributes.push(Attribute::new("removed", address));
    }

    RESERVE_DEPOSITORS.save(deps.storage, &res.as_ref().unwrap())?;

    Ok(response)
    
}

pub fn add_remove_spender(mut deps: DepsMut, info: MessageInfo, address: String) -> Result<Response, ContractError> {
    only_admin(deps.branch(), info)?;

    let mut response = Response::new();
    let err_string = "outlet_treasury::state::ReserveSpenders".to_string();
    let addr = deps.api.addr_validate(&address)?;
    
    let mut res = RESERVE_SPENDERS.load(deps.storage);
    
    if res == Err(StdError::NotFound{ kind: err_string}) {
        res = Ok(ReserveSpenders{ spenders: vec![]});
    }

    let in_list = res.as_ref().unwrap().spenders.clone().iter().position(|x| x == &addr);


    if in_list == None {
        res.as_mut().unwrap().spenders.push(addr.clone());
        response.attributes.push(Attribute::new("added", address));
    }else{
        res.as_mut().unwrap().spenders.remove(in_list.unwrap());
        response.attributes.push(Attribute::new("removed", address));
    }

    RESERVE_SPENDERS.save(deps.storage, &res.as_ref().unwrap())?;

    Ok(response)
    
}

// pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         state.count += 1;
//         Ok(state)
//     })?;

//     Ok(Response::new().add_attribute("method", "try_increment"))
// }
// pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         if info.sender != state.owner {
//             return Err(ContractError::Unauthorized {});
//         }
//         state.count = count;
//         Ok(state)
//     })?;
//     Ok(Response::new().add_attribute("method", "reset"))
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::TokenInfo{} => to_binary(&query_token_info(deps)?),
        QueryMsg::QueryAllowance{owner, spender} => to_binary(&query_allowance(deps, owner, spender)?),
        QueryMsg::Balance{address} => to_binary(&query_balance(deps, address)?)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let addr = "terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9";

        let msg = InstantiateMsg { admin: addr.to_string() };
        let info = mock_info(addr, &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let _res2 = add_remove_depositor(deps.as_mut(), info.clone(), addr.to_string());

        

        let _res3 = deposit(deps.as_mut(), mock_env(), info.clone(), 500, 50);
        
        let res4 = query(deps.as_ref(), mock_env(), QueryMsg::TokenInfo{}).unwrap();
        let val: TokenInfo = from_binary(&res4).unwrap();
        println!("{:#?}", val);
    }

    
}
