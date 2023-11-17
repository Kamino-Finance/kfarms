use anchor_lang::prelude::{msg, AccountInfo, CpiContext, Result};

use anchor_spl::token::{self, Transfer};

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
