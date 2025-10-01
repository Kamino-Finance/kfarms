use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{
    state::{GlobalConfig, RewardInfo, TimeUnit, TokenInfo},
    utils::{constraints::check_remaining_accounts, consts::*},
    FarmState,
};

pub fn process(ctx: Context<InitializeFarm>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let mut farm_state = ctx.accounts.farm_state.load_init()?;

    farm_state.farm_admin = ctx.accounts.farm_admin.key();
    farm_state.pending_farm_admin = ctx.accounts.farm_admin.key();
    farm_state.global_config = ctx.accounts.global_config.key();
    farm_state.farm_vaults_authority = ctx.accounts.farm_vaults_authority.key();
    farm_state.farm_vaults_authority_bump = ctx.bumps.farm_vaults_authority.into();
    farm_state.reward_infos = [RewardInfo::default(); 10];
    farm_state.scope_oracle_price_id = u64::MAX;

   
    farm_state.token = TokenInfo {
        mint: ctx.accounts.token_mint.key(),
        decimals: ctx.accounts.token_mint.decimals as u64,
        token_program: ctx.accounts.token_program.key(),
        _padding: [0; 6],
    };
    farm_state.farm_vault = ctx.accounts.farm_vault.key();
    farm_state.delegate_authority = Pubkey::default();
    farm_state.second_delegated_authority = Pubkey::default();
    msg!(
        "Initialize farm {:?} ts {}",
        ctx.accounts.farm_state.to_account_info().key(),
        TimeUnit::now_from_clock(farm_state.time_unit, &Clock::get()?)
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeFarm<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(zero)]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(init,
        payer = farm_admin,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = farm_vaults_authority,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: authority
    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub token_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}
