use anchor_lang::prelude::AccountInfo;
use anchor_lang::{Result, ToAccountInfo};

pub fn mint_decimals(account: &AccountInfo) -> Result<u8> {
    let bytes = account.try_borrow_data()?;
    let mut amount_bytes = [0u8; 1];
    amount_bytes.copy_from_slice(&bytes[0x2C..0x2D]);
    Ok(u8::from_le_bytes(amount_bytes))
}

pub fn account_discriminator(account: &dyn ToAccountInfo) -> Result<[u8; 8]> {
    let account = account.to_account_info();
    let data = account.try_borrow_data()?;
    let mut disc_bytes = [0u8; 8];
    disc_bytes.copy_from_slice(&data[..8]);
    Ok(disc_bytes)
}
