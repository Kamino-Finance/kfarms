use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    farm_operations, gen_signer_seeds_two,
    state::TimeUnit,
    token_operations,
    types::WithdrawEffects,
    utils::{constraints::check_remaining_accounts, consts::*},
    FarmError, FarmState, UserState,
};

pub fn process(ctx: Context<WithdrawUnstakedDeposits>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let user_state = &mut ctx.accounts.user_state.load_mut()?;

   
    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    let WithdrawEffects { amount_to_withdraw } = farm_operations::withdraw_unstaked_deposits(
        farm_state,
        user_state,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    let farm_state_key = ctx.accounts.farm_state.key();
    let signer_seeds: &[&[&[u8]]] = gen_signer_seeds_two!(
        BASE_SEED_FARM_VAULTS_AUTHORITY,
        farm_state_key,
        farm_state.farm_vaults_authority_bump as u8
    );

    if amount_to_withdraw > 0 {
        token_operations::transfer_from_vault(
            amount_to_withdraw,
            signer_seeds,
            &ctx.accounts.user_ata.to_account_info(),
            &ctx.accounts.farm_vault.to_account_info(),
            &ctx.accounts.farm_vaults_authority,
            &ctx.accounts.token_program,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawUnstakedDeposits<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(mut,
        has_one = farm_vault,
        has_one = farm_vaults_authority,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

   
    #[account(mut,
        has_one = owner,
        constraint = user_ata.mint == farm_state.load()?.token.mint @ FarmError::UserAtaFarmTokenMintMissmatch,
    )]
    pub user_ata: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), farm_state.load()?.token.mint.as_ref()],
        bump,
        constraint = farm_vault.delegate.is_none() @ FarmError::FarmVaultHasDelegate,
        constraint = farm_vault.close_authority.is_none() @ FarmError::FarmVaultHasCloseAuthority,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Verified with a has_one constraint in farm pool state
    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
