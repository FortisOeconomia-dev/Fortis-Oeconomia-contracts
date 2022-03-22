#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg};
use cw20::{TokenInfoResponse};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, 
};
use crate::state::{
    Config, CONFIG
};

// Version info, for migration info
const CONTRACT_NAME: &str = "fotburn";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const FOT_STEP:u128 = 10_000_000_000_000_000u128;


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
        fot_token_address: deps.api.addr_validate(&msg.fot_token_address)?,
        bfot_token_address: deps.api.addr_validate(&msg.bfot_token_address)?,
        fot_burn_amount: Uint128::zero(),
        bfot_sent_amount: Uint128::zero(),
        bfot_current_amount: Uint128::zero()
    };
    CONFIG.save(deps.storage, &config)?;

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
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, info, new_owner),
        ExecuteMsg::Receive(msg) => try_receive(deps, info, msg),
        ExecuteMsg::WithdrawAll {} => try_withdraw_all(deps, info),
    }
}

//calculate fot_rate according to the fot_amount
pub fn calc_fot_rate(
    fot_amount: Uint128
)-> Uint128 {
    
    let mut step = (fot_amount - Uint128::from(1u128)).checked_div(Uint128::from(FOT_STEP)).unwrap();
    step = step + Uint128::from(1u128);

    return Uint128::from(110u128) - step;
}

//calculate bfot amount to send according to the fot_amount and fot_rate
pub fn calc_bfot_amount(
    fot_amount: Uint128,
    fot_rate: Uint128
)-> Uint128 {
    
    return fot_amount.checked_mul(fot_rate).unwrap();
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

    let fot_token_info: TokenInfoResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cfg.fot_token_address.clone().into(),
            msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
        }))?;

    let fot_left_amount = Uint128::from(fot_token_info.total_supply);
    let user_addr = &deps.api.addr_validate(&wrapper.sender)?;

    if info.sender == cfg.fot_token_address {
        // Not possible code
        // if fot_left_amount < wrapper.amount {
        //     return Err(ContractError::NotEnoughFOT {});
        // }

        let fot_received_amount = wrapper.amount;
        let mut bfot_send_amount = Uint128::zero();
        let mut amount = wrapper.amount;
        let mut fot_amount = fot_left_amount;

        while amount > Uint128::zero() {
            let mut sliceamount = fot_amount.checked_rem(Uint128::from(FOT_STEP)).unwrap();
            if sliceamount == Uint128::zero() {
                sliceamount = Uint128::from(FOT_STEP);
            }
            if sliceamount > amount {
                sliceamount = amount;
            }
            bfot_send_amount = bfot_send_amount + calc_bfot_amount(sliceamount, calc_fot_rate(fot_amount));
            fot_amount = fot_amount - sliceamount;
            amount = amount - sliceamount;
        }
        // bfot_send_amount = amount.checked_div(Uint128::from(10u128)).unwrap();
        if cfg.bfot_current_amount < bfot_send_amount {
            return Err(ContractError::NotEnoughbFOT {})
        }
        cfg.fot_burn_amount = cfg.fot_burn_amount + fot_received_amount;
        cfg.bfot_sent_amount = cfg.bfot_sent_amount + bfot_send_amount;
        cfg.bfot_current_amount = cfg.bfot_current_amount - bfot_send_amount;

        CONFIG.save(deps.storage, &cfg)?;
        
        //send bfot_send_amount, burn fot_received_amount
        return Ok(Response::new()
            .add_message(WasmMsg::Execute {
                contract_addr: cfg.bfot_token_address.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: user_addr.into(),
                    amount: bfot_send_amount,
                })?,
            })
            .add_message(WasmMsg::Execute {
                contract_addr: cfg.fot_token_address.into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: fot_received_amount,
                })?,
            })
            .add_attributes(vec![
                attr("action", "send_bfot_burn_fot"),
                attr("address", user_addr),
                attr("bfot_amount", bfot_send_amount),
                attr("fot_amount", fot_received_amount),
            ]));

    } else if info.sender == cfg.bfot_token_address {
        //Just receive in contract cache and update config
        cfg.bfot_current_amount = cfg.bfot_current_amount + wrapper.amount;
        CONFIG.save(deps.storage, &cfg)?;

        return Ok(Response::new()
            .add_attributes(vec![
                attr("action", "receive_bfot"),
                attr("address", user_addr),
                attr("bfot_amount", wrapper.amount),
            ]));

    } else {
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

    Ok(Response::new().add_attribute("action", "update_config"))
}


pub fn try_withdraw_all(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {

    check_owner(&deps, &info)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    
    let bfot_current_amount = cfg.bfot_current_amount;
    let bfot_token_address = cfg.bfot_token_address.clone();
    cfg.bfot_current_amount = Uint128::zero();

    CONFIG.save(deps.storage, &cfg)?;

    // create transfer cw20 msg
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: bfot_token_address.into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: bfot_current_amount,
        })?,
        funds: vec![],
    };

    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "bfot_withdraw_all"),
            attr("address", info.sender.clone()),
            attr("bfot_amount", bfot_current_amount),
        ]));
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.into()),
        fot_token_address: cfg.fot_token_address.into(),
        bfot_token_address: cfg.bfot_token_address.into(),
        fot_burn_amount: cfg.fot_burn_amount,
        bfot_sent_amount: cfg.bfot_sent_amount,
        bfot_current_amount: cfg.bfot_current_amount
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
