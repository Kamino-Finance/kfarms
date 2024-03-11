use anchor_lang::prelude::*;
use scope::DatedPrice;

use crate::state::FarmState;
use crate::FarmError;

pub fn load_scope_price(
    scope_prices_account: &Option<AccountLoader<'_, scope::OraclePrices>>,
    farm_state: &FarmState,
) -> Result<Option<DatedPrice>> {
    if farm_state.scope_oracle_price_id == u64::MAX {
        Ok(None)
    } else if let Some(scope_prices_account) = scope_prices_account {
        let key = scope_prices_account.key();
        if key == Pubkey::default() || key == crate::ID {
            return Err(FarmError::InvalidOracleConfig.into());
        }

        if key != farm_state.scope_prices {
            return Err(FarmError::InvalidOracleConfig.into());
        }
        let scope_prices = scope_prices_account.load()?;
        Ok(Some(
            scope_prices.prices[farm_state.scope_oracle_price_id as usize],
        ))
    } else {
        Err(FarmError::InvalidOracleConfig.into())
    }
}
