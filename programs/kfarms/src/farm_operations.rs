use crate::state::{
    LockingMode, RewardPerTimeUnitPoint, RewardScheduleCurve, RewardType, TimeUnit,
};
use crate::types::{
    AddRewardEffects, HarvestEffects, StakeEffects, WithdrawEffects, WithdrawRewardEffects,
};
use crate::utils::consts::BPS_DIV_FACTOR;
use crate::utils::math::{ten_pow, u64_mul_div};
use crate::xmsg;
use crate::{
    dbg_msg, stake_operations as stake_ops, utils::consts::MAX_REWARDS_TOKENS, FarmConfigOption,
    FarmError, FarmState, GlobalConfig, GlobalConfigOption, RewardInfo, UserState,
};
use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use decimal_wad::decimal::Decimal;
use num_enum::TryFromPrimitive;
use std::cmp;
use std::ops::{AddAssign, SubAssign};

pub use scope::{DatedPrice, OraclePrices};

pub fn update_global_config(
    global_config: &mut GlobalConfig,
    key: GlobalConfigOption,
    value: &[u8],
) -> Result<()> {
    match key {
        GlobalConfigOption::SetPendingGlobalAdmin => {
            let value: [u8; 32] = value[0..32].try_into().unwrap();
            let pubkey = Pubkey::new_from_array(value);
            xmsg!(
                "Changing global_config admin {} -> {:?}",
                global_config.global_admin,
                pubkey
            );
            global_config.pending_global_admin = pubkey;
        }
        GlobalConfigOption::SetTreasuryFeeBps => {
            let value = u64::from_le_bytes(value[..8].try_into().unwrap());
            if value > 10_000 {
                xmsg!("ERROR: treasury_fee_bps must be <= 10000");
                return Err(FarmError::InvalidConfigValue.into());
            }
            xmsg!(
                "Changing global_config treasury_fee_bps {} -> {:?}",
                global_config.treasury_fee_bps,
                value
            );
            global_config.treasury_fee_bps = value;
        }
    }
    Ok(())
}

pub fn initialize_reward(
    farm_state: &mut FarmState,
    reward_vault: Pubkey,
    mint: Pubkey,
    mint_decimals: u8,
    mint_token_program: Pubkey,
    ts: u64,
) -> Result<()> {
    if farm_state.num_reward_tokens == MAX_REWARDS_TOKENS as u64 {
        return Err(FarmError::MaxRewardNumberReached.into());
    }

    let reward_info = &mut farm_state.reward_infos[farm_state.num_reward_tokens as usize];

    reward_info.rewards_available = 0;
    reward_info.rewards_vault = reward_vault;

    reward_info.token.mint = mint;
    reward_info.token.decimals = mint_decimals as u64;
    reward_info.token.token_program = mint_token_program;

    reward_info.last_issuance_ts = ts;

    farm_state.num_reward_tokens = farm_state
        .num_reward_tokens
        .checked_add(1)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    Ok(())
}

pub fn add_reward(
    farm_state: &mut FarmState,
    scope_price: Option<DatedPrice>,
    mint: Pubkey,
    reward_index: usize,
    amount: u64,
    ts: u64,
) -> Result<AddRewardEffects> {
    xmsg!("farm_operations::add_reward amount={}", amount);
    refresh_global_rewards(farm_state, scope_price, ts)?;

    let reward = &mut farm_state.reward_infos[reward_index];
    require!(reward.token.mint == mint, FarmError::RewardDoesNotExist);

    reward.rewards_available = reward
        .rewards_available
        .checked_add(amount)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    Ok(AddRewardEffects {
        reward_amount: amount,
    })
}

pub fn withdraw_reward(
    farm_state: &mut FarmState,
    scope_price: Option<DatedPrice>,
    mint: &Pubkey,
    reward_index: usize,
    amount: u64,
    ts: u64,
) -> Result<WithdrawRewardEffects> {
    xmsg!("farm_operations::withdraw_reward amount={}", amount);
    require!(amount > 0, FarmError::RewardDoesNotExist);
    refresh_global_rewards(farm_state, scope_price, ts)?;

    let reward = &mut farm_state.reward_infos[reward_index];
    require!(reward.token.mint == *mint, FarmError::RewardDoesNotExist);
    require!(
        reward.rewards_available > 0,
        FarmError::WithdrawRewardZeroAvailable
    );
    require!(
        reward.reward_schedule_curve == RewardScheduleCurve::default(),
        FarmError::RewardScheduleCurveSet
    );

    let max_withdrawable = cmp::min(reward.rewards_available, amount);

    reward.rewards_available -= max_withdrawable;

    Ok(WithdrawRewardEffects {
        reward_amount: max_withdrawable,
    })
}

pub fn update_farm_config(
    farm_state: &mut FarmState,
    scope_price: Option<DatedPrice>,
    mode: FarmConfigOption,
    data: &[u8],
) -> Result<()> {
    xmsg!(
        "farm_operations::update_farm_config mode={:?} with data of len {}",
        mode,
        data.len()
    );
    match mode {
        FarmConfigOption::UpdateRewardRps
        | FarmConfigOption::UpdateRewardMinClaimDuration
        | FarmConfigOption::RewardType
        | FarmConfigOption::RpsDecimals
        | FarmConfigOption::UpdateRewardScheduleCurvePoints => {
            let reward_index: u64 = BorshDeserialize::try_from_slice(&data[..8])?;
            require!(
                reward_index < farm_state.num_reward_tokens,
                FarmError::RewardIndexOutOfRange
            );

            let time_unit = farm_state.time_unit;

            refresh_global_rewards(
                farm_state,
                scope_price,
                TimeUnit::now_from_clock(time_unit, &Clock::get()?),
            )?;

            let reward_info = &mut farm_state.reward_infos[reward_index as usize];

            require!(reward_info.is_initialised(), FarmError::NoRewardInList);
            xmsg!("Updating reward index={}", reward_index);
            update_reward_config(
                reward_info,
                mode,
                &data[8..],
                TimeUnit::now_from_clock(time_unit, &Clock::get()?),
            )?;
        }
        FarmConfigOption::WithdrawAuthority => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config withdraw_authority={pubkey}",);
            xmsg!("prev value {:?}", farm_state.withdraw_authority);
            farm_state.withdraw_authority = pubkey;
        }
        FarmConfigOption::DepositWarmupPeriod => {
            if farm_state.is_delegated() {
                xmsg!("farm_operations::update_farm_config ERROR: delegated farm cannot change deposit_warmup_period");
                return err!(FarmError::FarmDelegated);
            }
            let value: u32 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config deposit_warmup_period={value}",);
            xmsg!("prev value {:?}", farm_state.deposit_warmup_period);
            farm_state.deposit_warmup_period = value;
        }
        FarmConfigOption::WithdrawCooldownPeriod => {
            if farm_state.is_delegated() {
                xmsg!("farm_operations::update_farm_config ERROR: delegated farm cannot change withdrawal_cooldown_period");
                return err!(FarmError::FarmDelegated);
            }
            let value: u32 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config withdrawal_cooldown_period={value}",);
            xmsg!("prev value {:?}", farm_state.withdrawal_cooldown_period);
            farm_state.withdrawal_cooldown_period = value;
        }
        FarmConfigOption::LockingMode => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config locking_mode={value}",);
            xmsg!("prev value {:?}", farm_state.locking_mode);
            farm_state.locking_mode = value;
            LockingMode::try_from_primitive(value).unwrap();
        }
        FarmConfigOption::LockingStartTimestamp => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config locking_start_timestamp={value}",);
            xmsg!("prev value {:?}", farm_state.locking_start_timestamp);
            farm_state.locking_start_timestamp = value;
        }
        FarmConfigOption::LockingEarlyWithdrawalPenaltyBps => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            require_gte!(10000, value, FarmError::InvalidConfigValue);
            xmsg!(
                "farm_operations::update_farm_config locking_early_withdrawal_penalty_bps={value}",
            );
            xmsg!(
                "prev value {:?}",
                farm_state.locking_early_withdrawal_penalty_bps
            );
            farm_state.locking_early_withdrawal_penalty_bps = value;
        }
        FarmConfigOption::LockingDuration => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config locking_duration={value}",);
            xmsg!("prev value {:?}", farm_state.locking_duration);
            farm_state.locking_duration = value;
        }
        FarmConfigOption::DepositCapAmount => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config deposit_cap_amount={value}",);
            xmsg!("prev value {:?}", farm_state.deposit_cap_amount);
            farm_state.deposit_cap_amount = value;
        }
        FarmConfigOption::SlashedAmountSpillAddress => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config slashed_amount_spill_address={pubkey}",);
            xmsg!("prev value {:?}", farm_state.slashed_amount_spill_address);
            farm_state.slashed_amount_spill_address = pubkey;
        }
        FarmConfigOption::ScopePricesAccount => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config scope_prices_account={pubkey}",);
            xmsg!("prev value {:?}", farm_state.scope_prices);
            farm_state.scope_prices = pubkey;
        }
        FarmConfigOption::ScopeOraclePriceId => {
            let value: u16 = BorshDeserialize::try_from_slice(&data[..2])?;
            xmsg!("farm_operations::update_farm_config scope_oracle_price_id={value}",);
            xmsg!("prev value {:?}", farm_state.scope_oracle_price_id);
            farm_state.scope_oracle_price_id = value.into();
        }
        FarmConfigOption::ScopeOracleMaxAge => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config scope_oracle_max_age={value}",);
            xmsg!("prev value {:?}", farm_state.scope_oracle_max_age);
            farm_state.scope_oracle_max_age = value;
        }
        FarmConfigOption::UpdatePendingFarmAdmin => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config farm_admin={pubkey}",);
            xmsg!("prev value {:?}", farm_state.pending_farm_admin);
            farm_state.pending_farm_admin = pubkey;
        }
        FarmConfigOption::UpdateStrategyId => {
            let pubkey: Pubkey = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config strategy_id={pubkey}",);
            xmsg!("prev value {:?}", farm_state.strategy_id);
            farm_state.strategy_id = pubkey;
        }
    };
    Ok(())
}

pub(crate) fn update_reward_config(
    reward_info: &mut RewardInfo,
    mode: FarmConfigOption,
    data: &[u8],
    ts: u64,
) -> Result<()> {
    match mode {
        FarmConfigOption::UpdateRewardRps => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config reward_rps={value} last_issuance_ts={ts}",);
            xmsg!("prev value {:?}", reward_info.reward_schedule_curve);
            reward_info.reward_schedule_curve.set_constant(value);
        }
        FarmConfigOption::UpdateRewardMinClaimDuration => {
            let value: u64 = BorshDeserialize::try_from_slice(data)?;
            xmsg!("farm_operations::update_farm_config reward_min_claim_duration={value}",);
            xmsg!("prev value {}", reward_info.min_claim_duration_seconds);
            reward_info.min_claim_duration_seconds = value
        }
        FarmConfigOption::RewardType => {
            let value: u8 = BorshDeserialize::try_from_slice(&data[..1])?;
            xmsg!(
                "farm_operations::update_farm_config reward_type={value} type={:?}",
                RewardType::try_from_primitive(value).unwrap()
            );
            xmsg!("prev value {:?}", reward_info.reward_type);
            reward_info.reward_type = value;
        }
        FarmConfigOption::RpsDecimals => {
            let value: u8 = BorshDeserialize::try_from_slice(&data[..1])?;
            xmsg!("farm_operations::update_farm_config rps_decimals={value}",);
            xmsg!("prev value {}", reward_info.rewards_per_second_decimals);
            reward_info.rewards_per_second_decimals = value;
        }
        FarmConfigOption::UpdateRewardScheduleCurvePoints => {
            let points: Vec<RewardPerTimeUnitPoint> = BorshDeserialize::try_from_slice(data)?;

            xmsg!("Updating reward schedule curve with points={:?}", points);
            xmsg!("Prev value {:?}", reward_info.reward_schedule_curve.points);
            reward_info.reward_schedule_curve = RewardScheduleCurve::from_points(&points).unwrap();
        }
        _ => unimplemented!(),
    }

    reward_info.last_issuance_ts = ts;
    Ok(())
}

pub fn initialize_user(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    owner_key: &Pubkey,
    farm_state_key: &Pubkey,
    ts: u64,
) -> Result<()> {
    user_state.owner = *owner_key;
    user_state.farm_state = *farm_state_key;

    let user_id = farm_state.num_users;

    user_state.user_id = user_id;

    user_state.rewards_tally_scaled = [0; MAX_REWARDS_TOKENS];
    user_state.rewards_issued_unclaimed = [0; MAX_REWARDS_TOKENS];
    user_state.active_stake_scaled = 0;
    user_state.last_claim_ts = [ts; MAX_REWARDS_TOKENS];

    if farm_state.is_delegated() {
        user_state.is_farm_delegated = true as u8;
    }

    farm_state.num_users = farm_state
        .num_users
        .checked_add(1)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    Ok(())
}

pub fn initialize_reward_ts_if_needed(farm_state: &mut FarmState, current_ts: u64) {
    if farm_state.total_staked_amount == 0 {
        for reward_info in farm_state
            .reward_infos
            .iter_mut()
            .take(farm_state.num_reward_tokens as usize)
        {
            reward_info.last_issuance_ts = current_ts;
        }
    }
}

pub fn stake(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    scope_price: Option<DatedPrice>,
    amount: u64,
    current_ts: u64,
) -> Result<StakeEffects> {
    xmsg!("farm_operations::stake amount={}", amount);
    refresh_global_rewards(farm_state, scope_price, current_ts)?;
    user_refresh_all_rewards(farm_state, user_state)?;
    user_refresh_stake(farm_state, user_state, current_ts)?;

    if !farm_state.can_accept_deposit(amount, scope_price, current_ts)? {
        return Err(FarmError::DepositCapReached.into());
    }

    if user_state.pending_deposit_stake_scaled != 0 {
        xmsg!(
            "farm_operations::stake BEFORE: pending_user_stake_scaled={}, pending_user_stake_ts={},\
             pending stake will be extended",
            user_state.pending_deposit_stake_scaled,
            user_state.pending_deposit_stake_ts
        );
    }

    if farm_state.deposit_warmup_period > 0 {
        user_state.pending_deposit_stake_ts = current_ts
            .checked_add(farm_state.deposit_warmup_period.into())
            .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;
        let stake_gained = stake_ops::add_pending_deposit_stake(user_state, farm_state, amount)?;
        xmsg!(
            "farm_operations::stake AFTER: pending_user_stake_ts={},\
             pending_stake_gained={}",
            user_state.pending_deposit_stake_ts,
            stake_gained
        );
    } else {
        let stake_gained = stake_ops::add_active_stake(user_state, farm_state, amount)?;
        xmsg!(
            "farm_operations::stake AFTER: active_user_stake_scaled={}, active_stake_gained={}",
            user_state.active_stake_scaled,
            stake_gained
        );
        update_user_rewards_tally_on_stake_increase(farm_state, user_state, stake_gained)?;
    };

    user_state.last_stake_ts = current_ts;

    Ok(StakeEffects {
        amount_to_stake: amount,
    })
}

pub fn set_stake(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    new_stake: u64,
    ts: u64,
) -> Result<()> {
    assert_eq!(
        farm_state.total_active_stake_scaled,
        u128::from(farm_state.total_staked_amount)
    );
    assert_eq!(farm_state.total_pending_stake_scaled, 0);
    assert_eq!(farm_state.total_pending_amount, 0);
    assert_eq!(farm_state.deposit_warmup_period, 0);
    assert_eq!(farm_state.withdrawal_cooldown_period, 0);

    let current_stake_amount: u64 = user_state
        .active_stake_scaled
        .try_into()
        .expect("Delegated farm: active stake don't fit on u64");

    if current_stake_amount == new_stake {
        xmsg!("farm_operations::set_stake nothing to do");
        return Ok(());
    }

    refresh_global_rewards(farm_state, None, ts)?;
    user_refresh_all_rewards(farm_state, user_state)?;

    type OpAssignU64 = dyn Fn(&mut u64, u64);
    type OpAssignU128 = dyn Fn(&mut u128, u128);

    let (diff, op_u64, op_u128): (u64, &OpAssignU64, &OpAssignU128) =
        if current_stake_amount > new_stake {
            let diff = current_stake_amount - new_stake;

            (diff, &u64::sub_assign, &u128::sub_assign)
        } else {
            let diff = new_stake - current_stake_amount;
            initialize_reward_ts_if_needed(farm_state, ts);
            user_state.last_stake_ts = ts;

            if !farm_state.can_accept_deposit(diff, None, ts)? {
                return Err(FarmError::DepositCapReached.into());
            }

            (diff, &u64::add_assign, &u128::add_assign)
        };
    let diff_u128 = u128::from(diff);

    op_u128(&mut farm_state.total_active_stake_scaled, diff_u128);
    op_u64(&mut farm_state.total_staked_amount, diff);

    op_u128(&mut user_state.active_stake_scaled, diff_u128);

    for i in 0..farm_state.num_reward_tokens as usize {
        let reward_tally = &mut user_state.rewards_tally_scaled[i];
        let reward_info = &farm_state.reward_infos[i];

        *reward_tally = reward_info.reward_per_share_scaled * u128::from(new_stake);
    }

    Ok(())
}

pub fn harvest(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    global_config: &GlobalConfig,
    scope_price: Option<DatedPrice>,
    reward_index: usize,
    ts: u64,
) -> Result<HarvestEffects> {
    xmsg!("farm_operations::harvest reward_index={}", reward_index);
    refresh_global_rewards(farm_state, scope_price, ts)?;
    user_refresh_reward(farm_state, user_state, reward_index)?;

    let reward = user_state.rewards_issued_unclaimed[reward_index];
    require!(
        ts.checked_sub(user_state.last_claim_ts[reward_index])
            .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?
            >= farm_state.reward_infos[reward_index].min_claim_duration_seconds,
        FarmError::MinClaimDurationNotReached
    );
    if reward == 0 {
        return Ok(HarvestEffects {
            reward_treasury: 0,
            reward_user: 0,
        });
    }

    farm_state.reward_infos[reward_index].rewards_issued_unclaimed = farm_state.reward_infos
        [reward_index]
        .rewards_issued_unclaimed
        .checked_sub(reward)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;
    user_state.rewards_issued_unclaimed[reward_index] = 0;
    user_state.last_claim_ts[reward_index] = ts;

    let reward_treasury = u64_mul_div(reward, global_config.treasury_fee_bps, BPS_DIV_FACTOR);
    let reward_user = reward
        .checked_sub(reward_treasury)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    Ok(HarvestEffects {
        reward_user,
        reward_treasury,
    })
}

pub fn user_refresh_reward(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    reward_index: usize,
) -> Result<()> {
    xmsg!(
        "farm_operations::user_refresh_reward reward_index {} Global stake {} User stake {} prev_reward_tally {} rpt {}",
        reward_index,
        farm_state.total_active_stake_scaled,
        user_state.active_stake_scaled,
        user_state.rewards_tally_scaled[reward_index],
        farm_state.reward_infos[reward_index].reward_per_share_scaled,
    );

    let rewards_tally = user_state.get_rewards_tally_decimal(reward_index);
    let reward_per_share = farm_state.reward_infos[reward_index].get_reward_per_share_decimal();

    let new_reward_tally: Decimal = if farm_state.is_delegated() {
        reward_per_share * user_state.active_stake_scaled
    } else {
        reward_per_share * user_state.get_active_stake_decimal()
    };

    let reward: u64 = (new_reward_tally - rewards_tally)
        .try_floor()
        .map_err(|_| dbg_msg!(FarmError::IntegerOverflow))?;

    let new_reward_tally = rewards_tally + reward.into();

    xmsg!(
        "farm_operations::user_refresh_reward reward {}, new_reward_tally (scaled) {}",
        reward,
        new_reward_tally.to_scaled_val::<u128>().unwrap()
    );

    user_state.set_rewards_tally_decimal(reward_index, new_reward_tally);

    user_state.rewards_issued_unclaimed[reward_index] += reward;

    Ok(())
}

pub fn user_refresh_all_rewards(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
) -> Result<()> {
    if user_state.active_stake_scaled > 0 {
        for reward_index in 0..farm_state.num_reward_tokens as usize {
            user_refresh_reward(farm_state, user_state, reward_index)?;
        }
    }

    Ok(())
}

fn user_refresh_stake(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    current_ts: u64,
) -> Result<()> {
    initialize_reward_ts_if_needed(farm_state, current_ts);

    if user_state.pending_deposit_stake_scaled > 0
        && current_ts >= user_state.pending_deposit_stake_ts
    {
        let (amount_staked, active_stake_gained) =
            stake_ops::activate_pending_stake(user_state, farm_state)?;
        xmsg!(
            "farm_operations::user_refresh_stake amount_staked {} active_stake_gained (scaled) {}",
            amount_staked,
            active_stake_gained.to_scaled_val::<u128>().unwrap()
        );

        update_user_rewards_tally_on_stake_increase(farm_state, user_state, active_stake_gained)?;
    }
    Ok(())
}

pub fn user_refresh_state(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    scope_price: Option<DatedPrice>,
    current_ts: u64,
) -> Result<()> {
    refresh_global_rewards(farm_state, scope_price, current_ts)?;
    user_refresh_all_rewards(farm_state, user_state)?;

    let is_delegated = farm_state.is_delegated();

    user_state.is_farm_delegated = is_delegated as u8;

    if !farm_state.is_delegated() {
        user_refresh_stake(farm_state, user_state, current_ts)?;
    }

    Ok(())
}

pub fn reward_user_once(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    reward_index: u64,
    amount: u64,
) -> Result<()> {
    farm_state.reward_infos[reward_index as usize].rewards_issued_unclaimed += amount;
    farm_state.reward_infos[reward_index as usize].rewards_issued_cumulative += amount;
    user_state.rewards_issued_unclaimed[reward_index as usize] += amount;
    Ok(())
}

pub fn unstake(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    scope_price: Option<DatedPrice>,
    requested_stake_withdrawal: Decimal,
    ts: u64,
) -> Result<()> {
    xmsg!(
        "farm_operations::unstake amount of stake={}",
        requested_stake_withdrawal
    );

    refresh_global_rewards(farm_state, scope_price, ts)?;

    user_refresh_all_rewards(farm_state, user_state)?;

    let stake_share_to_unstake = std::cmp::min(
        requested_stake_withdrawal,
        user_state.get_active_stake_decimal(),
    );
    require!(
        stake_share_to_unstake > Decimal::zero(),
        FarmError::NothingToUnstake
    );

    if user_state.pending_withdrawal_unstake_scaled > 0 {
        if user_state.pending_withdrawal_unstake_ts <= ts {
            xmsg!("farm_operations::unstake pending withdrawal elapsed already exist but not withdrawn yet");
            return err!(FarmError::PendingWithdrawalNotWithdrawnYet);
        }
        xmsg!(
            "farm_operations::unstake pending withdrawal already exist and will be extended.\
                already pending withdrawal stake={}, added={}, old ts={}",
            user_state.get_pending_withdrawal_unstake_decimal(),
            stake_share_to_unstake,
            user_state.pending_withdrawal_unstake_ts
        );
    }

    user_state.pending_withdrawal_unstake_ts = ts
        .checked_add(farm_state.withdrawal_cooldown_period.into())
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    let (token_amount_removed, added_pending_withdrawal_unstake, token_amount_penalty) =
        stake_ops::unstake(user_state, farm_state, stake_share_to_unstake, ts)?;

    xmsg!(
        "farm_operations::unstake added_pending_withdrawal_unstake={}, token_amount_unstaked={}",
        added_pending_withdrawal_unstake,
        token_amount_removed
    );

    farm_state.slashed_amount_current += token_amount_penalty;
    farm_state.slashed_amount_cumulative += token_amount_penalty;

    for i in 0..farm_state.num_reward_tokens as usize {
        let reward_tally = &mut user_state.rewards_tally_scaled[i];
        let reward_info = &farm_state.reward_infos[i];

        let reward_tally_decimal = Decimal::from_scaled_val(*reward_tally);
        let tally_loss = stake_share_to_unstake * reward_info.get_reward_per_share_decimal();

        require_gt!(
            reward_tally_decimal + Decimal::one(),
            tally_loss,
            FarmError::IntegerOverflow
        );
        let reward_tally_scaled: u128 = reward_tally_decimal.to_scaled_val().unwrap();
        let tally_loss_scaled: u128 = tally_loss.to_scaled_val().unwrap();
        let new_reward_tally_decimal_scaled = reward_tally_scaled.saturating_sub(tally_loss_scaled);

        *reward_tally = new_reward_tally_decimal_scaled;
    }

    Ok(())
}

pub fn withdraw_unstaked_deposits(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    ts: u64,
) -> Result<WithdrawEffects> {
    require!(
        user_state.pending_withdrawal_unstake_ts <= ts,
        FarmError::UnstakeNotElapsed
    );
    require!(
        user_state.pending_withdrawal_unstake_scaled > 0,
        FarmError::NothingToWithdraw
    );

    let amount_to_withdraw = stake_ops::remove_pending_withdrawal_stake(user_state, farm_state)?;

    Ok(WithdrawEffects { amount_to_withdraw })
}

pub fn refresh_global_reward(
    farm_state: &mut FarmState,
    scope_price: Option<DatedPrice>,
    ts: u64,
    reward_index: usize,
) -> Result<()> {
    let reward_info = farm_state.reward_infos[reward_index];

    if ts == reward_info.last_issuance_ts {
        return Ok(());
    }

    if farm_state.total_active_stake_scaled == 0 {
        farm_state.reward_infos[reward_index].last_issuance_ts = ts;
        return Ok(());
    }

    let amount: u64 = {
        let cumulative_amt = (reward_info
            .reward_schedule_curve
            .get_cumulative_amount_issued_since_last_ts(reward_info.last_issuance_ts, ts)?)
            as u128;

        let reward_type_amt = match reward_info.reward_type() {
            RewardType::Proportional => cumulative_amt,
            RewardType::Constant => cumulative_amt * u128::from(farm_state.total_staked_amount),
        };

        let decimal_adjusted_amt =
            reward_type_amt / u128::from(ten_pow(reward_info.rewards_per_second_decimals.into()));

        let oracle_adjusted_amt = if farm_state.scope_oracle_price_id == u64::MAX {
            decimal_adjusted_amt
        } else {
            let price = scope_price.ok_or(FarmError::MissingScopePrices)?;
            if ts - price.unix_timestamp > farm_state.scope_oracle_max_age {
                xmsg!(
                    "ts={} price_ts={} max_age={}",
                    ts,
                    price.unix_timestamp,
                    farm_state.scope_oracle_max_age
                );
                return Err(FarmError::ScopeOraclePriceTooOld.into());
            } else {
                xmsg!("Price: {:?}", price);
                let decimal_adjusted_amt = decimal_adjusted_amt as u128;
                let px = price.price.value as u128;
                let factor = ten_pow(price.price.exp as usize) as u128;
                decimal_adjusted_amt * px / factor
            }
        };

        xmsg!(
            "time_passed={} reward_type={:?} cumulative_amt={} decimal_adjusted_amt={} oracle_adjusted_amt={} ",
            ts - reward_info.last_issuance_ts,
            reward_info.reward_type(),
            cumulative_amt,
            decimal_adjusted_amt,
            oracle_adjusted_amt,
        );

        oracle_adjusted_amt.try_into().unwrap()
    };

    if amount == 0 {
        return Ok(());
    }

    let rewards = cmp::min(amount, reward_info.rewards_available);

    xmsg!(
        "farm_operations::refresh_global_reward issuing_reward={} last_ts={} ts={}",
        rewards,
        reward_info.last_issuance_ts,
        ts
    );

    farm_state.reward_infos[reward_index].last_issuance_ts = ts;

    farm_state.reward_infos[reward_index].rewards_issued_unclaimed = farm_state.reward_infos
        [reward_index]
        .rewards_issued_unclaimed
        .checked_add(rewards)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    farm_state.reward_infos[reward_index].rewards_issued_cumulative = farm_state.reward_infos
        [reward_index]
        .rewards_issued_cumulative
        .checked_add(rewards)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    farm_state.reward_infos[reward_index].rewards_available = farm_state.reward_infos[reward_index]
        .rewards_available
        .checked_sub(rewards)
        .ok_or_else(|| dbg_msg!(FarmError::IntegerOverflow))?;

    {
        let mut reward_per_share =
            farm_state.reward_infos[reward_index].get_reward_per_share_decimal();

        let added_reward_per_share = if farm_state.is_delegated() {
            Decimal::from(rewards) / farm_state.total_active_stake_scaled
        } else {
            Decimal::from(rewards) / farm_state.get_total_active_stake_decimal()
        };

        reward_per_share = reward_per_share + added_reward_per_share;

        farm_state.reward_infos[reward_index].set_reward_per_share_decimal(reward_per_share);
    }

    Ok(())
}

pub fn refresh_global_rewards(
    farm_state: &mut FarmState,
    scope_price: Option<DatedPrice>,
    ts: u64,
) -> Result<()> {
    xmsg!("farm_operations::refresh_global_rewards ts={}", ts);

    for reward_index in 0..farm_state.num_reward_tokens as usize {
        refresh_global_reward(farm_state, scope_price, ts, reward_index)?;
    }

    Ok(())
}

pub fn deposit_to_farm_vault(farm_state: &mut FarmState, amount: u64) -> Result<()> {
    xmsg!("farm_operations::deposit_to_farm_vault amount={}", amount);
    stake_ops::increase_total_amount(farm_state, amount).map_err(Into::into)
}

pub fn withdraw_from_farm_vault(farm_state: &mut FarmState, amount: u64) -> Result<u64> {
    xmsg!(
        "farm_operations::withdraw_from_farm_vault amount={}",
        amount
    );
    let res = stake_ops::withdraw_farm(farm_state, amount)?;

    if res.farm_to_freeze {
        farm_state.is_farm_frozen = true as u8;
    }

    Ok(res.amount_to_withdraw)
}

pub fn withdraw_slashed_amount(farm_state: &mut FarmState) -> Result<u64> {
    let amount = farm_state.slashed_amount_current;
    farm_state.slashed_amount_current = 0;
    Ok(amount)
}
fn update_user_rewards_tally_on_stake_increase(
    farm_state: &mut FarmState,
    user_state: &mut UserState,
    added_shares: Decimal,
) -> Result<()> {
    if added_shares == Decimal::zero() {
        return Ok(());
    }

    xmsg!(
        "farm_operations::update_user_rewards_tally_on_stake_increase amount(scaled)={}",
        added_shares.to_scaled_val::<u128>().unwrap()
    );
    for index in 0..farm_state.num_reward_tokens as usize {
        let reward_info = farm_state.reward_infos[index];

        let rewards_tally = user_state.get_rewards_tally_decimal(index);
        let reward_per_share = reward_info.get_reward_per_share_decimal();

        let new_reward_tally: Decimal = rewards_tally + (added_shares * reward_per_share);

        user_state.rewards_tally_scaled[index] = new_reward_tally
            .to_scaled_val()
            .map_err(|_| dbg_msg!(FarmError::IntegerOverflow))?;

        xmsg!(
            "farm_operations::update_user_rewards_tally_on_stake_increase reward_index={} new_reward_tally(scaled)={}",
            index,
            new_reward_tally.to_scaled_val::<u128>().unwrap()
        );
    }

    Ok(())
}
