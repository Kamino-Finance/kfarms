use crate::farm_operations;
use crate::state::TimeUnit;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::scope::load_scope_price;
use crate::FarmState;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<RefreshFarm>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;

    farm_state.is_farm_delegated = farm_state.is_delegated() as u8;

    if farm_state.token.token_program == Pubkey::default() {
        farm_state.token.token_program = anchor_spl::token::ID;
    }

    for i in 0..farm_state.num_reward_tokens {
        let reward_info = &mut farm_state.reward_infos[i as usize];
        if reward_info.token.token_program == Pubkey::default() {
            reward_info.token.token_program = anchor_spl::token::ID;
        }
    }
    farm_operations::refresh_global_rewards(
        farm_state,
        scope_price,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )?;
    Ok(())
}

#[derive(Accounts)]
pub struct RefreshFarm<'info> {
    #[account(mut)]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,
}
