use anchor_lang::prelude::*;

use crate::{
    farm_operations,
    state::FarmConfigOption,
    utils::{constraints::check_remaining_accounts, scope::load_scope_price},
    FarmError, FarmState,
};

pub fn process(ctx: Context<UpdateFarmConfig>, mode: u16, data: &[u8]) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state).map_or(None, |v| v);

    let mode: FarmConfigOption = mode.try_into().unwrap();

   
    if matches!(
        mode,
        FarmConfigOption::UpdateRewardRps | FarmConfigOption::UpdateRewardScheduleCurvePoints
    ) {
        require!(
            farm_state.delegated_rps_admin == *ctx.accounts.signer.key
                || farm_state.farm_admin == *ctx.accounts.signer.key,
            FarmError::InvalidFarmConfigUpdateAuthority
        );
    } else {
        require_keys_eq!(farm_state.farm_admin, *ctx.accounts.signer.key);
    }

    farm_operations::update_farm_config(farm_state, scope_price, mode, data)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateFarmConfig<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub farm_state: AccountLoader<'info, FarmState>,

    /// CHECK: Farm checks this
    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,
}
