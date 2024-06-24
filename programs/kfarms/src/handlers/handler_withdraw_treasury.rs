use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::FarmError;
use crate::{
    state::GlobalConfig, utils::constraints::token_2022::validate_reward_token_extensions,
};
use crate::{token_operations, xmsg};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
};

pub fn process(ctx: Context<WithdrawTreasury>, amount: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    validate_reward_token_extensions(&ctx.accounts.reward_mint.to_account_info())?;

    let global_config_key = ctx.accounts.global_config.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        BASE_SEED_TREASURY_VAULTS_AUTHORITY,
        global_config_key.as_ref(),
        &[ctx.bumps.treasury_vault_authority],
    ]];

    if amount > 0 {
        xmsg!(
            "WithdrawTreasury amount: {}, available amount: {}",
            amount,
            ctx.accounts.reward_treasury_vault.amount
        );
        token_operations::transfer_2022_from_vault(
            amount,
            signer_seeds,
            &ctx.accounts
                .withdraw_destination_token_account
                .to_account_info(),
            &ctx.accounts.reward_treasury_vault.to_account_info(),
            &ctx.accounts.treasury_vault_authority,
            &ctx.accounts.token_program,
            &ctx.accounts.reward_mint.to_account_info(),
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

    pub reward_mint: Box<InterfaceAccount<'info, MintInterface>>,

    #[account(mut,
        seeds = [BASE_SEED_REWARD_TREASURY_VAULT, global_config.key().as_ref(), reward_mint.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = treasury_vault_authority,
        token::token_program = token_program
    )]
    pub reward_treasury_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    #[account(
        seeds = [BASE_SEED_TREASURY_VAULTS_AUTHORITY, global_config.key().as_ref()],
        bump,
    )]
    pub treasury_vault_authority: AccountInfo<'info>,

    #[account(mut,
        token::mint = reward_mint,
        token::token_program = token_program
    )]
    pub withdraw_destination_token_account: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    pub token_program: Interface<'info, TokenInterface>,
}
