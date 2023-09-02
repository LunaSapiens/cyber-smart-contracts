use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use classic_cyberswap::farming::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    StakerInfoResponse, StateResponse,
};

use crate::state::{Config, StakerInfo, StateInfo, CONFIG, STAKER_INFO, STATE_INFO};

const CONTRACT_NAME: &str = "crates.io:cyberswap-farming";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ONE_DAY: u64 = 24 * 60 * 60; // 86400s

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner = deps
        .api
        .addr_validate(&msg.owner)
        .unwrap_or(Addr::unchecked(msg.owner));
    let reward_token = deps
        .api
        .addr_validate(&msg.reward_token)
        .unwrap_or(Addr::unchecked(msg.reward_token));
    let staking_token = deps
        .api
        .addr_validate(&msg.staking_token)
        .unwrap_or(Addr::unchecked(msg.staking_token));

    let config = Config {
        owner: owner.clone(),
        reward_token: reward_token.clone(),
        staking_token: staking_token.clone(),
        staking_token_decimals: msg.staking_token_decimals,
        distribution_schedule: (0, 0, Uint128::zero()),
        referral_rate: msg.referral_rate,
        referral_lock_days: msg.referral_lock_days,
    };

    CONFIG.save(deps.storage, &config)?;

    STATE_INFO.save(
        deps.storage,
        &StateInfo {
            last_distributed: env.block.time.seconds(),
            total_stake_amount: Uint128::zero(),
            global_reward_index: Decimal::zero(),
            leftover: Uint128::zero(),
            reward_rate_per_token: Decimal::zero(),
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            referral_rate,
            referral_lock_days,
        } => update_config(deps, env, info, owner, referral_rate, referral_lock_days),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unstake {
            amount,
            withdraw_pending_reward,
        } => unstake(deps, env, info, amount, withdraw_pending_reward),
        ExecuteMsg::Claim {} => try_claim(deps, env, info),
        ExecuteMsg::ClaimReferralReward {} => try_claim_referral_reward(deps, env, info),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::StateInfo { timestamp } => to_binary(&query_state(deps, _env, timestamp)?),
        QueryMsg::StakerInfo { staker, timestamp } => {
            to_binary(&query_staker_info(deps, _env, staker, timestamp)?)
        }
        QueryMsg::Timestamp {} => to_binary(&query_timestamp(_env)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Err(StdError::generic_err("unimplemented"))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Stake { referral_addr }) => {
            if config.staking_token != info.sender.as_str() {
                return Err(StdError::generic_err("unauthorized"));
            }
            let cw20_sender = deps
                .api
                .addr_validate(&cw20_msg.sender)
                .unwrap_or(Addr::unchecked(cw20_msg.sender));
            //TODO: let cw20_sender = deps.api.addr_validate(&cw20_msg.sender)?;
            stake(deps, env, cw20_sender, cw20_msg.amount, referral_addr)
        }
        Ok(Cw20HookMsg::UpdateRewardSchedule {
            period_start,
            period_finish,
            amount,
        }) => {
            if config.reward_token != info.sender.as_str() {
                return Err(StdError::generic_err(
                    "Unauthorized : Only CYBER Token is allowed",
                ));
            }
            if config.owner != cw20_msg.sender {
                return Err(StdError::generic_err("Only owner can update the schedule"));
            }
            update_reward_schedule(
                deps,
                env,
                info,
                period_start,
                period_finish,
                amount,
                cw20_msg.amount,
            )
        }

        Err(_) => Err(StdError::generic_err("data should be given")),
    }
}

pub fn stake(
    deps: DepsMut,
    env: Env,
    sender_addr: Addr,
    amount: Uint128,
    referral_addr: Addr,
) -> StdResult<Response> {
    if referral_addr.to_string().is_empty() || referral_addr == sender_addr.clone() {
        return Err(StdError::generic_err("Invalid referral address"));
    }
    let config: Config = CONFIG.load(deps.storage)?;
    let mut state: StateInfo = STATE_INFO.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .may_load(deps.storage, &sender_addr)?
        .unwrap_or_default();

    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;
    increase_stake_amount(&mut state, &mut staker_info, amount);

    let mut real_referral_addr = referral_addr;
    if !staker_info.referral_addr.to_string().is_empty() {
        real_referral_addr = staker_info.referral_addr.clone().into();
    }
    let mut referral_info = STAKER_INFO
        .may_load(deps.storage, &real_referral_addr)?
        .unwrap_or_default();
    compute_referral_reward(
        &staker_info,
        &mut referral_info,
        config.referral_rate,
        amount,
        env.block.time.seconds(),
    )?;

    // Store updated state with staker's staker_info and referral
    STAKER_INFO.save(deps.storage, &sender_addr, &staker_info)?;
    STAKER_INFO.save(deps.storage, &real_referral_addr, &referral_info)?;
    STATE_INFO.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "Stake"),
        ("user", sender_addr.as_str()),
        ("amount", amount.to_string().as_str()),
        (
            "total_staked",
            staker_info.stake_amount.to_string().as_str(),
        ),
    ]))
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    referral_rate: Option<u64>,
    referral_lock_days: Option<u64>,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    // ONLY OWNER CAN UPDATE CONFIG
    if info.sender != config.owner {
        return Err(StdError::generic_err("Only owner can update configuration"));
    }

    if let Some(owner) = owner {
        config.owner = deps
            .api
            .addr_validate(&owner)
            .unwrap_or(Addr::unchecked(owner.clone()));
        //TODO: config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(referral_rate) = referral_rate {
        config.referral_rate = referral_rate;
    }

    if let Some(referral_lock_days) = referral_lock_days {
        config.referral_lock_days = referral_lock_days;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "UpdateConfig"))
}

pub fn update_reward_schedule(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    period_start: u64,
    period_finish: u64,
    amount_to_distribute: Uint128,
    amount_sent: Uint128,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    let mut state: StateInfo = STATE_INFO.load(deps.storage)?;

    compute_reward(&config, &mut state, env.block.time.seconds());

    if period_start > period_finish {
        return Err(StdError::generic_err("Invalid Period"));
    }

    if amount_sent + state.leftover < amount_to_distribute {
        return Err(StdError::generic_err("insufficient funds on contract"));
    }

    config.distribution_schedule = (period_start, period_finish, amount_to_distribute);

    CONFIG.save(deps.storage, &config)?;
    STATE_INFO.save(deps.storage, &state)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "update_reward_schedule"),
        (
            "cyber_to_distribute",
            amount_to_distribute.to_string().as_str(),
        ),
        (
            "total_stake_amount",
            state.total_stake_amount.to_string().as_str(),
        ),
    ]))
}

pub fn unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    withdraw_pending_reward: Option<bool>,
) -> StdResult<Response> {
    let sender_addr = info.sender;
    let config: Config = CONFIG.load(deps.storage)?;
    let mut state: StateInfo = STATE_INFO.load(deps.storage)?;
    let mut staker_info: StakerInfo = STAKER_INFO
        .may_load(deps.storage, &sender_addr)?
        .unwrap_or_default();

    if staker_info.stake_amount < amount {
        return Err(StdError::generic_err(
            "Cannot unstake more than stake amount",
        ));
    }

    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;
    decrease_stake_amount(&mut state, &mut staker_info, amount);
    let mut messages = vec![];
    let mut claimed_rewards = Uint128::zero();

    if let Some(withdraw_pending_reward) = withdraw_pending_reward {
        if withdraw_pending_reward {
            claimed_rewards = staker_info.pending_reward;
            if claimed_rewards > Uint128::zero() {
                staker_info.pending_reward = Uint128::zero();
                messages.push(build_send_cw20_token_msg(
                    sender_addr.clone(),
                    config.reward_token,
                    claimed_rewards,
                )?);
            }
        }
    }

    STAKER_INFO.save(deps.storage, &sender_addr, &staker_info)?;
    STATE_INFO.save(deps.storage, &state)?;

    messages.push(build_send_cw20_token_msg(
        sender_addr.clone(),
        config.staking_token,
        amount,
    )?);
    
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "Unstake"),
        ("user", sender_addr.as_str()),
        ("amount", amount.to_string().as_str()),
        (
            "total_staked",
            staker_info.stake_amount.to_string().as_str(),
        ),
        ("claimed_rewards", claimed_rewards.to_string().as_str()),
    ]))
}

pub fn try_claim(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let sender_addr = info.sender;
    let config: Config = CONFIG.load(deps.storage)?;
    let mut state: StateInfo = STATE_INFO.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .may_load(deps.storage, &sender_addr)?
        .unwrap_or_default();

    compute_reward(&config, &mut state, env.block.time.seconds());
    compute_staker_reward(&state, &mut staker_info)?;

    let accrued_rewards = staker_info.pending_reward;
    staker_info.pending_reward = Uint128::zero();

    STAKER_INFO.save(deps.storage, &sender_addr, &staker_info)?; 
    STATE_INFO.save(deps.storage, &state)?; 

    let mut messages = vec![];

    if accrued_rewards == Uint128::zero() {
        return Err(StdError::generic_err("No rewards to claim"));
    } else {
        messages.push(build_send_cw20_token_msg(
            sender_addr.clone(),
            config.reward_token,
            accrued_rewards,
        )?);
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "Claim"),
        ("user", sender_addr.as_str()),
        ("claimed_rewards", accrued_rewards.to_string().as_str()),
    ]))
}

pub fn try_claim_referral_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> StdResult<Response> {
    let sender_addr = info.sender;
    let config: Config = CONFIG.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .may_load(deps.storage, &sender_addr)?
        .unwrap_or_default();
    if env.block.time.seconds()
        < (staker_info.referral_timestamp + config.referral_lock_days * ONE_DAY)
    {
        return Err(StdError::generic_err("Still in Lock period"));
    }

    if staker_info.referral_reward.is_zero() {
        return Err(StdError::generic_err("No rewards to claim"));
    }

    let referral_reward = staker_info.referral_reward;
    staker_info.referral_reward = Uint128::zero();
    STAKER_INFO.save(deps.storage, &sender_addr, &staker_info)?;

    let mut messages = vec![];
    messages.push(build_send_cw20_token_msg(
        sender_addr.clone(),
        config.reward_token,
        referral_reward,
    )?);
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "claim_referral_reward"),
        ("user", sender_addr.as_str()),
        ("claimed_rewards", referral_reward.to_string().as_str()),
    ]))
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        reward_token: config.reward_token.to_string(),
        staking_token: config.staking_token.to_string(),
        distribution_schedule: config.distribution_schedule,
        referral_rate: config.referral_rate,
        referral_lock_days: config.referral_lock_days,
    })
}

pub fn query_state(deps: Deps, env: Env, timestamp: Option<u64>) -> StdResult<StateResponse> {
    let mut state: StateInfo = STATE_INFO.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    match timestamp {
        Some(timestamp) => {
            compute_reward(
                &config,
                &mut state,
                std::cmp::max(timestamp, env.block.time.seconds()),
            );
        }
        None => {
            compute_reward(&config, &mut state, env.block.time.seconds());
        }
    }

    Ok(StateResponse {
        last_distributed: state.last_distributed,
        total_stake_amount: state.total_stake_amount,
        global_reward_index: state.global_reward_index,
        leftover: state.leftover,
        reward_rate_per_token: state.reward_rate_per_token,
    })
}

pub fn query_staker_info(
    deps: Deps,
    env: Env,
    staker: String,
    timestamp: Option<u64>,
) -> StdResult<StakerInfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let mut state = STATE_INFO.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .may_load(
            deps.storage,
            &deps
                .api
                .addr_validate(&staker)
                .unwrap_or(Addr::unchecked(staker.clone())),
        )?
        //TODO: .may_load(deps.storage, &deps.api.addr_validate(&staker)?)?
        .unwrap_or_default();

    match timestamp {
        Some(timestamp) => {
            compute_reward(
                &config,
                &mut state,
                std::cmp::max(timestamp, env.block.time.seconds()),
            );
        }
        None => {
            compute_reward(&config, &mut state, env.block.time.seconds());
        }
    }

    compute_staker_reward(&state, &mut staker_info)?;

    Ok(StakerInfoResponse {
        staker,
        reward_index: staker_info.reward_index,
        stake_amount: staker_info.stake_amount,
        pending_reward: staker_info.pending_reward,
        referral_addr: staker_info.referral_addr,
        referral_reward: staker_info.referral_reward,
        referral_count: staker_info.referral_count,
        referral_timestamp: staker_info.referral_timestamp,
    })
}

pub fn query_timestamp(env: Env) -> StdResult<u64> {
    Ok(env.block.time.seconds())
}

fn increase_stake_amount(state: &mut StateInfo, staker_info: &mut StakerInfo, amount: Uint128) {
    state.total_stake_amount += amount;
    staker_info.stake_amount += amount;
}

fn decrease_stake_amount(state: &mut StateInfo, staker_info: &mut StakerInfo, amount: Uint128) {
    staker_info.stake_amount -= amount;
    state.total_stake_amount -= amount;
}

fn compute_state_extra(config: &Config, state: &mut StateInfo, timestamp: u64) {
    let s = config.distribution_schedule;

    if timestamp <= s.0 {
        state.leftover = s.2;
        state.reward_rate_per_token = Decimal::zero();
    }
    else if timestamp >= s.1 {
        state.leftover = Uint128::zero();
        state.reward_rate_per_token = Decimal::zero();
    }
    else {
        let duration = s.1 - s.0;
        let distribution_rate: Decimal = Decimal::from_ratio(s.2, duration);
        let time_left = s.1 - timestamp;
        state.leftover = distribution_rate * Uint128::from(time_left as u128);
        if state.total_stake_amount.is_zero() {
            state.reward_rate_per_token = Decimal::zero();
        } else {
            let denom = Uint128::from(10u128.pow(config.staking_token_decimals as u32));
            state.reward_rate_per_token =
                Decimal::from_ratio(distribution_rate * denom, state.total_stake_amount);
        }
    }
}

fn compute_reward(config: &Config, state: &mut StateInfo, timestamp: u64) {
    compute_state_extra(config, state, timestamp);

    if state.total_stake_amount.is_zero() {
        state.last_distributed = timestamp;
        return;
    }

    let mut distributed_amount: Uint128 = Uint128::zero();
    let s = config.distribution_schedule;
    if s.0 < timestamp && s.1 > state.last_distributed {
        let time_passed =
            std::cmp::min(s.1, timestamp) - std::cmp::max(s.0, state.last_distributed);
        let duration = s.1 - s.0;
        let distribution_rate: Decimal = Decimal::from_ratio(s.2, duration);
        distributed_amount += distribution_rate * Uint128::from(time_passed as u128);
    }

    state.last_distributed = timestamp;
    state.global_reward_index = state.global_reward_index
        + Decimal::from_ratio(distributed_amount, state.total_stake_amount);
}

fn compute_staker_reward(state: &StateInfo, staker_info: &mut StakerInfo) -> StdResult<()> {
    let pending_reward = (staker_info.stake_amount * state.global_reward_index)
        - (staker_info.stake_amount * staker_info.reward_index);
    staker_info.reward_index = state.global_reward_index;
    staker_info.pending_reward += pending_reward;
    Ok(())
}

fn compute_referral_reward(
    staker_info: &StakerInfo,
    referral_info: &mut StakerInfo,
    referral_rate: u64,
    staked_amount: Uint128,
    timestamp: u64,
) -> StdResult<()> {
    let referral_reward = staked_amount
        .checked_mul(Uint128::from(referral_rate))?
        .checked_div(Uint128::from(100u64))
        .unwrap_or_default();

    referral_info.referral_reward += referral_reward;
    if staker_info.referral_addr.to_string().is_empty() {
        referral_info.referral_count += 1u64;
    }
    if referral_info.referral_timestamp == 0 {
        referral_info.referral_timestamp = timestamp;
    }
    Ok(())
}

fn build_send_cw20_token_msg(
    recipient: Addr,
    token_contract_address: Addr,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_contract_address.into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.into(),
            amount,
        })?,
        funds: vec![],
    }))
}
