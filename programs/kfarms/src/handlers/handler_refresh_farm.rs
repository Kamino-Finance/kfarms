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

    pub scope_prices: Option<AccountInfo<'info>>,
}
