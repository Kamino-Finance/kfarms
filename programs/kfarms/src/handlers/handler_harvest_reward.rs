use crate::farm_operations;
use crate::gen_signer_seeds_two;
use crate::state::TimeUnit;
use crate::token_operations;
use crate::types::HarvestEffects;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::constraints::token_2022::validate_reward_token_extensions;
use crate::utils::consts::*;
use crate::utils::scope::load_scope_price;
use crate::{FarmError, FarmState, GlobalConfig, UserState};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
};

pub fn process(ctx: Context<HarvestReward>, reward_index: u64) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    validate_reward_token_extensions(&ctx.accounts.reward_mint.to_account_info())?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;

    let user_state = &mut ctx.accounts.user_state.load_mut()?;
    let global_config = &ctx.accounts.global_config.load()?;

    require!(
        reward_index < farm_state.num_reward_tokens,
        FarmError::RewardIndexOutOfRange
    );

    msg!(
        "HarvestReward user_state {:?}, farm_state {:?} ts {}",
        ctx.accounts.user_state.key(),
        ctx.accounts.farm_state.key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );
    let HarvestEffects {
        reward_user,
        reward_treasury,
    } = farm_operations::harvest(
        farm_state,
        user_state,
        global_config,
        scope_price,
        reward_index as usize,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    msg!(
        "owner {:?} amount_user {:?}, amount_treasury {:?}",
        user_state.owner,
        reward_user,
        reward_treasury
    );

    let farm_state_key = ctx.accounts.farm_state.key();

    let signer_seeds: &[&[&[u8]]] = gen_signer_seeds_two!(
        BASE_SEED_FARM_VAULTS_AUTHORITY,
        farm_state_key,
        farm_state.farm_vaults_authority_bump as u8
    );

    if reward_user > 0 {
        token_operations::transfer_2022_from_vault(
            reward_user,
            signer_seeds,
            &ctx.accounts.user_reward_ata.to_account_info(),
            &ctx.accounts.rewards_vault.to_account_info(),
            &ctx.accounts.farm_vaults_authority,
            &ctx.accounts.token_program,
            &ctx.accounts.reward_mint.to_account_info(),
        )?;
    }

    if reward_treasury > 0 {
        token_operations::transfer_2022_from_vault(
            reward_treasury,
            signer_seeds,
            &ctx.accounts.rewards_treasury_vault.to_account_info(),
            &ctx.accounts.rewards_vault.to_account_info(),
            &ctx.accounts.farm_vaults_authority,
            &ctx.accounts.token_program,
            &ctx.accounts.reward_mint.to_account_info(),
        )?;
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(reward_index: u64)]
pub struct HarvestReward<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(
        mut,
        has_one = global_config,
        has_one = farm_vaults_authority
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub global_config: AccountLoader<'info, GlobalConfig>,

    pub reward_mint: Box<InterfaceAccount<'info, MintInterface>>,

    #[account(mut,
        has_one = owner,
        constraint = user_reward_ata.mint == reward_mint.key() @ FarmError::UserAtaRewardVaultMintMissmatch,
        token::token_program = token_program
    )]
    pub user_reward_ata: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    #[account(mut,
        seeds = [BASE_SEED_REWARD_VAULT, farm_state.key().as_ref(), rewards_vault.mint.as_ref()],
        bump,
        constraint = rewards_vault.delegate.is_none() @ FarmError::RewardsVaultHasDelegate,
        constraint = rewards_vault.close_authority.is_none() @ FarmError::RewardsVaultHasCloseAuthority,
        constraint = rewards_vault.key() == farm_state.load()?.reward_infos[reward_index as usize].rewards_vault @ FarmError::RewardVaultMismatch,
        token::mint = reward_mint,
        token::token_program = token_program
    )]
    pub rewards_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    #[account(mut,
        seeds = [BASE_SEED_REWARD_TREASURY_VAULT.as_ref(), global_config.key().as_ref(), rewards_vault.mint.as_ref()],
        bump,
        constraint = rewards_vault.delegate.is_none() @ FarmError::RewardsTreasuryVaultHasDelegate,
        constraint = rewards_vault.close_authority.is_none() @ FarmError::RewardsTreasuryVaultHasCloseAuthority,
        token::mint = reward_mint,
        token::token_program = token_program
    )]
    pub rewards_treasury_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    #[account(
        seeds = [BASE_SEED_FARM_VAULTS_AUTHORITY, farm_state.key().as_ref()],
        bump,
    )]
    pub farm_vaults_authority: AccountInfo<'info>,

    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,

    pub token_program: Interface<'info, TokenInterface>,
}
