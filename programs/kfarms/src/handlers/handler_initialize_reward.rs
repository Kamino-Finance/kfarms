use crate::state::GlobalConfig;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::constraints::token_2022::validate_reward_token_extensions;
use crate::utils::consts::*;
use crate::FarmState;
use crate::{farm_operations, state::TimeUnit};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    Mint as MintInterface, TokenAccount as TokenAccountInterface, TokenInterface,
};

pub fn process(ctx: Context<InitializeReward>) -> Result<()> {
    check_remaining_accounts(&ctx)?;
    validate_reward_token_extensions(&ctx.accounts.reward_mint.to_account_info())?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let reward_mint = &mut ctx.accounts.reward_mint;

    farm_operations::initialize_reward(
        farm_state,
        ctx.accounts.reward_vault.key(),
        reward_mint.key(),
        reward_mint.decimals,
        ctx.accounts.token_program.key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;

    msg!(
        "InitializeReward {:?} farm_state {:?} ts {}",
        ctx.accounts.reward_mint.key(),
        ctx.accounts.farm_state.key(),
        TimeUnit::now_from_clock(time_unit, &Clock::get()?)
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeReward<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(
        mut,
        has_one = farm_admin,
        has_one = global_config,
        has_one = farm_vaults_authority
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    #[account(has_one = treasury_vaults_authority)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(
        mint::token_program = token_program,
    )]
    pub reward_mint: Box<InterfaceAccount<'info, MintInterface>>,

    #[account(init,
        payer = farm_admin,
        seeds = [BASE_SEED_REWARD_VAULT, farm_state.key().as_ref(), reward_mint.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = farm_vaults_authority,
        token::token_program = token_program
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    #[account(init_if_needed,
        payer = farm_admin,
        seeds = [BASE_SEED_REWARD_TREASURY_VAULT, global_config.key().as_ref(), reward_mint.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = treasury_vaults_authority,
        token::token_program = token_program
    )]
    pub reward_treasury_vault: Box<InterfaceAccount<'info, TokenAccountInterface>>,

    pub farm_vaults_authority: AccountInfo<'info>,

    pub treasury_vaults_authority: AccountInfo<'info>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}
