use anchor_lang::prelude::{msg, AccountInfo, CpiContext, Result};
use anchor_spl::{
    token::{self, Transfer},
    token_interface::TransferChecked,
};

use crate::utils::accessors::mint_decimals;

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_vault<'info>(
    amount: u64,
    signer: &[&[&[u8]]],
    to_vault: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let cpi_transfer_accounts = Transfer {
        from: from_vault.clone(),
        to: to_vault.clone(),
        authority: from_vault_authority.clone(),
    };

    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_transfer_accounts).with_signer(signer);
    token::transfer(cpi_ctx, amount)
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_2022_from_vault<'info>(
    amount: u64,
    signer: &[&[&[u8]]],
    to_vault: &AccountInfo<'info>,
    from_vault: &AccountInfo<'info>,
    from_vault_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
) -> Result<()> {
    let cpi_transfer_accounts = TransferChecked {
        from: from_vault.clone(),
        to: to_vault.clone(),
        authority: from_vault_authority.clone(),
        mint: mint.clone(),
    };

    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_transfer_accounts).with_signer(signer);
    anchor_spl::token_2022::transfer_checked(cpi_ctx, amount, mint_decimals(mint)?)
}

pub fn transfer_from_user<'info>(
    amount: u64,
    from_ata: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<()> {
    let cpi_transfer_accounts = Transfer {
        from: from_ata.clone(),
        to: to.clone(),
        authority: authority.clone(),
    };
    let cpi_ctx = CpiContext::new(token_program.clone(), cpi_transfer_accounts);

    let result = token::transfer(cpi_ctx, amount);
    msg!("Transferred {:?}", result);
    result
}
