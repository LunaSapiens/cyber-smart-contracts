use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

//----------------------------------------------------------------------------------------
// Struct's :: Contract State
//----------------------------------------------------------------------------------------

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE_INFO: Item<StateInfo> = Item::new("state");
pub const STAKER_INFO: Map<&Addr, StakerInfo> = Map::new("staker");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub reward_token: Addr,
    pub staking_token: Addr,
    pub staking_token_decimals: u8,
    pub distribution_schedule: (u64, u64, Uint128),
    pub referral_rate: u64,
    pub referral_lock_days: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateInfo {
    /// Timestamp at which the global_reward_index was last updated
    pub last_distributed: u64,
    /// Total number of CYBER-LUNC LP tokens staked with the contract
    pub total_stake_amount: Uint128,
    /// Used to calculate CYBER rewards accured over time elapsed. Ratio =  Total distributed CYBER tokens / total stake amount
    pub global_reward_index: Decimal,
    /// Number of CYBER tokens that are yet to be distributed
    pub leftover: Uint128,
    /// Number of CYBER tokens distributed per staked LP token
    pub reward_rate_per_token: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerInfo {
    /// Number of CYBER-LUNC LP tokens staked by the user
    pub stake_amount: Uint128,
    /// Used to calculate CYBER rewards accured over time elapsed. Ratio = distributed CYBER tokens / user's staked amount
    pub reward_index: Decimal,
    /// Pending CYBER tokens which are yet to be claimed
    pub pending_reward: Uint128,
    pub referral_addr: Addr,
    pub referral_reward: Uint128,
    pub referral_count: u64,
    pub referral_timestamp: u64,
}

impl Default for StakerInfo {
    fn default() -> Self {
        StakerInfo {
            reward_index: Decimal::one(),
            stake_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
            referral_addr: Addr::unchecked(""),
            referral_reward: Uint128::zero(),
            referral_count: 0,
            referral_timestamp: 0,
        }
    }
}
