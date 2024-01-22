use crate::state::TimeUnit;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::consts::BASE_SEED_FARM_VAULTS_AUTHORITY;
use crate::utils::scope::load_scope_price;
use crate::FarmState;
use crate::{farm_operations, types::AddRewardEffects, FarmError};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

pub fn process(ctx: Context<AddReward>, amount: u64, reward_index: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let reward_mint = &mut ctx.accounts.reward_mint;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;
    msg!(
        "AddReward farm_state {:?} amount {}, reward_index {} ts {}",
        ctx.accounts.farm_state.key(),
        amount,
        reward_index,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    let AddRewardEffects { reward_amount } = farm_operations::add_reward(
        farm_state,
        scope_price,
        reward_mint.key(),
        reward_index as usize,
        amount,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    msg!(
        "add {} to reward {:?} index {}",
        reward_amount,
        reward_mint.key(),
        reward_index
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.reward_token_ata.to_account_info().clone(),
                to: ctx.accounts.reward_vault.to_account_info().clone(),
                authority: ctx.accounts.farm_admin.to_account_info().clone(),
            },
        ),
        reward_amount,
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, reward_index: u64)]
pub struct AddReward<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(mut,
        has_one = farm_admin,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(mut,
        token::mint = reward_mint,
        token::authority = farm_vaults_authority,
        constraint = reward_vault.key() == farm_state.load()?.reward_infos[reward_index as usize].rewards_vault,
    )]
    pub reward_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    #[account(mut,
        constraint = reward_token_ata.mint == reward_mint.key() @ FarmError::RewardAtaRewardMintMissmatch,
        constraint = reward_token_ata.owner == farm_admin.key() @ FarmError::RewardAtaOwnerNotFarmAdmin,
    )]
    pub reward_token_ata: Box<Account<'info, TokenAccount>>,

    pub reward_mint: Account<'info, Mint>,

    pub scope_prices: Option<AccountInfo<'info>>,

    pub token_program: Program<'info, Token>,
}
