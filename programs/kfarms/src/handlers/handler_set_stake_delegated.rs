use crate::farm_operations;
use crate::state::TimeUnit;
use crate::utils::constraints::check_remaining_accounts;
use crate::{FarmError, FarmState, UserState};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<SetStakeDelegated>, new_stake: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;

    require!(farm_state.is_delegated(), FarmError::FarmNotDelegated);
    require!(
        farm_state.delegate_authority == ctx.accounts.delegate_authority.key()
            || farm_state.second_delegated_authority == ctx.accounts.delegate_authority.key(),
        FarmError::AuthorityFarmDelegateMissmatch
    );

    let user_state = &mut ctx.accounts.user_state.load_mut()?;

    msg!(
        "SetStakeDelegated: prev:{} -> new:{} ts:{}",
        user_state.active_stake_scaled,
        new_stake,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    );

    farm_operations::set_stake(
        farm_state,
        user_state,
        new_stake,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct SetStakeDelegated<'info> {
    pub delegate_authority: Signer<'info>,

    #[account(mut,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(mut)]
    pub farm_state: AccountLoader<'info, FarmState>,
}
