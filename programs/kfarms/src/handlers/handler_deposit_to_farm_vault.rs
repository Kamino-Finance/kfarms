use crate::farm_operations;
use crate::token_operations::transfer_from_user;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::{FarmError, FarmState};
use anchor_lang::prelude::*;
use anchor_lang::ToAccountInfo;
use anchor_spl::token::{Token, TokenAccount};

pub fn process(ctx: Context<DepositToFarmVault>, amount: u64) -> Result<()> {
    require!(amount != 0, FarmError::DepositZero);
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;

    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    farm_operations::deposit_to_farm_vault(farm_state, amount)?;

    transfer_from_user(
        amount,
        &ctx.accounts.depositor_ata.to_account_info(),
        &ctx.accounts.farm_vault.to_account_info(),
        &ctx.accounts.depositor,
        &ctx.accounts.token_program,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct DepositToFarmVault<'info> {
    pub depositor: Signer<'info>,

    #[account(mut,
        has_one = farm_vault,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), farm_state.load()?.token.mint.as_ref()],
        bump,
        constraint = farm_vault.delegate.is_none() @ FarmError::FarmVaultHasDelegate,
        constraint = farm_vault.close_authority.is_none() @ FarmError::FarmVaultHasCloseAuthority,
        constraint = farm_vault.mint == farm_state.load_mut()?.token.mint @ FarmError::TokenFarmTokenMintMissmatch,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        constraint = depositor_ata.mint == farm_state.load()?.token.mint @ FarmError::UserAtaFarmTokenMintMissmatch,
        token::authority = depositor,
    )]
    pub depositor_ata: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}
