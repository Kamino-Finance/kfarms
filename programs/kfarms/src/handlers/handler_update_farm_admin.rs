use crate::utils::constraints::check_remaining_accounts;
use crate::FarmState;
use anchor_lang::prelude::*;

pub fn process(ctx: Context<UpdateFarmAdmin>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;

    msg!(
        "Update farm admin prev={:?} new={:?}",
        farm_state.farm_admin,
        farm_state.pending_farm_admin
    );

    farm_state.farm_admin = farm_state.pending_farm_admin;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateFarmAdmin<'info> {
    #[account(mut)]
    pub pending_farm_admin: Signer<'info>,

    #[account(
        mut,
        has_one = pending_farm_admin,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,
}
