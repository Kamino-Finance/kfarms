use anchor_lang::prelude::*;

use crate::{
    farm_operations, state::UserState, utils::constraints::check_remaining_accounts, FarmError,
    FarmState,
};






pub fn process(
    ctx: Context<RewardUserOnce>,
    reward_index: u64,
    amount: u64,
    expected_reward_issued_unclaimed: u64,
) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let mut farm_state = ctx.accounts.farm_state.load_mut()?;
    let mut user_state = ctx.accounts.user_state.load_mut()?;

    require_eq!(
        farm_state.is_reward_user_once_enabled,
        1,
        FarmError::RewardUserOnceFeatureDisabled
    );

    require_eq!(
        user_state.rewards_issued_unclaimed[reward_index as usize],
        expected_reward_issued_unclaimed,
        FarmError::CurrentRewardIssuedUnclaimedMismatch
    );

    farm_operations::reward_user_once(&mut farm_state, &mut user_state, reward_index, amount)?;

    Ok(())
}

#[derive(Accounts)]
pub struct RewardUserOnce<'info> {
    #[account(mut)]
    pub delegate_authority: Signer<'info>,

    #[account(mut, has_one = delegate_authority)]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,
}
