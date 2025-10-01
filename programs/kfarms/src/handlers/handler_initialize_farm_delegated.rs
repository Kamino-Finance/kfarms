use anchor_lang::prelude::*;

use crate::{
    state::{GlobalConfig, RewardInfo, TimeUnit, TokenInfo},
    utils::{constraints::check_remaining_accounts, consts::*},
    FarmState,
};

pub fn process(ctx: Context<InitializeFarmDelegated>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let mut farm_state = ctx.accounts.farm_state.load_init()?;
    let time_unit = farm_state.time_unit;

    farm_state.farm_admin = ctx.accounts.farm_admin.key();
    farm_state.pending_farm_admin = ctx.accounts.farm_admin.key();
    farm_state.global_config = ctx.accounts.global_config.key();
    farm_state.farm_vaults_authority = ctx.accounts.farm_vaults_authority.key();
    farm_state.farm_vaults_authority_bump = ctx.bumps.farm_vaults_authority.into();
    farm_state.reward_infos = [RewardInfo::default(); 10];
    farm_state.scope_oracle_price_id = u64::MAX;

   
    farm_state.token = TokenInfo::default();
    farm_state.farm_vault = Pubkey::default();
    farm_state.delegate_authority = ctx.accounts.farm_delegate.key();
    farm_state.is_farm_delegated = true as u8;

    msg!(
        "InitializeFarmDelegated {:?} ts {}",
        ctx.accounts.farm_state.to_account_info().key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeFarmDelegated<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    pub farm_delegate: Signer<'info>,

    #[account(zero)]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub global_config: AccountLoader<'info, GlobalConfig>,

    /// CHECK: authority
    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}
