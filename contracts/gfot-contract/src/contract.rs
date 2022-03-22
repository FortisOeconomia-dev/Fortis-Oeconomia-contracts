#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest, CosmosMsg, SubMsg, Addr
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ReceiveMsg};
use cw20::{TokenInfoResponse};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ExpectedAmountResponse, 
};
use crate::state::{
    Config, CONFIG
};

use cw20_base::{
    msg::ExecuteMsg as Cw20ExecuteMsg, msg::InstantiateMsg as Cw20InstantiateMsg, 
    msg::QueryMsg as Cw20QueryMsg
};

use integer_sqrt::IntegerSquareRoot;

// Version info, for migration info
const CONTRACT_NAME: &str = "bfotburn";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const BFOT_START_AMOUNT:u128 = 100_000_000_000_000u128;
const STEP_AMOUNT:u128 = 10_000_000_000u128;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg
        .owner
        .map_or(Ok(info.sender), |o| deps.api.addr_validate(&o))?;

    let config = Config {
        owner: Some(owner),
        bfot_token_address: msg.bfot_token_address.clone(),
        gfot_token_address: msg.bfot_token_address.clone(),
        bfot_burn_amount: Uint128::zero(),
        gfot_sent_amount: Uint128::zero(),
        rate: Uint128::from(BFOT_START_AMOUNT),
        left: Uint128::from(BFOT_START_AMOUNT)
    };
    CONFIG.save(deps.storage, &config)?;
    //testnet 397 mainnet 9
    

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_owner , gfot_token_address} 
            => execute_update_config(deps, info, new_owner, gfot_token_address),
        ExecuteMsg::Receive(msg) => try_receive(deps, info, msg),
    }
}

pub fn try_receive(
    deps: DepsMut, 
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, ContractError> {
    
    let mut cfg = CONFIG.load(deps.storage)?;
    // let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    // let balance = Balance::Cw20(Cw20CoinVerified {
    //     address: info.sender,
    //     amount: wrapper.amount,
    // });

    // match msg {
    //     ReceiveMsg::Fot {} => {
    //         execute_fot(deps, balance, &deps.api.addr_validate(&wrapper.sender)?);
    //     },
    //     ReceiveMsg::Bfot {} => {
    //         execute_bfot(deps, balance);
    //     }
    // }

    // let fot_token_info: TokenInfoResponse =
    //     deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
    //         contract_addr: cfg.bfot_token_address.clone().into(),
    //         msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    //     }))?;

    // let gfot_left_amount = Uint128::from(fot_token_info.total_supply);
    let user_addr = &deps.api.addr_validate(&wrapper.sender)?;


    if info.sender == cfg.bfot_token_address {

        // input bfot amount
        let mut bfot_received_amount = wrapper.amount;
        let mut gfot_send_amount = Uint128::zero();
        let mut bfot_burn_amount = Uint128::zero();

        while bfot_received_amount > Uint128::zero() {
            
            if cfg.left < bfot_received_amount {
                //calculate the first slice 
                bfot_received_amount -= cfg.left;
                bfot_burn_amount += cfg.left;
                gfot_send_amount += cfg.left * Uint128::from(STEP_AMOUNT) / cfg.rate;
                
                cfg.rate += Uint128::from(STEP_AMOUNT);
                cfg.left = cfg.rate;

                if cfg.left >= bfot_received_amount {
                    continue;
                }
                
                //now use the quadratic equation
                let a = bfot_received_amount.u128() / STEP_AMOUNT;
                let b = cfg.rate.u128() / STEP_AMOUNT;
                let mut n = (4 * b * b - 4 * b + 1 + 8 * a).integer_sqrt();
                n = n - 2 * b + 1;
                n = n / 2;
                
                // This n is the floor value, so use this
                cfg.rate += Uint128::from(n).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
                cfg.left = cfg.rate;

                let bfot_sum = n * b + n * (n - 1) / 2;
                let bfot_amount = Uint128::from(bfot_sum).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
                bfot_burn_amount += bfot_amount;
                gfot_send_amount += Uint128::from(n).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
                bfot_received_amount -= bfot_amount;

            } else {
                cfg.left -= bfot_received_amount;
                bfot_burn_amount += bfot_received_amount;
                gfot_send_amount += bfot_received_amount * Uint128::from(STEP_AMOUNT) / cfg.rate;
                bfot_received_amount = Uint128::zero();
            }
        }
        
        cfg.gfot_sent_amount += gfot_send_amount;
        cfg.bfot_burn_amount += bfot_burn_amount;
        
        CONFIG.save(deps.storage, &cfg)?;
        
        let mut messages:Vec<CosmosMsg> = vec![];
        if gfot_send_amount > Uint128::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.gfot_token_address.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: user_addr.clone().into(),
                    amount: gfot_send_amount,
                })?,
            }));
        }

        if bfot_burn_amount > Uint128::zero() {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.bfot_token_address.clone().into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: bfot_burn_amount,
                })?,
            }));
        }

        return Ok(Response::new()
            .add_messages(messages)
            .add_attributes(vec![
                attr("action", "send_gfot_burn_bfot"),
                attr("address", user_addr),
                attr("bfot_burn_amount", bfot_burn_amount),
                attr("gfot_send_amount", gfot_send_amount),
            ]));

    } 
     else {
        return Err(ContractError::UnacceptableToken {})
    }
}


pub fn check_owner(
    deps: &DepsMut,
    info: &MessageInfo
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {})
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    gfot_token_address: Option<Addr>
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;
    
    //test code for checking if check_owner works well
    // return Err(ContractError::InvalidInput {});
    // if owner some validated to addr, otherwise set to none
    let mut tmp_owner = None;
    if let Some(addr) = new_owner {
        tmp_owner = Some(deps.api.addr_validate(&addr)?)
    }

    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = tmp_owner;
        Ok(exists)
    })?;
    
    

    if let Some(new_address) = gfot_token_address {
        CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
            exists.gfot_token_address = new_address;
            Ok(exists)
        })?;
    }

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ExpectedAmount{bfot_amount} => to_binary(&query_expected_amount(deps, bfot_amount)?)
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.into()),
        bfot_token_address: cfg.bfot_token_address.into(),
        gfot_token_address: cfg.gfot_token_address.into(),
        bfot_burn_amount: cfg.bfot_burn_amount,
        gfot_sent_amount: cfg.gfot_sent_amount,
        bfot_expected_amount: cfg.gfot_sent_amount + Uint128::from(BFOT_START_AMOUNT),
        rate: cfg.rate,
        left: cfg.left
    })
}

pub fn query_expected_amount(deps: Deps, bfot_amount:Uint128) -> StdResult<ExpectedAmountResponse> {
    let mut cfg = CONFIG.load(deps.storage)?;
    
    let mut bfot_received_amount = bfot_amount.clone();
    let mut gfot_send_amount = Uint128::zero();
    let mut bfot_burn_amount = Uint128::zero();

    while bfot_received_amount > Uint128::zero() {
        
        if cfg.left < bfot_received_amount {
            //calculate the first slice 
            bfot_received_amount -= cfg.left;
            bfot_burn_amount += cfg.left;
            gfot_send_amount += cfg.left * Uint128::from(STEP_AMOUNT) / cfg.rate;
            
            cfg.rate += Uint128::from(STEP_AMOUNT);
            cfg.left = cfg.rate;

            if cfg.left >= bfot_received_amount {
                continue;
            }
            
            //now use the quadratic equation
            let a = bfot_received_amount.u128() / STEP_AMOUNT;
            let b = cfg.rate.u128() / STEP_AMOUNT;
            let mut n = (4 * b * b - 4 * b + 1 + 8 * a).integer_sqrt();
            n = n - 2 * b + 1;
            n = n / 2;
            
            // This n is the floor value, so use this
            cfg.rate += Uint128::from(n).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
            cfg.left = cfg.rate;

            let bfot_sum = n * b + n * (n - 1) / 2;
            let bfot_amount = Uint128::from(bfot_sum).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
            bfot_burn_amount += bfot_amount;
            gfot_send_amount += Uint128::from(n).checked_mul(Uint128::from(STEP_AMOUNT)).unwrap();
            bfot_received_amount -= bfot_amount;

        } else {
            cfg.left -= bfot_received_amount;
            bfot_burn_amount += bfot_received_amount;
            gfot_send_amount += bfot_received_amount * Uint128::from(STEP_AMOUNT) / cfg.rate;
            bfot_received_amount = Uint128::zero();
        }
    }
    Ok(ExpectedAmountResponse {
        bfot_burn_amount: bfot_burn_amount,
        gfot_send_amount: gfot_send_amount
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}
