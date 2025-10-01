use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::{
    farm_operations, gen_signer_seeds_two, token_operations,
    utils::{constraints::check_remaining_accounts, consts::*},
    FarmError, FarmState,
};

pub fn process(ctx: Context<WithdrawSlashedAmount>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;

   
    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    let amount_to_withdraw = farm_operations::withdraw_slashed_amount(farm_state)?;

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
            &ctx.accounts.slashed_amount_spill_address.to_account_info(),
            &ctx.accounts.farm_vault.to_account_info(),
            &ctx.accounts.farm_vaults_authority,
            &ctx.accounts.token_program,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawSlashedAmount<'info> {
    #[account(mut)]
    pub crank: Signer<'info>,

    #[account(mut,
        has_one = farm_vault,
        has_one = farm_vaults_authority,
        has_one = slashed_amount_spill_address,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        token::mint = farm_state.load()?.token.mint,
    )]
    pub slashed_amount_spill_address: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), farm_state.load()?.token.mint.as_ref()],
        bump,
        constraint = farm_vault.delegate.is_none() @ FarmError::FarmVaultHasDelegate,
        constraint = farm_vault.close_authority.is_none() @ FarmError::FarmVaultHasCloseAuthority,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: Verified with a has_one constraint in farm_state
    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}
