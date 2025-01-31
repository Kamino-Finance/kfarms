use crate::farm_operations;
use crate::state::TimeUnit;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::{FarmError, FarmState, UserState};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<InitializeUser>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let user_state = &mut ctx.accounts.user_state.load_init()?;
    let payer = ctx.accounts.payer.key();
    let owner = ctx.accounts.owner.key();
    let user_state_bump = ctx.bumps.user_state.into();

    msg!(
        "InitializeUser: user {} farm {} ts {}",
        ctx.accounts.user_state.key(),
        ctx.accounts.farm_state.key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    if !farm_state.is_delegated() {
        require_keys_eq!(
            payer,
            ctx.accounts.delegatee.key(),
            FarmError::UserDelegatedFarmNonDelegatedMissmatch
        );
        require_keys_eq!(
            ctx.accounts.authority.key(),
            payer,
            FarmError::UserDelegatedFarmNonDelegatedMissmatch
        );
        require_keys_eq!(
            payer,
            owner,
            FarmError::UserDelegatedFarmNonDelegatedMissmatch
        );
    } else {
        require_keys_eq!(
            farm_state.delegate_authority,
            ctx.accounts.authority.key(),
            FarmError::AuthorityFarmDelegateMissmatch
        );
    }

    user_state.bump = user_state_bump;
    user_state.delegatee = ctx.accounts.delegatee.key();

    farm_operations::initialize_user(
        farm_state,
        user_state,
        &owner,
        &ctx.accounts.farm_state.key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: AccountInfo<'info>,

    pub delegatee: AccountInfo<'info>,

    #[account(init,
        seeds = [BASE_SEED_USER_STATE, farm_state.key().as_ref(), delegatee.key().as_ref()],
        bump,
        payer = payer,
        space = SIZE_USER_STATE,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(mut)]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
