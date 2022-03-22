use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::{Cw20ReceiveMsg};
use cosmwasm_std::{Uint128, Addr};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner if none set to info.sender.
    pub owner: Option<String>,
    pub fot_token_address: Addr,
    pub bfot_token_address: Addr,
    pub gfot_token_address: Addr,
    pub daily_fot_amount: Uint128,
    pub apy_prefix: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerInfo {
    pub address: Addr,
    pub amount: Uint128,
    pub reward: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        new_owner: Option<String>,
    },
    UpdateConstants {
        daily_fot_amount: Uint128,
        apy_prefix: Uint128,
    },
    Receive(Cw20ReceiveMsg),
    WithdrawFot { },
    WithdrawGFot { },
    ClaimReward { },
    Unstake {},
    UpdateLastTime {
        last_time: u64
    },
    AddStakers {
        stakers: Vec<StakerInfo>
    },
    RemoveStaker {
        address: Addr
    },
    RemoveAllStakers {
        start_after: Option<String>,
        limit: Option<u32>
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Stake {},
    InitialFund {},
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Staker {
        address: Addr
    },
    ListStakers {
        start_after: Option<String>,
        limit: Option<u32>
    },
    Apy {

    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: Option<String>,
    pub fot_token_address: String,
    pub bfot_token_address: String,
    pub gfot_token_address: String,
    pub fot_amount: Uint128,
    pub gfot_amount: Uint128,
    pub last_time: u64,
    pub daily_fot_amount: Uint128,
    pub apy_prefix: Uint128

}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct StakerListResponse {
    pub stakers: Vec<StakerInfo>,
}

/// Returns the vote (opinion as well as weight counted) as well as
/// the address of the voter who submitted it

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct StakerResponse {
    pub address: Addr,
    pub amount: Uint128,
    pub reward: Uint128
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CountInfo {
    pub count: u128
}
