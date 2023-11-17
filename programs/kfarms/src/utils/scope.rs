use std::cell::Ref;

use anchor_lang::prelude::{AccountInfo, Pubkey};
use anchor_lang::{prelude::*, Discriminator};
use scope::{DatedPrice, OraclePrices};

use crate::state::FarmState;
use crate::FarmError;

fn get_price_account<'a>(
    scope_price_account: &'a AccountInfo,
    farm_state: &FarmState,
) -> Result<Ref<'a, OraclePrices>> {
    let key = *scope_price_account.key;
    if key == Pubkey::default() || key == crate::ID {
        return Err(FarmError::InvalidOracleConfig.into());
    }

    if key != farm_state.scope_prices {
        return Err(FarmError::InvalidOracleConfig.into());
    }

    let data = scope_price_account.try_borrow_data()?;

    let disc_bytes = &data[0..8];
    if disc_bytes != OraclePrices::discriminator() {
        return Err(FarmError::CouldNotDeserializeScope.into());
    }

    Ok(Ref::map(data, |data| bytemuck::from_bytes(&data[8..])))
}

pub fn load_scope_price(
    scope_price_account: &Option<AccountInfo>,
    farm_state: &FarmState,
) -> Result<Option<DatedPrice>> {
    if farm_state.scope_oracle_price_id == u64::MAX {
        Ok(None)
    } else if scope_price_account.is_none() {
        Err(FarmError::InvalidOracleConfig.into())
    } else {
        let scope_prices = get_price_account(scope_price_account.as_ref().unwrap(), farm_state)?;
        Ok(Some(
            scope_prices.prices[farm_state.scope_oracle_price_id as usize],
        ))
    }
}
