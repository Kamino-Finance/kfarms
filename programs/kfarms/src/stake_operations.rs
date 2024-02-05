use std::ops::{Deref, DerefMut};

use decimal_wad::decimal::Decimal;

use crate::{
    state::{self, LockingMode},
    types::VaultWithdrawEffects,
    utils::{
        math::{full_decimal_mul_div, u64_mul_div},
        withdrawal_penalty::apply_early_withdrawal_penalty,
    },
    xmsg, FarmError,
};

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct UserStake {
    active_stake: Decimal,
    pending_deposit_stake: Decimal,
    pending_withdrawal_unstake: Decimal,
    last_stake_ts: u64,
}

pub trait UserStakeAccessor {
    fn get_accessor(&mut self) -> UserStakeAbstract<Self>
    where
        Self: Sized;
    fn update(&mut self, abstract_val: UserStake)
    where
        Self: Sized;
}

pub struct UserStakeAbstract<'a, T: UserStakeAccessor> {
    internal: UserStake,
    src_ref: &'a mut T,
}

impl<T: UserStakeAccessor> Deref for UserStakeAbstract<'_, T> {
    type Target = UserStake;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<T: UserStakeAccessor> DerefMut for UserStakeAbstract<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.internal
    }
}

impl<T: UserStakeAccessor> Drop for UserStakeAbstract<'_, T> {
    fn drop(&mut self) {
        self.src_ref.update(self.internal);
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct FarmStake {
    total_active_stake: Decimal,
    total_pending_stake: Decimal,
    total_active_amount: u64,
    total_pending_amount: u64,
    locking_mode: LockingMode,
    locking_start_timestamp: u64,
    locking_duration: u64,
    locking_early_withdrawal_penalty_bps: u64,
}

pub trait FarmStakeAccessor {
    fn get_accessor(&mut self) -> FarmStakeAbstract<Self>
    where
        Self: Sized;
    fn update(&mut self, abstract_val: FarmStake)
    where
        Self: Sized;
}
pub struct FarmStakeAbstract<'a, T: FarmStakeAccessor> {
    internal: FarmStake,
    src_ref: &'a mut T,
}

impl<T: FarmStakeAccessor> Deref for FarmStakeAbstract<'_, T> {
    type Target = FarmStake;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<T: FarmStakeAccessor> DerefMut for FarmStakeAbstract<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.internal
    }
}

impl<T: FarmStakeAccessor> Drop for FarmStakeAbstract<'_, T> {
    fn drop(&mut self) {
        self.src_ref.update(self.internal);
    }
}

impl UserStakeAccessor for state::UserState {
    fn get_accessor(&mut self) -> UserStakeAbstract<'_, Self> {
        UserStakeAbstract {
            internal: UserStake {
                active_stake: self.get_active_stake_decimal(),
                pending_deposit_stake: self.get_pending_deposit_stake_decimal(),
                pending_withdrawal_unstake: self.get_pending_withdrawal_unstake_decimal(),
                last_stake_ts: self.last_stake_ts,
            },
            src_ref: self,
        }
    }

    fn update(&mut self, abstract_val: UserStake) {
        self.set_active_stake_decimal(abstract_val.active_stake);
        self.set_pending_deposit_stake_decimal(abstract_val.pending_deposit_stake);
        self.set_pending_withdrawal_unstake_decimal(abstract_val.pending_withdrawal_unstake);
    }
}

impl FarmStakeAccessor for state::FarmState {
    fn get_accessor(&mut self) -> FarmStakeAbstract<'_, Self> {
        FarmStakeAbstract {
            internal: FarmStake {
                total_active_stake: self.get_total_active_stake_decimal(),
                total_pending_stake: self.get_total_pending_stake_decimal(),
                total_active_amount: self.total_staked_amount,
                total_pending_amount: self.total_pending_amount,
                locking_duration: self.locking_duration,
                locking_early_withdrawal_penalty_bps: self.locking_early_withdrawal_penalty_bps,
                locking_mode: self.get_locking_mode(),
                locking_start_timestamp: self.locking_start_timestamp,
            },
            src_ref: self,
        }
    }

    fn update(&mut self, abstract_val: FarmStake) {
        self.set_total_active_stake_decimal(abstract_val.total_active_stake);
        self.set_total_pending_stake_decimal(abstract_val.total_pending_stake);
        self.total_staked_amount = abstract_val.total_active_amount;
        self.total_pending_amount = abstract_val.total_pending_amount;
    }
}

pub fn convert_stake_to_amount(
    stake: Decimal,
    total_stake: Decimal,
    total_amount: u64,
    round_up: bool,
) -> u64 {
    if stake == Decimal::zero() {
        return 0;
    }

    let amount_dec = if total_stake != Decimal::zero() {
        full_decimal_mul_div(stake, total_amount, total_stake)
    } else {
        total_amount.into()
    };

    if round_up {
        amount_dec.try_ceil().unwrap()
    } else {
        amount_dec.try_floor().unwrap()
    }
}

pub fn convert_amount_to_stake(amount: u64, total_stake: Decimal, total_amount: u64) -> Decimal {
    if amount == 0 {
        return Decimal::zero();
    }
    if total_stake == Decimal::zero() || total_amount == 0 {
        assert_eq!(
            total_stake,
            Decimal::zero(),
            "Total amount is zero but total stake is not"
        );
        Decimal::from(amount)
    } else {
        total_stake * amount / total_amount
    }
}

pub fn add_pending_deposit_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
    deposited_amount: u64,
) -> Result<Decimal, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    let user_gained_pending_stake = convert_amount_to_stake(
        deposited_amount,
        farm.total_pending_stake,
        farm.total_pending_amount,
    );

    user_stake.pending_deposit_stake = user_stake.pending_deposit_stake + user_gained_pending_stake;

    farm.total_pending_amount += deposited_amount;
    farm.total_pending_stake = farm.total_pending_stake + user_gained_pending_stake;

    Ok(user_gained_pending_stake)
}

pub fn remove_pending_deposit_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
) -> Result<u64, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    let pending_amount_removed: u64 = convert_stake_to_amount(
        user_stake.pending_deposit_stake,
        farm.total_pending_stake,
        farm.total_pending_amount,
        false,
    );

    farm.total_pending_amount -= pending_amount_removed;

    farm.total_pending_stake = farm.total_pending_stake - user_stake.pending_deposit_stake;

    user_stake.pending_deposit_stake = Decimal::zero();

    Ok(pending_amount_removed)
}

pub fn add_active_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
    staked_amount: u64,
) -> Result<Decimal, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    let user_gained_active_stake = convert_amount_to_stake(
        staked_amount,
        farm.total_active_stake,
        farm.total_active_amount,
    );

    user_stake.active_stake = user_stake.active_stake + user_gained_active_stake;

    farm.total_active_amount += staked_amount;
    farm.total_active_stake = farm.total_active_stake + user_gained_active_stake;

    Ok(user_gained_active_stake)
}

pub fn activate_pending_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
) -> Result<(u64, Decimal), FarmError> {
    let amount_to_stake = remove_pending_deposit_stake(user_stake, farm)?;
    let gained_active_stake = add_active_stake(user_stake, farm, amount_to_stake)?;
    Ok((amount_to_stake, gained_active_stake))
}

pub fn remove_active_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
    unstaked_shares: Decimal,
) -> Result<u64, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    assert!(
        unstaked_shares <= user_stake.active_stake,
        "Not enough active stake ({}) to perform this unstake ({} requested)",
        user_stake.active_stake,
        unstaked_shares
    );

    let unstaked_amount: u64 = convert_stake_to_amount(
        unstaked_shares,
        farm.total_active_stake,
        farm.total_active_amount,
        false,
    );

    user_stake.active_stake = user_stake.active_stake - unstaked_shares;

    farm.total_active_amount -= unstaked_amount;
    farm.total_active_stake = farm.total_active_stake - unstaked_shares;

    Ok(unstaked_amount)
}

pub fn add_pending_withdrawal_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
    unstaked_amount: u64,
) -> Result<Decimal, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    let user_gained_pending_stake = convert_amount_to_stake(
        unstaked_amount,
        farm.total_pending_stake,
        farm.total_pending_amount,
    );

    user_stake.pending_withdrawal_unstake =
        user_stake.pending_withdrawal_unstake + user_gained_pending_stake;

    farm.total_pending_amount += unstaked_amount;
    farm.total_pending_stake = farm.total_pending_stake + user_gained_pending_stake;

    Ok(user_gained_pending_stake)
}

pub fn unstake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
    stake_share_to_unstake: Decimal,
    ts: u64,
) -> Result<(u64, Decimal, u64), FarmError> {
    let amount_to_unstake = remove_active_stake(user_stake, farm, stake_share_to_unstake)?;

    let farm_accessor = farm.get_accessor();
    let user_accessor = user_stake.get_accessor();
    let (amount_to_unstake_post_penalty, unstake_penalty) = match farm_accessor.locking_mode {
        LockingMode::None => (amount_to_unstake, 0),
        LockingMode::WithExpiry => apply_early_withdrawal_penalty(
            farm_accessor.locking_duration,
            farm_accessor.locking_start_timestamp,
            ts,
            farm_accessor.locking_early_withdrawal_penalty_bps,
            amount_to_unstake,
        )?,
        LockingMode::Continuous => apply_early_withdrawal_penalty(
            farm_accessor.locking_duration,
            user_accessor.last_stake_ts,
            ts,
            farm_accessor.locking_early_withdrawal_penalty_bps,
            amount_to_unstake,
        )?,
    };

    if farm_accessor.locking_mode != LockingMode::None {
        xmsg!(
            "Unstaking {}, with mode {:?}, got {} and penalty {}",
            amount_to_unstake,
            farm_accessor.locking_mode,
            amount_to_unstake_post_penalty,
            unstake_penalty
        );
    }

    drop(farm_accessor);
    drop(user_accessor);

    let gained_pending_stake =
        add_pending_withdrawal_stake(user_stake, farm, amount_to_unstake_post_penalty)?;
    Ok((
        amount_to_unstake_post_penalty,
        gained_pending_stake,
        unstake_penalty,
    ))
}

pub fn remove_pending_withdrawal_stake(
    user_stake: &mut impl UserStakeAccessor,
    farm: &mut impl FarmStakeAccessor,
) -> Result<u64, FarmError> {
    let mut user_stake = user_stake.get_accessor();
    let mut farm = farm.get_accessor();

    let pending_amount_removed: u64 = convert_stake_to_amount(
        user_stake.pending_withdrawal_unstake,
        farm.total_pending_stake,
        farm.total_pending_amount,
        false,
    );

    farm.total_pending_amount -= pending_amount_removed;
    farm.total_pending_stake = farm.total_pending_stake - user_stake.pending_withdrawal_unstake;

    user_stake.pending_withdrawal_unstake = Decimal::zero();

    Ok(pending_amount_removed)
}

pub fn increase_total_amount(
    farm: &mut impl FarmStakeAccessor,
    amount: u64,
) -> Result<(), FarmError> {
    let mut farm = farm.get_accessor();

    farm.total_active_amount += amount;
    Ok(())
}

pub fn withdraw_farm(
    farm: &mut impl FarmStakeAccessor,
    req_withdraw_amount: u64,
) -> Result<VaultWithdrawEffects, FarmError> {
    let mut farm = farm.get_accessor();

    let vault_amount = farm.total_active_amount + farm.total_pending_amount;

    if req_withdraw_amount >= vault_amount {
        farm.total_active_amount = 0;
        farm.total_pending_amount = 0;
        xmsg!("Withdraw all farm vault (left frozen): {vault_amount}");
        return Ok(VaultWithdrawEffects {
            amount_to_withdraw: vault_amount,
            farm_to_freeze: true,
        });
    }

    let removed_active_amount: u64 =
        u64_mul_div(farm.total_active_amount, req_withdraw_amount, vault_amount);
    let removed_pending_amount: u64 =
        u64_mul_div(farm.total_pending_amount, req_withdraw_amount, vault_amount);

    farm.total_active_amount -= removed_active_amount;
    farm.total_pending_amount -= removed_pending_amount;

    let amount_to_withdraw = removed_active_amount + removed_pending_amount;

    xmsg!("Withdraw farm vault: {removed_active_amount} (active) + {removed_pending_amount} (pending) = {amount_to_withdraw} / {req_withdraw_amount}");

    Ok(VaultWithdrawEffects {
        amount_to_withdraw,
        farm_to_freeze: false,
    })
}
