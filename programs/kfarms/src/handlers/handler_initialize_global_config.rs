use crate::state::GlobalConfig;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::BASE_SEED_TREASURY_VAULTS_AUTHORITY;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<InitializeGlobalConfig>) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    let global_config = &mut ctx.accounts.global_config.load_init()?;

    global_config.global_admin = ctx.accounts.global_admin.key();
    global_config.pending_global_admin = ctx.accounts.global_admin.key();
    global_config.treasury_vaults_authority = ctx.accounts.treasury_vaults_authority.key();
    global_config.treasury_vaults_authority_bump =
        *ctx.bumps.get("treasury_vaults_authority").unwrap() as u64;

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGlobalConfig<'info> {
    #[account(mut)]
    pub global_admin: Signer<'info>,

    #[account(zero)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(
        seeds = [BASE_SEED_TREASURY_VAULTS_AUTHORITY, global_config.key().as_ref()],
        bump,
    )]
    pub treasury_vaults_authority: AccountInfo<'info>,

    system_program: Program<'info, System>,
}
