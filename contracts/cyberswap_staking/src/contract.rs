use crate::error::ContractError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QuerierWrapper, Response, StdError, StdResult, Storage, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_storage_plus::Bound;

use crate::state::{Config, CONFIG, STAKED_DETAIL, STAKER_INFO};
use crate::util;
use classic_bindings::{TerraMsg, TerraQuery};
use classic_cyberswap::asset::AssetInfo;
use classic_cyberswap::querier::compute_tax;
use classic_cyberswap::staking::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg, StakedDetail,
    StakingList, StakingListResponse, StakingResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cyberswap-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub const ONE_DAY: u64 = 24 * 60 * 60; // 86400s
pub const ONE_WEEK: u64 = 7 * ONE_DAY; // 1 week to sec
pub const ONE_MONTH: u64 = 30 * ONE_DAY; // 30 days to sec
pub const ONE_YEAR: u64 = 12 * ONE_MONTH; // 12 months to sec
pub const ONE_YEAR_MONTH: u64 = 12; // 12 months
pub const ONE_MONTH_DAY: u64 = 30; // 30 days
pub const ONE_WEEK_DAY: u64 = 7; // 7 days
pub const DEFAULT_PRECISION: u64 = 1_000_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<TerraQuery>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<TerraMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        staking_asset: msg.staking_asset,
        reward_asset: msg.reward_asset,
        router_address: msg.router_address,
        min_lock_week: msg.min_lock_week,
        max_lock_week: msg.max_lock_week,
        min_ratio: msg.min_ratio,
        max_ratio: msg.max_ratio,
        referral_rate: msg.referral_rate,
        referral_lock_days: msg.referral_lock_days,
        enabled: msg.enabled,
    };

    CONFIG.save(deps.storage, &config)?;

    let staked_detail = StakedDetail {
        total_acc: Uint128::zero(),
        total_wcc: Uint128::zero(),
        total_staked: Uint128::zero(),
    };

    STAKED_DETAIL.save(deps.storage, &staked_detail)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsg>, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            staking_asset,
            reward_asset,
            router_address,
            min_lock_week,
            max_lock_week,
            min_ratio,
            max_ratio,
            referral_rate,
            referral_lock_days,
            enabled,
        } => execute_update_config(
            deps,
            env,
            info,
            owner,
            staking_asset,
            reward_asset,
            router_address,
            min_lock_week,
            max_lock_week,
            min_ratio,
            max_ratio,
            referral_rate,
            referral_lock_days,
            enabled,
        ),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Stake {
            lock_week,
            amount,
            referral_addr,
        } => {
            let referral_address = deps
                .api
                .addr_validate(&referral_addr.to_string())
                .unwrap_or(referral_addr);
            execute_stake(deps, env, info.sender, lock_week, amount, referral_address)
        }
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::ClaimReward {} => execute_claim_reward(deps, env, info),
        ExecuteMsg::ClaimReferralReward {} => execute_claim_referral_reward(deps, env, info),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, env, info),
    }
}

pub fn execute_update_config(
    deps: DepsMut<TerraQuery>,
    _env: Env,
    info: MessageInfo,
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
) -> Result<Response<TerraMsg>, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    check_owner(&deps, &info)?;

    if let Some(owner) = owner {
        // validate address format
        let _ = deps
            .api
            .addr_validate(&owner)
            .unwrap_or(Addr::unchecked(owner.clone()));
        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(staking_asset) = staking_asset {
        config.staking_asset = staking_asset;
    }

    if let Some(reward_asset) = reward_asset {
        config.reward_asset = reward_asset;
    }

    if let Some(router_address) = router_address {
        config.router_address = router_address;
    }

    if let Some(min_lock_week) = min_lock_week {
        config.min_lock_week = min_lock_week;
    }

    if let Some(max_lock_week) = max_lock_week {
        config.max_lock_week = max_lock_week;
    }

    if let Some(min_ratio) = min_ratio {
        config.min_ratio = min_ratio;
    }

    if let Some(max_ratio) = max_ratio {
        config.max_ratio = max_ratio;
    }

    if let Some(referral_rate) = referral_rate {
        config.referral_rate = referral_rate;
    }

    if let Some(referral_lock_days) = referral_lock_days {
        config.referral_lock_days = referral_lock_days;
    }

    if let Some(enabled) = enabled {
        config.enabled = enabled;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_receive(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response<TerraMsg>, ContractError> {
    if wrapper.amount == Uint128::zero() {
        return Err(ContractError::InvalidInput {});
    }
    let config: Config = CONFIG.load(deps.storage)?;

    if info.sender.clone() != config.staking_asset.to_addr()? {
        return Err(ContractError::UnacceptableToken {});
    }

    let user_addr = &deps
        .api
        .addr_validate(&wrapper.sender)
        .unwrap_or(Addr::unchecked(wrapper.sender));
    //TODO: let user_addr = &deps.api.addr_validate(&wrapper.sender)?;
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;

    match msg {
        ReceiveMsg::Stake {
            lock_week,
            referral_addr,
        } => {
            let referral_address = deps
                .api
                .addr_validate(&referral_addr.to_string())
                .unwrap_or(referral_addr);

            execute_stake(
                deps,
                env,
                user_addr.clone(),
                lock_week,
                wrapper.amount,
                referral_address,
            )
        }
    }
}

// This calculate the ratio value by the inputed lock_week value using the quadratic function.
pub fn calc_ratio(config: Config, lock_week: u64) -> StdResult<u64> {
    // -------------------------------------- //
    // y = (dY / dX^2) * (x - xMin)^2 + yMin;
    // x: lock week
    // xMax: max lock week
    // xMin: min lock week
    // yMax: max ratio
    // yMin: min ratio
    // dX = xMax - xMin
    // dY = yMax - yMin
    // -------------------------------------- //
    let x = lock_week;
    let x_min = config.min_lock_week.clone();
    let x_max = config.max_lock_week.clone();
    let y_min = config.min_ratio.clone();
    let y_max = config.max_ratio.clone();
    let d_x = x_max - x_min;
    let d_y = y_max - y_min;
    let result =
        ((d_y * DEFAULT_PRECISION) / d_x.pow(2)) * (x - x_min).pow(2) + y_min * DEFAULT_PRECISION;
    Ok(result)
}

// This calculate the rewards for stakers.
pub fn calc_reward(
    config: Config,
    now: u64,
    lock_week: u64,
    stake_amount: Uint128,
    reward_timestamp: u64,
) -> Result<Uint128, ContractError> {
    let periods = now - reward_timestamp;
    if now < reward_timestamp {
        return Ok(Uint128::zero());
    }
    let ratio = calc_ratio(config, lock_week)?;
    let annual_reward = stake_amount
        .checked_mul(Uint128::from(ratio))?
        .checked_div(Uint128::from(100u64))
        .map_err(StdError::divide_by_zero)?
        .checked_div(Uint128::from(DEFAULT_PRECISION))
        .map_err(StdError::divide_by_zero)?;
    let return_reward = annual_reward
        .checked_mul(Uint128::from(DEFAULT_PRECISION))?
        .checked_div(Uint128::from(ONE_YEAR))
        .map_err(StdError::divide_by_zero)?
        .checked_mul(Uint128::from(periods))?
        .checked_div(Uint128::from(DEFAULT_PRECISION))
        .map_err(StdError::divide_by_zero)?;

    Ok(return_reward)
}

pub fn update_reward(
    storage: &mut dyn Storage,
    querier: QuerierWrapper<TerraQuery>,
    env: Env,
    address: Addr,
) -> Result<Response<TerraMsg>, ContractError> {
    let config = CONFIG.load(storage)?;
    let mut staker_info = STAKER_INFO
        .load(storage, address.clone())
        .unwrap_or_default();
    if staker_info.stake_amount.is_zero() {
        return Ok(Response::new());
    }
    let swap_amount = util::get_simulate_swap_operations(
        querier,
        config.router_address.clone().into(),
        staker_info.stake_amount,
        config.staking_asset.clone().into(),
        config.reward_asset.clone().into(),
    )?;
    let reward = calc_reward(
        config,
        env.block.time.seconds(),
        staker_info.lock_week,
        swap_amount,
        staker_info.reward_timestamp,
    )?;
    staker_info.reward_amount = reward;
    STAKER_INFO
        .save(storage, address.clone(), &staker_info.clone())
        .unwrap();
    Ok(Response::new().add_attribute("action", "update_reward"))
}

pub fn check_lock(now: u64, staked_timestamp: u64, lock_week: u64) -> bool {
    let lock_times = staked_timestamp + lock_week * ONE_WEEK;
    if now < lock_times {
        return false;
    } else {
        return true;
    }
}

pub fn execute_stake(
    deps: DepsMut<TerraQuery>,
    env: Env,
    sender: Addr,
    lock_week: u64,
    amount: Uint128,
    referral_addr: Addr,
) -> Result<Response<TerraMsg>, ContractError> {
    if referral_addr.to_string().is_empty() || referral_addr == sender.clone() {
        return Err(ContractError::InvalidReferralAddr {});
    }
    let config = CONFIG.load(deps.storage)?;

    let balance = util::get_token_amount(
        deps.querier,
        config.staking_asset.to_denom()?,
        sender.clone(),
    )?;

    if lock_week < config.min_lock_week || lock_week > config.max_lock_week {
        return Err(ContractError::InvalidInput {});
    }

    if amount > balance {
        return Err(ContractError::Insufficient {});
    }

    let mut staker_info = STAKER_INFO
        .load(deps.storage, sender.clone())
        .unwrap_or_default();

    let mut last_reward = Uint128::zero();
    if staker_info.lock_week > 0 {
        let swap_amount = util::get_simulate_swap_operations(
            deps.querier,
            config.router_address.clone().into(),
            staker_info.stake_amount,
            config.staking_asset.clone().into(),
            config.reward_asset.clone().into(),
        )?;
        last_reward = calc_reward(
            config.clone(),
            env.block.time.seconds(),
            staker_info.lock_week,
            swap_amount,
            staker_info.reward_timestamp,
        )?;
    }

    let referral_staked_amount = amount
        .checked_mul(Uint128::from(config.referral_rate))?
        .checked_div(Uint128::from(100u64))
        .map_err(StdError::divide_by_zero)?;
    let referral_reward = util::get_simulate_swap_operations(
        deps.querier,
        config.router_address.clone().into(),
        referral_staked_amount,
        config.staking_asset.clone().into(),
        config.reward_asset.clone().into(),
    )?;

    let mut real_referral_addr = referral_addr;
    if !staker_info.referral_addr.to_string().is_empty() {
        real_referral_addr = staker_info.referral_addr.clone().into();
    }

    let mut referral_info = STAKER_INFO
        .load(deps.storage, real_referral_addr.clone())
        .unwrap_or_default();
    referral_info.referral_reward += referral_reward;
    if staker_info.referral_addr.to_string().is_empty() {
        referral_info.referral_count += 1u64;
    }
    if referral_info.referral_timestamp == 0 {
        referral_info.referral_timestamp = env.block.time.seconds();
    }

    STAKER_INFO.save(deps.storage, real_referral_addr.clone(), &referral_info)?;

    staker_info.lock_week = lock_week;
    staker_info.stake_amount += amount;
    staker_info.last_reward += last_reward;
    staker_info.staked_timestamp = env.block.time.seconds();
    staker_info.reward_timestamp = env.block.time.seconds();
    staker_info.referral_addr = real_referral_addr.clone();
    STAKER_INFO.save(deps.storage, sender.clone(), &staker_info)?;

    let ratio = calc_ratio(config.clone(), lock_week)?;
    let mut detail = STAKED_DETAIL.load(deps.storage)?;
    detail.total_acc += Uint128::from(ratio)
        .checked_mul(Uint128::from(amount))?
        .checked_div(Uint128::from(DEFAULT_PRECISION))
        .map_err(StdError::divide_by_zero)?;
    detail.total_wcc += Uint128::from(lock_week) * Uint128::from(amount);
    detail.total_staked += amount;
    STAKED_DETAIL.save(deps.storage, &detail)?;

    return Ok(Response::new().add_attributes(vec![
        ("action", "stake"),
        ("staker", &sender.to_string()),
        ("stake_amount", &amount.to_string()),
    ]));
}

pub fn execute_unstake(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<TerraMsg>, ContractError> {
    check_enabled(&deps, &info)?;
    update_reward(deps.storage, deps.querier, env.clone(), info.sender.clone()).unwrap();

    let config = CONFIG.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .load(deps.storage, info.sender.clone())
        .unwrap_or_default();

    if amount > staker_info.stake_amount {
        return Err(ContractError::InvalidInput {});
    }

    if check_lock(
        env.block.time.seconds(),
        staker_info.staked_timestamp,
        staker_info.lock_week,
    ) == true
    {
        let ratio = calc_ratio(config.clone(), staker_info.lock_week)?;
        let mut detail = STAKED_DETAIL.load(deps.storage)?;
        detail.total_acc -= Uint128::from(ratio) * amount / Uint128::from(DEFAULT_PRECISION);
        detail.total_wcc -= Uint128::from(staker_info.lock_week) * amount;
        detail.total_staked -= amount;
        STAKED_DETAIL.save(deps.storage, &detail)?;
    } else {
        return Err(ContractError::StillInLock {});
    }

    let total_rewards = staker_info.reward_amount + staker_info.last_reward;

    if total_rewards.is_zero() {
        return Err(ContractError::NotEnoughReward {});
    }

    staker_info.stake_amount -= amount;
    staker_info.reward_amount = Uint128::zero();
    staker_info.last_reward = Uint128::zero();
    staker_info.reward_timestamp = env.block.time.seconds();
    STAKER_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;

    let mut amount1 = amount;
    let mut amount2 = total_rewards;
    if let AssetInfo::NativeToken { denom } = config.staking_asset.clone() {
        amount1 = amount1.checked_sub(compute_tax(&deps.querier, amount1, denom.clone())?)?;
    }

    if let AssetInfo::NativeToken { denom } = config.reward_asset.clone() {
        amount2 = amount2.checked_sub(compute_tax(&deps.querier, amount2, denom.clone())?)?;
    }

    let mut msgs: Vec<CosmosMsg<TerraMsg>> = vec![];
    msgs.push(util::transfer_token_message(
        config.staking_asset.to_denom()?,
        amount1,
        info.sender.clone(),
    )?);
    msgs.push(util::transfer_token_message(
        config.reward_asset.to_denom()?,
        amount2,
        info.sender.clone(),
    )?);

    return Ok(Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "unstake"),
        ("unstaker", info.sender.as_str()),
        ("ustaked_amount", &amount.to_string()),
    ]));
}

pub fn execute_claim_reward(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TerraMsg>, ContractError> {
    check_enabled(&deps, &info)?;
    let config = CONFIG.load(deps.storage)?;
    update_reward(deps.storage, deps.querier, env.clone(), info.sender.clone()).unwrap();

    let mut staker_info = STAKER_INFO
        .load(deps.storage, info.sender.clone())
        .unwrap_or_default();

    if check_lock(
        env.block.time.seconds(),
        staker_info.staked_timestamp,
        staker_info.lock_week,
    ) == false
    {
        return Err(ContractError::StillInLock {});
    }

    let reward_amount = staker_info.reward_amount + staker_info.last_reward;

    if reward_amount.is_zero() {
        return Err(ContractError::NotEnoughReward {});
    }

    staker_info.reward_timestamp = env.block.time.seconds();
    staker_info.reward_amount = Uint128::zero();
    staker_info.last_reward = Uint128::zero();
    STAKER_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;

    let mut msgs: Vec<CosmosMsg<TerraMsg>> = vec![];
    msgs.push(util::transfer_token_message(
        config.reward_asset.to_denom()?,
        reward_amount,
        info.sender.clone(),
    )?);

    return Ok(Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "claim_reward"),
        ("address", info.sender.as_str()),
        ("reward_amount", &reward_amount.to_string()),
    ]));
}

pub fn execute_claim_referral_reward(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TerraMsg>, ContractError> {
    check_enabled(&deps, &info)?;
    let config = CONFIG.load(deps.storage)?;
    update_reward(deps.storage, deps.querier, env.clone(), info.sender.clone()).unwrap();

    let mut staker_info = STAKER_INFO
        .load(deps.storage, info.sender.clone())
        .unwrap_or_default();

    if env.block.time.seconds()
        < (staker_info.referral_timestamp + config.referral_lock_days * ONE_DAY)
    {
        return Err(ContractError::StillInLock {});
    }

    if staker_info.referral_reward.is_zero() {
        return Err(ContractError::NotEnoughReward {});
    }

    let referral_reward = staker_info.referral_reward;
    staker_info.referral_reward = Uint128::zero();
    STAKER_INFO.save(deps.storage, info.sender.clone(), &staker_info)?;

    let mut msgs: Vec<CosmosMsg<TerraMsg>> = vec![];
    msgs.push(util::transfer_token_message(
        config.reward_asset.to_denom()?,
        referral_reward,
        info.sender.clone(),
    )?);

    return Ok(Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "claim_referral_reward"),
        ("address", info.sender.as_str()),
        ("reward_amount", &referral_reward.to_string()),
    ]));
}

pub fn execute_withdraw(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TerraMsg>, ContractError> {
    check_owner(&deps, &info)?;
    let config: Config = CONFIG.load(deps.storage)?;

    let staking_balance = util::get_token_amount(
        deps.querier,
        config.staking_asset.to_denom()?,
        env.contract.address.clone(),
    )?;

    let reward_balance = util::get_token_amount(
        deps.querier,
        config.reward_asset.to_denom()?,
        env.contract.address.clone(),
    )?;

    let mut msgs: Vec<CosmosMsg<TerraMsg>> = vec![];
    let mut amount1 = staking_balance;
    let mut amount2 = reward_balance;

    if let AssetInfo::NativeToken { denom } = config.staking_asset.clone() {
        amount1 = amount1.checked_sub(compute_tax(&deps.querier, amount1, denom.clone())?)?;
    }

    if let AssetInfo::NativeToken { denom } = config.reward_asset.clone() {
        amount2 = amount2.checked_sub(compute_tax(&deps.querier, amount2, denom.clone())?)?;
    }

    msgs.push(util::transfer_token_message(
        config.staking_asset.to_denom()?,
        amount1,
        info.sender.clone(),
    )?);

    if config.staking_asset.equal(&config.reward_asset) {
        amount2 = Uint128::zero();
    } else {
        msgs.push(util::transfer_token_message(
            config.reward_asset.to_denom()?,
            amount2,
            info.sender.clone(),
        )?);
    }

    return Ok(Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "withdraw_reward"),
        ("address", info.sender.as_str()),
        ("amount1", &amount1.to_string()),
        ("amount2", &amount2.to_string()),
    ]));
}

pub fn check_owner(
    deps: &DepsMut<TerraQuery>,
    info: &MessageInfo,
) -> Result<Response<TerraMsg>, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn check_enabled(
    deps: &DepsMut<TerraQuery>,
    _info: &MessageInfo,
) -> Result<Response<TerraMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.enabled {
        return Err(ContractError::Disabled {});
    }
    Ok(Response::new().add_attribute("action", "check_enabled"))
}

//---------------------------------------------------------//
//                           QUERY                         //
//---------------------------------------------------------//
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<TerraQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::StakerInfo { address } => to_binary(&query_staking_info(deps, _env, address)?),
        QueryMsg::StakingList { start_after, limit } => {
            to_binary(&query_get_staking_list(deps, start_after, limit)?)
        }
        QueryMsg::StakedDetail {} => to_binary(&query_staked_detail(deps)?),
        QueryMsg::Now {} => to_binary(&query_get_now(_env)?),
    }
}

pub fn query_config(deps: Deps<TerraQuery>) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        staking_asset: config.staking_asset,
        reward_asset: config.reward_asset,
        router_address: config.router_address,
        min_lock_week: config.min_lock_week,
        max_lock_week: config.max_lock_week,
        min_ratio: config.min_ratio,
        max_ratio: config.max_ratio,
        referral_rate: config.referral_rate,
        referral_lock_days: config.referral_lock_days,
        enabled: config.enabled,
    })
}

fn query_staking_info(
    deps: Deps<TerraQuery>,
    env: Env,
    address: Addr,
) -> StdResult<StakingResponse> {
    let config = CONFIG.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .load(deps.storage, address.clone())
        .unwrap_or_default();

    let mut reward = Uint128::zero();
    if !staker_info.stake_amount.is_zero() {
        let swap_amount = util::get_simulate_swap_operations(
            deps.querier,
            config.router_address.clone().into(),
            staker_info.stake_amount,
            config.staking_asset.clone().into(),
            config.reward_asset.clone().into(),
        )
        .unwrap_or(Uint128::zero());
        reward = calc_reward(
            config,
            env.block.time.seconds(),
            staker_info.lock_week,
            swap_amount,
            staker_info.reward_timestamp,
        )
        .unwrap_or(Uint128::zero());
    }
    staker_info.reward_amount = reward;

    Ok(StakingResponse {
        address: address.to_string(),
        info: staker_info,
    })
}

pub fn query_get_staking_list(
    deps: Deps<TerraQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<StakingListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));

    let amount_list = STAKER_INFO
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(address, info)| StakingList {
                address: address.to_string(),
                info,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(StakingListResponse { list: amount_list })
}

pub fn query_staked_detail(deps: Deps<TerraQuery>) -> StdResult<StakedDetail> {
    let detail = STAKED_DETAIL.load(deps.storage)?;
    Ok(detail)
}

pub fn query_get_now(env: Env) -> StdResult<u64> {
    Ok(env.block.time.seconds())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut<TerraQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> StdResult<Response<TerraMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
