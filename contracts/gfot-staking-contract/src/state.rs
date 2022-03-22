use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Option<Addr>,
    pub fot_token_address: Addr,
    pub bfot_token_address: Addr,
    pub gfot_token_address: Addr,
    pub fot_amount: Uint128,
    pub gfot_amount: Uint128,
    pub last_time: u64,
    pub daily_fot_amount: Uint128,
    pub apy_prefix: Uint128
    
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const STAKERS_KEY: &str = "stakers";
pub const STAKERS: Map<Addr, (Uint128, Uint128)> = Map::new(STAKERS_KEY);
