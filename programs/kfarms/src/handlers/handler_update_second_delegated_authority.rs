use anchor_lang::prelude::*;

use crate::{utils::constraints::check_remaining_accounts, FarmState, GlobalConfig};

pub fn process(ctx: Context<UpdateSecondDelegatedAuthority>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let mut farm_state = ctx.accounts.farm_state.load_mut()?;
    msg!(
        "prv second_delegated_authority: {}",
        farm_state.second_delegated_authority
    );
    msg!(
        "new second_delegated_authority: {}",
        ctx.accounts.new_second_delegated_authority.key()
    );
    farm_state.second_delegated_authority = ctx.accounts.new_second_delegated_authority.key();

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateSecondDelegatedAuthority<'info> {
    #[account(mut)]
    pub global_admin: Signer<'info>,

    #[account(mut,
        has_one = global_config,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(
        has_one = global_admin,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    pub new_second_delegated_authority: AccountInfo<'info>,
}
