use crate::state::UserState;
use crate::utils::constraints::check_remaining_accounts;
use anchor_lang::prelude::*;
use anchor_lang::{
    prelude::{msg, Context},
    Key,
};

pub fn process(ctx: Context<TransferOwnership>, new_owner: Pubkey) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let user_state = &mut ctx.accounts.user_state.load_mut()?;
    user_state.owner = new_owner;

    msg!(
        "Transferring ownership of farm account {} from {} to {}",
        ctx.accounts.user_state.key(),
        ctx.accounts.owner.key,
        new_owner
    );

    Ok(())
}

#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner,
    )]
    pub user_state: AccountLoader<'info, UserState>,
}
