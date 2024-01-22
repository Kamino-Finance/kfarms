use crate::farm_operations;
use crate::token_operations;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::{gen_signer_seeds_two, FarmError, FarmState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

pub fn process(ctx: Context<WithdrawFromFarmVault>, amount_to_withdraw: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;

    require!(
        farm_state.withdraw_authority != Pubkey::default(),
        FarmError::UnexpectedAccount
    );

    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    let final_amount_to_withdraw =
        farm_operations::withdraw_from_farm_vault(farm_state, amount_to_withdraw)?;

    let farm_state_key = ctx.accounts.farm_state.key();
    let signer_seeds: &[&[&[u8]]] = gen_signer_seeds_two!(
        BASE_SEED_FARM_VAULTS_AUTHORITY,
        farm_state_key,
        farm_state.farm_vaults_authority_bump as u8
    );

    token_operations::transfer_from_vault(
        final_amount_to_withdraw,
        signer_seeds,
        &ctx.accounts.withdrawer_token_account.to_account_info(),
        &ctx.accounts.farm_vault.to_account_info(),
        &ctx.accounts.farm_vaults_authority,
        &ctx.accounts.token_program,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawFromFarmVault<'info> {
    #[account(mut)]
    pub withdraw_authority: Signer<'info>,

    #[account(mut,
        has_one = farm_vault,
        has_one = farm_vaults_authority,
        has_one = withdraw_authority,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        token::mint = farm_state.load()?.token.mint,
        token::authority = withdraw_authority,
    )]
    pub withdrawer_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), farm_state.load()?.token.mint.as_ref()],
        bump,
        constraint = farm_vault.delegate.is_none() @ FarmError::FarmVaultHasDelegate,
        constraint = farm_vault.close_authority.is_none() @ FarmError::FarmVaultHasCloseAuthority,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
