use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub reward_token: String,
    pub staking_token: String,
    pub staking_token_decimals: u8,
    pub referral_rate: u64,
    pub referral_lock_days: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Open a new user position or add to an existing position
    /// @dev Increase the total LP shares Staked by equal no. of shares as sent by the user
    Receive(Cw20ReceiveMsg),
    /// @param new_owner The new owner address
    UpdateConfig {
        owner: Option<String>,
        referral_rate: Option<u64>,
        referral_lock_days: Option<u64>,
    },
    /// Decrease the total LP shares Staked by the user
    /// Accrued rewards are claimed along-with this function
    /// @param amount The no. of LP shares to be subtracted from the total Staked and sent back to the user
    Unstake {
        amount: Uint128,
        withdraw_pending_reward: Option<bool>,
    },
    /// Claim pending rewards
    Claim {},
    ClaimReferralReward {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Open a new user position or add to an existing position (Cw20ReceiveMsg)
    Stake { referral_addr: Addr },
    UpdateRewardSchedule {
        period_start: u64,
        period_finish: u64,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the contract configuration
    Config {},
    /// Returns the global state of the contract
    /// @param timestamp Optional value which can be passed to calculate global_reward_index at a certain timestamp
    StateInfo { timestamp: Option<u64> },
    /// Returns the state of a user's staked position (StakerInfo)
    /// @param timestamp Optional value which can be passed to calculate reward_index, pending_reward at a certain timestamp
    StakerInfo {
        staker: String,
        timestamp: Option<u64>,
    },
    /// Helper function, returns the current timestamp
    Timestamp {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub reward_token: String,
    ///  LP token address
    pub staking_token: String,
    /// Distribution Schedules
    pub distribution_schedule: (u64, u64, Uint128),
    pub referral_rate: u64,
    pub referral_lock_days: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// Timestamp at which the global_reward_index was last updated
    pub last_distributed: u64,
    /// Total number of CYBER-UST LP tokens deposited in the contract
    pub total_stake_amount: Uint128,
    ///  total CYBER rewards / total_stake_amount ratio. Used to calculate CYBER rewards accured over time elapsed
    pub global_reward_index: Decimal,
    /// Number of CYBER tokens that are yet to be distributed
    pub leftover: Uint128,
    /// Number of CYBER tokens distributed per staked LP tokens
    pub reward_rate_per_token: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerInfoResponse {
    /// User address
    pub staker: String,
    /// CYBER-UST LP tokens deposited by the user
    pub stake_amount: Uint128,
    /// CYBER rewards / stake_amount ratio.  Used to calculate CYBER rewards accured over time elapsed
    pub reward_index: Decimal,
    /// Pending CYBER rewards which are yet to be claimed
    pub pending_reward: Uint128,
    pub referral_addr: Addr,
    pub referral_reward: Uint128,
    pub referral_count: u64,
    pub referral_timestamp: u64,
}
