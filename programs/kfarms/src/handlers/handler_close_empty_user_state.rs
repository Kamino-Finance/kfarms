use anchor_lang::prelude::*;

use crate::{utils::constraints::check_remaining_accounts, FarmError, FarmState, UserState};

pub fn process(ctx: Context<CloseEmptyUserState>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let user_state = &ctx.accounts.user_state.load()?;
    let farm_state = &ctx.accounts.farm_state.load()?;

    require_eq!(
        user_state.active_stake_scaled,
        0,
        FarmError::CannotCloseUserStateStakeNonZero
    );
    require_eq!(
        user_state.pending_withdrawal_unstake_scaled,
        0,
        FarmError::CannotCloseUserStatePendingUnstakes
    );
    require_eq!(
        user_state.pending_deposit_stake_scaled,
        0,
        FarmError::CannotCloseUserStatePendingDeposits
    );
    for &unclaimed_reward in user_state.rewards_issued_unclaimed.iter() {
        require_eq!(
            unclaimed_reward,
            0,
            FarmError::CannotCloseUserStateUnharvestedRewards
        );
    }

    let signer_key = ctx.accounts.signer.key();
    let rent_receiver_key = ctx.accounts.rent_receiver.key();

    if farm_state.is_delegated() {
        require!(
            signer_key == farm_state.delegate_authority
                || signer_key == farm_state.second_delegated_authority,
            FarmError::CannotCloseUserStateDelegatedSignerNotDelegateAuthority
        );
        require_eq!(
            rent_receiver_key,
            farm_state.farm_admin,
            FarmError::CannotCloseUserStateDelegatedRentReceiverNotAdmin
        );
    } else {
        require_eq!(
            signer_key,
            user_state.owner,
            FarmError::CannotCloseUserStateSignerNotOwner
        );
        require_eq!(
            rent_receiver_key,
            user_state.owner,
            FarmError::CannotCloseUserStateRentReceiverNotOwner
        );
    }

    Ok(())
}

#[derive(Accounts)]
pub struct CloseEmptyUserState<'info> {



    pub signer: Signer<'info>,

    #[account(
        mut,
        close = rent_receiver,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,


    #[account()]
    pub farm_state: AccountLoader<'info, FarmState>,




    /// CHECK: This account is validated in the handler logic
    #[account(mut)]
    pub rent_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
