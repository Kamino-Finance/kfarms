use anchor_lang::prelude::*;

use crate::{
    state::GlobalConfig,
    utils::{constraints::check_remaining_accounts, consts::BASE_SEED_TREASURY_VAULTS_AUTHORITY},
};

pub fn process(ctx: Context<InitializeGlobalConfig>) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    let global_config = &mut ctx.accounts.global_config.load_init()?;

    global_config.global_admin = ctx.accounts.global_admin.key();
    global_config.pending_global_admin = ctx.accounts.global_admin.key();
    global_config.treasury_vaults_authority = ctx.accounts.treasury_vaults_authority.key();
    global_config.treasury_vaults_authority_bump = ctx.bumps.treasury_vaults_authority.into();

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGlobalConfig<'info> {
    #[account(mut)]
    pub global_admin: Signer<'info>,

    #[account(zero)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    /// CHECK: authority
    #[account(
        seeds = [BASE_SEED_TREASURY_VAULTS_AUTHORITY, global_config.key().as_ref()],
        bump,
    )]
    pub treasury_vaults_authority: AccountInfo<'info>,

    system_program: Program<'info, System>,
}
