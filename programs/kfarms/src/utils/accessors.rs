use anchor_lang::prelude::AccountInfo;
use anchor_lang::Result;

pub fn mint_decimals(account: &AccountInfo) -> Result<u8> {
    let bytes = account.try_borrow_data()?;
    let mut amount_bytes = [0u8; 1];
    amount_bytes.copy_from_slice(&bytes[0x2C..0x2D]);
    Ok(u8::from_le_bytes(amount_bytes))
}
