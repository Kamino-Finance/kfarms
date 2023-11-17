use crate::farm_operations;
use crate::state::TimeUnit;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::scope::load_scope_price;
use crate::{FarmError, FarmState, UserState};
use anchor_lang::prelude::*;
use decimal_wad::decimal::Decimal;

pub fn process(ctx: Context<Unstake>, amount: Decimal) -> Result<()> {
    require!(amount != Decimal::zero(), FarmError::UnstakeZero);
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let user_state = &mut ctx.accounts.user_state.load_mut()?;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;

    require!(!farm_state.is_delegated(), FarmError::FarmDelegated);

    farm_operations::unstake(
        farm_state,
        user_state,
        scope_price,
        amount,
        TimeUnit::now_from_clock(time_unit, &Clock::get()?),
    )
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut,
        has_one = owner,
        has_one = farm_state,
    )]
    pub user_state: AccountLoader<'info, UserState>,

    #[account(mut)]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub scope_prices: Option<AccountInfo<'info>>,
}
