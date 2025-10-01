use anchor_lang::prelude::*;

use crate::{
    farm_operations, utils::constraints::check_remaining_accounts, GlobalConfig, GlobalConfigOption,
};

const VALUE_BYTE_ARRAY_LEN: usize = 32;

pub fn process(
    ctx: Context<UpdateGlobalConfig>,
    key: GlobalConfigOption,
    value: &[u8; VALUE_BYTE_ARRAY_LEN],
) -> Result<()> {
    msg!("Update global config key={:?} value={:?}", key, value);
    check_remaining_accounts(&ctx)?;
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    farm_operations::update_global_config(global_config, key, value)?;

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateGlobalConfig<'info> {
    pub global_admin: Signer<'info>,

    #[account(
        mut,
        has_one = global_admin,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,
}
