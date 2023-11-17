use crate::utils::constraints::check_remaining_accounts;
use crate::GlobalConfig;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<UpdateGlobalConfigAdmin>) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    global_config.global_admin = global_config.pending_global_admin;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateGlobalConfigAdmin<'info> {
    pub pending_global_admin: Signer<'info>,

    #[account(
        mut,
        has_one = pending_global_admin,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,
}
