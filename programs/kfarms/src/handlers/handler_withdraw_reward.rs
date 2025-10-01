use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
};

use crate::{
    farm_operations, gen_signer_seeds_two,
    state::TimeUnit,
    token_operations,
    types::WithdrawRewardEffects,
    utils::{
        constraints::{check_remaining_accounts, token_2022::validate_reward_token_extensions},
        consts::BASE_SEED_FARM_VAULTS_AUTHORITY,
        scope::load_scope_price,
    },
    FarmState,
};

pub fn process(ctx: Context<WithdrawReward>, amount: u64, reward_index: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    validate_reward_token_extensions(&ctx.accounts.reward_mint.to_account_info())?;

    let farm_state_key = ctx.accounts.farm_state.key();
    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let reward_mint = ctx.accounts.reward_vault.mint;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;
    msg!(
        "WithdrawReward farm_state {:?} amount {}, reward_index {} ts {}",
        ctx.accounts.farm_state.key(),
        amount,
        reward_index,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    let WithdrawRewardEffects { reward_amount } = farm_operations::withdraw_reward(
        farm_state,
        scope_price,
        &reward_mint,
        reward_index as usize,
        amount,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    msg!(
        "withdraw {} from reward {:?} index {}",
        reward_amount,
        reward_mint.key(),
        reward_index
    );

    let signer_seeds: &[&[&[u8]]] = gen_signer_seeds_two!(
        BASE_SEED_FARM_VAULTS_AUTHORITY,
        farm_state_key,
        farm_state.farm_vaults_authority_bump as u8
    );

    token_operations::transfer_2022_from_vault(
        reward_amount,
        signer_seeds,
        &ctx.accounts.admin_reward_token_ata.to_account_info(),
        &ctx.accounts.reward_vault.to_account_info(),
        &ctx.accounts.farm_vaults_authority,
        &ctx.accounts.token_program,
        &ctx.accounts.reward_mint.to_account_info(),
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, reward_index: u64)]
pub struct WithdrawReward<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(mut,
        has_one = farm_admin,
        has_one = farm_vaults_authority
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub reward_mint: Box<InterfaceAccount<'info, MintInterface>>,

    #[account(mut,
        token::mint = reward_mint,
        token::authority = farm_vaults_authority,
        token::token_program = token_program,
        constraint = reward_vault.key() == farm_state.load()?.reward_infos[reward_index as usize].rewards_vault,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    /// CHECK: authority
    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    #[account(mut,
        token::mint = reward_mint,
        token::token_program = token_program,
    )]
    pub admin_reward_token_ata: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    /// CHECK: Farm checks this
    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,

    pub token_program: Interface<'info, TokenInterface>,
}
