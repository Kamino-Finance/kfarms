use super::{consts::BPS_DIV_FACTOR, math::u64_mul_div};
use crate::{xmsg, FarmError};

pub(crate) fn get_withdrawal_penalty_bps(
    timestamp_beginning: u64,
    timestamp_now: u64,
    timestamp_maturity: u64,
    penalty_bps: u64,
) -> Result<u64, FarmError> {
    if timestamp_maturity < timestamp_beginning {
        return Err(FarmError::InvalidLockingTimestamps);
    }

   
   
    if timestamp_now < timestamp_beginning {
        xmsg!(
            "timestamp_now < timestamp_beginning where the user withdraws before
            the official locking period starts in the case of a \"WithExpiry\" locking mode"
        );
        return Ok(0);
    }

   
    if timestamp_now >= timestamp_maturity {
        xmsg!(
            "Time has passed, can unstake as usual ts_now={:?} ts_maturity={:?}",
            timestamp_now,
            timestamp_maturity
        );
        return Ok(0);
    }

    if penalty_bps > 10000 {
        xmsg!("Penalty percentage is greater than 1000");
        return Err(FarmError::InvalidPenaltyPercentage);
    }

   
   
   
    if penalty_bps == 0 || penalty_bps == 10000 {
        xmsg!("Penalty percentage is 0 or 100, therefore early withdrawal is not allowed");
        return Err(FarmError::EarlyWithdrawalNotAllowed);
    }

   
    let time_remaining = timestamp_maturity - timestamp_now;

   
    let total_duration = timestamp_maturity - timestamp_beginning;

   
    let penalty = penalty_bps * time_remaining / total_duration;

    Ok(penalty)
}

pub fn apply_early_withdrawal_penalty(
    locking_duration: u64,
    locking_start: u64,
    timestamp_now: u64,
    penalty_bps: u64,
    unstake_amount: u64,
) -> Result<(u64, u64), FarmError> {
    let timestamp_maturity = locking_start + locking_duration;
    let timestamp_beginning = locking_start;

    let penalty_bps = get_withdrawal_penalty_bps(
        timestamp_beginning,
        timestamp_now,
        timestamp_maturity,
        penalty_bps,
    )?;

    let penalty_amount = u64_mul_div(unstake_amount, penalty_bps, BPS_DIV_FACTOR);

    Ok((unstake_amount - penalty_amount, penalty_amount))
}
