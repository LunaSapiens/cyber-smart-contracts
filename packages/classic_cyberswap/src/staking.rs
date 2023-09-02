use crate::asset::AssetInfo;
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// UpdateConfig update relevant code IDs
    UpdateConfig {
        owner: Option<String>,
        staking_asset: Option<AssetInfo>,
        reward_asset: Option<AssetInfo>,
        router_address: Option<Addr>,
        min_lock_week: Option<u64>,
        max_lock_week: Option<u64>,
        min_ratio: Option<u64>,
        max_ratio: Option<u64>,
        referral_rate: Option<u64>,
        referral_lock_days: Option<u64>,
        enabled: Option<bool>,
    },
    Receive(Cw20ReceiveMsg),
    Stake {
        lock_week: u64,
        amount: Uint128,
        referral_addr: Addr,
    },
    Unstake {
        amount: Uint128,
    },
    ClaimReward {},
    ClaimReferralReward {},
    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Stake { lock_week: u64, referral_addr: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    StakerInfo {
        address: Addr,
    },
    StakingList {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    StakedDetail {},
    Now {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
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

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakerInfo {
    pub lock_week: u64,
    pub stake_amount: Uint128,
    pub reward_amount: Uint128,
    pub last_reward: Uint128,
    pub referral_addr: Addr,
    pub referral_reward: Uint128,
    pub referral_count: u64,
    pub staked_timestamp: u64,
    pub reward_timestamp: u64,
    pub referral_timestamp: u64,
}

impl Default for StakerInfo {
    fn default() -> Self {
        StakerInfo {
            lock_week: 0,
            stake_amount: Uint128::zero(),
            reward_amount: Uint128::zero(),
            last_reward: Uint128::zero(),
            referral_addr: Addr::unchecked(""),
            referral_reward: Uint128::zero(),
            referral_count: 0,
            staked_timestamp: 0,
            reward_timestamp: 0,
            referral_timestamp: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakedDetail {
    pub total_acc: Uint128,
    pub total_wcc: Uint128,
    pub total_staked: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakingResponse {
    pub address: String,
    pub info: StakerInfo,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct StakingListResponse {
    pub list: Vec<StakingList>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct StakingList {
    pub address: String,
    pub info: StakerInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestBalanceResponse {
    pub balance: Uint128,
}
