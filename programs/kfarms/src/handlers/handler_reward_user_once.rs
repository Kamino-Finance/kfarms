use crate::state::UserState;
use crate::utils::constraints::check_remaining_accounts;
use crate::{farm_operations, FarmState};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<RewardUserOnce>, reward_index: u64, amount: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let mut farm_state = ctx.accounts.farm_state.load_mut()?;
    let mut user_state = ctx.accounts.user_state.load_mut()?;

    farm_operations::reward_user_once(&mut farm_state, &mut user_state, reward_index, amount)?;

    Ok(())
}

#[derive(Accounts)]
pub struct RewardUserOnce<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(mut, has_one = farm_admin)]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,
}
