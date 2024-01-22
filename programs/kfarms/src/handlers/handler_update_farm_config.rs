use crate::state::FarmConfigOption;
use crate::utils::constraints::check_remaining_accounts;
use crate::utils::scope::load_scope_price;
use crate::{farm_operations, FarmState};
use anchor_lang::prelude::*;

pub fn process(ctx: Context<UpdateFarmConfig>, mode: u16, data: &[u8]) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state).map_or(None, |v| v);

    let mode: FarmConfigOption = mode.try_into().unwrap();
    farm_operations::update_farm_config(farm_state, scope_price, mode, data)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateFarmConfig<'info> {
    #[account(mut)]
    pub farm_admin: Signer<'info>,

    #[account(
        mut,
        has_one = farm_admin,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    pub scope_prices: Option<AccountInfo<'info>>,
}
