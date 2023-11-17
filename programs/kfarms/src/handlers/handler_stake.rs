use crate::farm_operations;
use crate::state::TimeUnit;
use crate::token_operations::transfer_from_user;
use crate::types::StakeEffects;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::*;
use crate::utils::scope::load_scope_price;
use crate::{FarmError, FarmState, UserState};
use anchor_lang::prelude::*;
use anchor_lang::ToAccountInfo;
use anchor_spl::token::{Mint, Token, TokenAccount};

pub fn process(ctx: Context<Stake>, amount: u64) -> Result<()> {
    require!(amount != 0, FarmError::StakeZero);
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let user_state = &mut ctx.accounts.user_state.load_mut()?;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;
    let time_unit = farm_state.time_unit;

    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    let StakeEffects { amount_to_stake } = farm_operations::stake(
        farm_state,
        user_state,
        scope_price,
        amount,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    msg!(
        "Stake {:} ts {:?}",
        amount_to_stake,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    if amount_to_stake > 0 {
        transfer_from_user(
            amount_to_stake,
            &ctx.accounts.user_ata.to_account_info(),
            &ctx.accounts.farm_vault.to_account_info(),
            &ctx.accounts.owner,
            &ctx.accounts.token_program,
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
pub struct Stake<'info> {
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(mut,
        has_one = farm_vault,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        seeds = [BASE_SEED_FARM_VAULT, farm_state.key().as_ref(), farm_state.load_mut()?.token.mint.as_ref()],
        bump,
        constraint = farm_vault.delegate.is_none() @ FarmError::FarmVaultHasDelegate,
        constraint = farm_vault.close_authority.is_none() @ FarmError::FarmVaultHasCloseAuthority,
    )]
    pub farm_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut,
        has_one = owner,
        constraint = user_ata.mint == farm_state.load_mut()?.token.mint @ FarmError::UserAtaFarmTokenMintMissmatch,
    )]
    pub user_ata: Box<Account<'info, TokenAccount>>,

    #[account(
        constraint = token_mint.key() == farm_state.load_mut()?.token.mint @ FarmError::TokenFarmTokenMintMissmatch,
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    pub scope_prices: Option<AccountInfo<'info>>,

    pub token_program: Program<'info, Token>,
}
