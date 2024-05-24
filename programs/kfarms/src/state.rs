#![allow(clippy::derivable_impls)]

use crate::{
    utils::{consts::REWARD_CURVE_POINTS, math::ten_pow},
    xmsg,
};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use scope::DatedPrice;

use crate::{
    utils::consts::{self, MAX_REWARDS_TOKENS},
    FarmError,
};
use anchor_lang::prelude::Pubkey;
use decimal_wad::decimal::Decimal;
use num_enum::{IntoPrimitive, TryFromPrimitive};

static_assertions::const_assert_eq!(
    consts::SIZE_GLOBAL_CONFIG,
    std::mem::size_of::<GlobalConfig>() + 8
);
static_assertions::const_assert_eq!(0, std::mem::size_of::<GlobalConfig>() % 8);
#[account(zero_copy)]
#[derive(Debug)]
pub struct GlobalConfig {
    pub global_admin: Pubkey,

    pub treasury_fee_bps: u64,

    pub treasury_vaults_authority: Pubkey,
    pub treasury_vaults_authority_bump: u64,

    pub pending_global_admin: Pubkey,

    pub _padding1: [u128; 126],
}

impl Default for GlobalConfig {
    fn default() -> GlobalConfig {
        GlobalConfig {
            global_admin: Pubkey::default(),

            treasury_vaults_authority: Pubkey::default(),
            treasury_vaults_authority_bump: 0,
            treasury_fee_bps: 0,
            pending_global_admin: Pubkey::default(),
            _padding1: [0; 126],
        }
    }
}

#[derive(
    AnchorSerialize, AnchorDeserialize, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug,
)]
#[repr(u8)]
pub enum GlobalConfigOption {
    SetPendingGlobalAdmin = 0,
    SetTreasuryFeeBps = 1,
}

static_assertions::const_assert_eq!(0, std::mem::size_of::<FarmState>() % 8);
#[account(zero_copy)]
#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct FarmState {
    pub farm_admin: Pubkey,
    pub global_config: Pubkey,

    pub token: TokenInfo,
    pub reward_infos: [RewardInfo; MAX_REWARDS_TOKENS],
    pub num_reward_tokens: u64,

    pub num_users: u64,
    pub total_staked_amount: u64,

    pub farm_vault: Pubkey,
    pub farm_vaults_authority: Pubkey,
    pub farm_vaults_authority_bump: u64,

    pub delegate_authority: Pubkey,

    pub time_unit: u8,

    pub is_farm_frozen: u8,

    pub is_farm_delegated: u8,

    pub _padding0: [u8; 5],

    pub withdraw_authority: Pubkey,

    pub deposit_warmup_period: u32,
    pub withdrawal_cooldown_period: u32,

    pub total_active_stake_scaled: u128,
    pub total_pending_stake_scaled: u128,

    pub total_pending_amount: u64,

    pub slashed_amount_current: u64,
    pub slashed_amount_cumulative: u64,
    pub slashed_amount_spill_address: Pubkey,

    pub locking_mode: u64,
    pub locking_start_timestamp: u64,
    pub locking_duration: u64,
    pub locking_early_withdrawal_penalty_bps: u64,

    pub deposit_cap_amount: u64,

    pub scope_prices: Pubkey,
    pub scope_oracle_price_id: u64,
    pub scope_oracle_max_age: u64,

    pub pending_farm_admin: Pubkey,
    pub strategy_id: Pubkey,
    pub _padding: [u64; 86],
}

impl FarmState {
    pub fn get_total_active_stake_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.total_active_stake_scaled)
    }

    pub fn get_total_pending_stake_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.total_pending_stake_scaled)
    }

    pub fn set_total_active_stake_decimal(&mut self, value: Decimal) {
        self.total_active_stake_scaled = value.to_scaled_val().unwrap();
    }

    pub fn set_total_pending_stake_decimal(&mut self, value: Decimal) {
        self.total_pending_stake_scaled = value.to_scaled_val().unwrap();
    }

    pub fn is_delegated(&self) -> bool {
        self.delegate_authority != Pubkey::default()
    }

    pub fn get_locking_mode(&self) -> LockingMode {
        LockingMode::try_from(self.locking_mode).unwrap()
    }

    pub fn can_accept_deposit(
        &self,
        amount: u64,
        scope_price: Option<DatedPrice>,
        ts: u64,
    ) -> Result<bool> {
        let unadjusted_total = self.total_staked_amount + amount;
        let final_amount = if self.scope_oracle_price_id == u64::MAX {
            unadjusted_total
        } else {
            let price = scope_price.ok_or(FarmError::MissingScopePrices)?;
            if ts - price.unix_timestamp > self.scope_oracle_max_age {
                xmsg!(
                    "ts={} price_ts={} max_age={}",
                    ts,
                    price.unix_timestamp,
                    self.scope_oracle_max_age
                );
                return Err(FarmError::ScopeOraclePriceTooOld.into());
            } else {
                xmsg!("Price: {:?}", price);
                let unadjusted_total = u128::from(unadjusted_total);
                let price_value = u128::from(price.price.value);
                let price_ten_pow = u128::from(ten_pow(price.price.exp as usize));
                (unadjusted_total * price_value / price_ten_pow)
                    .try_into()
                    .unwrap()
            }
        };
        Ok(self.deposit_cap_amount == 0 || final_amount <= self.deposit_cap_amount)
    }
}

impl Default for FarmState {
    fn default() -> FarmState {
        FarmState {
            farm_admin: Pubkey::default(),
            global_config: Pubkey::default(),

            token: TokenInfo::default(),
            reward_infos: [RewardInfo::default(); MAX_REWARDS_TOKENS],
            num_reward_tokens: 0,

            num_users: 0,
            total_staked_amount: 0,

            farm_vault: Pubkey::default(),
            farm_vaults_authority: Pubkey::default(),
            farm_vaults_authority_bump: 0,

            delegate_authority: Pubkey::default(),
            time_unit: 0,

            is_farm_frozen: 0,
            is_farm_delegated: 0,

            _padding0: [0; 5],

            withdraw_authority: Pubkey::default(),

            deposit_warmup_period: 0,
            withdrawal_cooldown_period: 0,

            total_active_stake_scaled: Decimal::zero().to_scaled_val().unwrap(),
            total_pending_stake_scaled: Decimal::zero().to_scaled_val().unwrap(),
            total_pending_amount: 0,

            slashed_amount_current: 0,
            slashed_amount_cumulative: 0,
            slashed_amount_spill_address: Pubkey::default(),

            locking_mode: 0,
            locking_start_timestamp: 0,
            locking_early_withdrawal_penalty_bps: 0,
            locking_duration: 0,

            deposit_cap_amount: 0,

            scope_prices: Pubkey::default(),
            scope_oracle_price_id: u64::MAX,
            scope_oracle_max_age: u64::MAX,

            pending_farm_admin: Pubkey::default(),
            strategy_id: Pubkey::default(),

            _padding: [0; 86],
        }
    }
}

#[derive(Clone, Copy, Zeroable, Pod, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct RewardScheduleCurve {
    pub points: [RewardPerTimeUnitPoint; REWARD_CURVE_POINTS],
}

#[derive(
    Clone, Copy, Zeroable, Pod, Debug, Default, PartialEq, Eq, AnchorSerialize, AnchorDeserialize,
)]
#[repr(C)]
pub struct RewardPerTimeUnitPoint {
    pub ts_start: u64,
    pub reward_per_time_unit: u64,
}

impl RewardPerTimeUnitPoint {
    pub fn new(ts_start: u64, reward_per_time_unit: u64) -> Self {
        Self {
            ts_start,
            reward_per_time_unit,
        }
    }
}

impl Default for RewardScheduleCurve {
    fn default() -> Self {
        RewardScheduleCurve::from_constant(0)
    }
}

impl RewardScheduleCurve {
    pub fn from_constant(reward_per_time_unit: u64) -> Self {
        let points = [RewardPerTimeUnitPoint {
            ts_start: 0,
            reward_per_time_unit,
        }];
        Self::from_points(&points).unwrap()
    }

    pub fn set_constant(&mut self, rps: u64) {
        *self = Self::from_constant(rps);
    }

    pub fn set_point(&mut self, idx: usize, point: RewardPerTimeUnitPoint) {
        self.points[idx] = point;
    }

    pub fn from_points(pts: &[RewardPerTimeUnitPoint]) -> Result<Self> {
        if pts.is_empty() {
            msg!("Rps curve must have at least 1 point");
            return err!(FarmError::InvalidRpsCurvePoint);
        }
        if pts.len() > REWARD_CURVE_POINTS {
            msg!("Reward rate curve must have at most 20 points");
            return err!(FarmError::InvalidRpsCurvePoint);
        }

        let mut points = [RewardPerTimeUnitPoint {
            ts_start: u64::MAX,
            reward_per_time_unit: 0,
        }; REWARD_CURVE_POINTS];

        points[..pts.len()].copy_from_slice(pts);

        let curve = RewardScheduleCurve { points };
        curve.validate()?;
        Ok(curve)
    }

    pub fn validate(&self) -> Result<()> {
        let pts = &self.points;

        let mut last_ts = pts[0].ts_start;
        for pt in pts.iter().skip(1) {
            if pt.ts_start < last_ts {
                msg!("Rps curve points must be sorted by timestamp");
                return err!(FarmError::InvalidRpsCurvePoint);
            }
            last_ts = pt.ts_start;
        }

        let mut found_max = false;
        for pt in pts.iter() {
            if pt.ts_start == u64::MAX {
                found_max = true;
            } else if found_max {
                msg!("Rps curve points with the special timestamp lower than u64::MAX must never follow a u64::MAX timestamp");
                return err!(FarmError::InvalidRpsCurvePoint);
            }
        }

        for i in 0..pts.len() - 1 {
            if pts[i].ts_start == pts[i + 1].ts_start && pts[i].ts_start != u64::MAX {
                msg!("Rps curve points cannot have the same timestamp");
                return err!(FarmError::InvalidRpsCurvePoint);
            }
        }

        if pts[0].ts_start == u64::MAX {
            msg!("Rps curve points cannot start with a timestamp of u64::MAX");
            return err!(FarmError::InvalidRpsCurvePoint);
        }

        Ok(())
    }

    fn most_recent_curve_starting_point(&self, last_issued_ts: u64) -> Result<usize> {
        for (i, point) in self.points.iter().enumerate() {
            if point.ts_start > last_issued_ts {
                return if i > 0 {
                    Ok(i - 1)
                } else {
                    msg!("Invalid curve: first point has ts_start > last_issued_ts");
                    err!(FarmError::InvalidRpsCurvePoint)
                };
            }
        }
        Ok(self.points.len() - 1)
    }
    pub fn get_cumulative_amount_issued_since_last_ts(
        &self,
        last_issued_ts: u64,
        current_ts: u64,
    ) -> Result<u64> {
        if last_issued_ts > current_ts {
            msg!("last_issued_ts should be less than current_ts");
            return err!(FarmError::InvalidTimestamp);
        }

        let mut cumulative_amount = 0u64;

        let start_index = self.most_recent_curve_starting_point(last_issued_ts)?;

        for i in start_index..self.points.len() {
            let point = &self.points[i];
            if point.ts_start >= current_ts {
                break;
            }

            let start_ts = if point.ts_start > last_issued_ts {
                point.ts_start
            } else {
                last_issued_ts
            };

            let end_ts = if i < self.points.len() - 1 && self.points[i + 1].ts_start < current_ts {
                self.points[i + 1].ts_start
            } else {
                current_ts
            };

            let period_amount = point.reward_per_time_unit * (end_ts - start_ts);
            cumulative_amount += period_amount;
        }

        Ok(cumulative_amount)
    }

    pub fn get_current_rps(&self, current_ts: u64) -> Result<u64> {
        let index = self.most_recent_curve_starting_point(current_ts)?;
        Ok(self.points[index].reward_per_time_unit)
    }
}

static_assertions::const_assert_eq!(0, std::mem::size_of::<UserState>() % 8);
#[account(zero_copy)]
#[derive(Debug, Eq, PartialEq)]
pub struct UserState {
    pub user_id: u64,
    pub farm_state: Pubkey,
    pub owner: Pubkey,

    pub is_farm_delegated: u8,
    pub _padding_0: [u8; 7],

    pub rewards_tally_scaled: [u128; MAX_REWARDS_TOKENS],
    pub rewards_issued_unclaimed: [u64; MAX_REWARDS_TOKENS],
    pub last_claim_ts: [u64; MAX_REWARDS_TOKENS],

    pub active_stake_scaled: u128,

    pub pending_deposit_stake_scaled: u128,
    pub pending_deposit_stake_ts: u64,

    pub pending_withdrawal_unstake_scaled: u128,
    pub pending_withdrawal_unstake_ts: u64,
    pub bump: u64,
    pub delegatee: Pubkey,

    pub last_stake_ts: u64,

    pub _padding_1: [u64; 50],
}

impl UserState {
    pub fn get_active_stake_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.active_stake_scaled)
    }

    pub fn get_pending_deposit_stake_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.pending_deposit_stake_scaled)
    }

    pub fn get_pending_withdrawal_unstake_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.pending_withdrawal_unstake_scaled)
    }

    pub fn get_rewards_tally_decimal(&self, index: usize) -> Decimal {
        Decimal::from_scaled_val(self.rewards_tally_scaled[index])
    }

    pub fn set_active_stake_decimal(&mut self, value: Decimal) {
        self.active_stake_scaled = value.to_scaled_val().unwrap();
    }

    pub fn set_pending_deposit_stake_decimal(&mut self, value: Decimal) {
        self.pending_deposit_stake_scaled = value.to_scaled_val().unwrap();
    }

    pub fn set_pending_withdrawal_unstake_decimal(&mut self, value: Decimal) {
        self.pending_withdrawal_unstake_scaled = value.to_scaled_val().unwrap();
    }

    pub fn set_rewards_tally_decimal(&mut self, index: usize, value: Decimal) {
        self.rewards_tally_scaled[index] = value.to_scaled_val().unwrap();
    }
}

impl Default for UserState {
    fn default() -> UserState {
        UserState {
            user_id: 0,
            farm_state: Pubkey::default(),
            owner: Pubkey::default(),

            is_farm_delegated: false as u8,
            _padding_0: Default::default(),

            rewards_tally_scaled: [0; MAX_REWARDS_TOKENS],
            rewards_issued_unclaimed: [0; MAX_REWARDS_TOKENS],
            last_claim_ts: [0; MAX_REWARDS_TOKENS],

            active_stake_scaled: Decimal::zero().to_scaled_val().unwrap(),
            pending_deposit_stake_scaled: Decimal::zero().to_scaled_val().unwrap(),
            pending_deposit_stake_ts: 0,
            pending_withdrawal_unstake_scaled: Decimal::zero().to_scaled_val().unwrap(),
            pending_withdrawal_unstake_ts: 0,
            bump: 0,
            delegatee: Pubkey::default(),
            last_stake_ts: 0,
            _padding_1: [0; 50],
        }
    }
}

#[zero_copy]
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RewardInfo {
    pub token: TokenInfo,

    pub rewards_vault: Pubkey,

    pub rewards_available: u64,
    pub reward_schedule_curve: RewardScheduleCurve,
    pub min_claim_duration_seconds: u64,
    pub last_issuance_ts: u64,
    pub rewards_issued_unclaimed: u64,
    pub rewards_issued_cumulative: u64,
    pub reward_per_share_scaled: u128,
    pub placeholder_0: u64,

    pub reward_type: u8,
    pub rewards_per_second_decimals: u8,

    pub _padding0: [u8; 6],
    pub _padding1: [u64; 20],
}

impl RewardInfo {
    pub fn get_reward_per_share_decimal(&self) -> Decimal {
        Decimal::from_scaled_val(self.reward_per_share_scaled)
    }

    pub fn set_reward_per_share_decimal(&mut self, value: Decimal) {
        self.reward_per_share_scaled = value.to_scaled_val().unwrap();
    }
}

#[zero_copy]
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub decimals: u64,
    pub _padding: [u64; 10],
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    TryFromPrimitive,
    IntoPrimitive,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
)]
#[repr(u16)]
pub enum FarmConfigOption {
    UpdateRewardRps,
    UpdateRewardMinClaimDuration,
    WithdrawAuthority,
    DepositWarmupPeriod,
    WithdrawCooldownPeriod,
    RewardType,
    RpsDecimals,
    LockingMode,
    LockingStartTimestamp,
    LockingDuration,
    LockingEarlyWithdrawalPenaltyBps,
    DepositCapAmount,
    SlashedAmountSpillAddress,
    ScopePricesAccount,
    ScopeOraclePriceId,
    ScopeOracleMaxAge,
    UpdateRewardScheduleCurvePoints,
    UpdatePendingFarmAdmin,
    UpdateStrategyId,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug,
)]
#[repr(u8)]
pub enum TimeUnit {
    Seconds = 0,
    Slots = 1,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug,
)]
#[repr(u8)]
pub enum RewardType {
    Proportional = 0,
    Constant = 1,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug,
)]
#[repr(u64)]
pub enum LockingMode {
    None = 0,
    Continuous = 1,
    WithExpiry = 2,
}

impl Default for LockingMode {
    fn default() -> LockingMode {
        LockingMode::None
    }
}

impl TimeUnit {
    pub fn now_from_clock(value: u8, click: &Clock) -> u64 {
        let unit = TimeUnit::try_from(value).unwrap();
        match unit {
            TimeUnit::Seconds => click.unix_timestamp as u64,
            TimeUnit::Slots => click.slot,
        }
    }
}

impl RewardInfo {
    pub fn is_initialised(&self) -> bool {
        self.rewards_vault != Pubkey::default()
    }

    pub fn has_rewards_available(&self) -> bool {
        self.rewards_available > 0
    }

    pub fn reward_type(&self) -> RewardType {
        RewardType::try_from(self.reward_type).unwrap()
    }
}
