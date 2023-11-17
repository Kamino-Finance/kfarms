use crate::FarmError;
use anchor_lang::{
    err,
    prelude::{Context, Result},
};

pub fn check_remaining_accounts<T>(ctx: &Context<T>) -> Result<()> {
    if !ctx.remaining_accounts.is_empty() {
        return err!(FarmError::UnexpectedAccount);
    }

    Ok(())
}
