use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::{
        Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
    },
};

use crate::{
    farm_operations,
    state::TimeUnit,
    types::AddRewardEffects,
    utils::{
        constraints::{check_remaining_accounts, token_2022::validate_reward_token_extensions},
        consts::BASE_SEED_FARM_VAULTS_AUTHORITY,
        scope::load_scope_price,
    },
    FarmError, FarmState,
};

pub fn process(ctx: Context<AddReward>, amount: u64, reward_index: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    validate_reward_token_extensions(&ctx.accounts.reward_mint.to_account_info())?;

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

    token_2022::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token_2022::TransferChecked {
                from: ctx
                    .accounts
                    .payer_reward_token_ata
                    .to_account_info()
                    .clone(),
                to: ctx.accounts.reward_vault.to_account_info().clone(),
                authority: ctx.accounts.payer.to_account_info().clone(),
                mint: ctx.accounts.reward_mint.to_account_info().clone(),
            },
        ),
        reward_amount,
        ctx.accounts.reward_mint.decimals,
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, reward_index: u64)]
pub struct AddReward<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
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
        token::token_program = token_program,
        constraint = payer_reward_token_ata.mint == reward_mint.key() @ FarmError::RewardAtaRewardMintMissmatch,
        constraint = payer_reward_token_ata.owner == payer.key() @ FarmError::RewardAtaOwnerNotPayer,
    )]
    pub payer_reward_token_ata: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    /// CHECK: Farm checks this
    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,

    pub token_program: Interface<'info, TokenInterface>,
}
