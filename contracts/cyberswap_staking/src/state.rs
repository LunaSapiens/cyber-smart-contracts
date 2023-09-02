use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use classic_cyberswap::asset::AssetInfo;
use classic_cyberswap::staking::{StakedDetail, StakerInfo};
use cosmwasm_std::{Addr, CanonicalAddr};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub staking_asset: AssetInfo,
    pub reward_asset: AssetInfo,
    pub router_address: Addr,
    pub min_lock_week: u64,
    pub max_lock_week: u64,
    pub min_ratio: u64,
    pub max_ratio: u64,
    pub referral_rate: u64,
    pub referral_lock_days: u64,
    pub enabled: bool,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new("config");

pub const STAKING_KEY: &str = "staking";
pub const STAKER_INFO: Map<Addr, StakerInfo> = Map::new(STAKING_KEY);

pub const STAKED_DETAIL_KEY: &str = "staked_detail";
pub const STAKED_DETAIL: Item<StakedDetail> = Item::new(STAKED_DETAIL_KEY);
