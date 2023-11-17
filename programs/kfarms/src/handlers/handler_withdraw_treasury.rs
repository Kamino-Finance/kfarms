use crate::state::GlobalConfig;
use crate::token_operations;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::FarmError;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

pub fn process(ctx: Context<WithdrawTreasury>, amount: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let global_config_key = ctx.accounts.global_config.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        BASE_SEED_TREASURY_VAULTS_AUTHORITY,
        global_config_key.as_ref(),
        &[*ctx.bumps.get("treasury_vault_authority").unwrap()],
    ]];

    if amount > 0 {
        token_operations::transfer_from_vault(
            amount,
            signer_seeds,
            &ctx.accounts
                .withdraw_destination_token_account
                .to_account_info(),
            &ctx.accounts.reward_treasury_vault.to_account_info(),
            &ctx.accounts.treasury_vault_authority,
            &ctx.accounts.token_program,
        )?;
    } else {
        return Err(FarmError::NothingToWithdraw.into());
    }

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawTreasury<'info> {
    #[account(mut)]
    pub global_admin: Signer<'info>,

    #[account(
        has_one = global_admin
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut,
        seeds = [BASE_SEED_REWARD_TREASURY_VAULT, global_config.key().as_ref(), reward_mint.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = treasury_vault_authority,
    )]
    pub reward_treasury_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [BASE_SEED_TREASURY_VAULTS_AUTHORITY, global_config.key().as_ref()],
        bump,
    )]
    pub treasury_vault_authority: AccountInfo<'info>,

    #[account(mut,
        token::mint = reward_mint,
    )]
    pub withdraw_destination_token_account: Box<Account<'info, TokenAccount>>,

    pub reward_mint: Box<Account<'info, Mint>>,

    pub token_program: Program<'info, Token>,
}
