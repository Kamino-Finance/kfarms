use crate::FarmError;
use anchor_lang::{
    err,
    prelude::{Context, Result},
    Bumps,
};

pub fn check_remaining_accounts<T>(ctx: &Context<T>) -> Result<()>
where
    T: Bumps,
{
    if !ctx.remaining_accounts.is_empty() {
        return err!(FarmError::UnexpectedAccount);
    }

    Ok(())
}

pub mod token_2022 {
    use crate::FarmError;
    use anchor_lang::err;
    use anchor_lang::prelude::msg;
    use anchor_lang::prelude::{AccountInfo, Pubkey};
    use anchor_spl::token_2022::spl_token_2022;
    use anchor_spl::token_interface::spl_token_2022::extension::ExtensionType;
    use anchor_spl::token_interface::spl_token_2022::extension::{
        BaseStateWithExtensions, StateWithExtensions,
    };
    const VALID_BASE_TOKEN_EXTENSIONS: &[ExtensionType] = &[
        ExtensionType::ConfidentialTransferFeeConfig,
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::MintCloseAuthority,
        ExtensionType::MetadataPointer,
        ExtensionType::PermanentDelegate,
        ExtensionType::TransferFeeConfig,
        ExtensionType::TokenMetadata,
        ExtensionType::TransferHook,
    ];

    pub fn validate_reward_token_extensions(
        mint_acc_info: &AccountInfo,
    ) -> anchor_lang::Result<()> {
        validate_base_token_extensions(mint_acc_info)
    }

    pub fn validate_base_token_extensions(mint_acc_info: &AccountInfo) -> anchor_lang::Result<()> {
        let mint_data = mint_acc_info.data.borrow();
        let mint = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data)?;
        for mint_ext in mint.get_extension_types()? {
            if !VALID_BASE_TOKEN_EXTENSIONS.contains(&mint_ext) {
                msg!(
                    "Invalid base token (2022) extension: {:?}, supported extensions: {:?}",
                    mint_ext,
                    VALID_BASE_TOKEN_EXTENSIONS
                );
                return err!(FarmError::UnsupportedTokenExtension);
            }
            if mint_ext == ExtensionType::TransferFeeConfig {
                let ext = mint
                    .get_extension::<spl_token_2022::extension::transfer_fee::TransferFeeConfig>(
                    )?;
                if <u16>::from(ext.older_transfer_fee.transfer_fee_basis_points) != 0
                    || <u16>::from(ext.newer_transfer_fee.transfer_fee_basis_points) != 0
                {
                    msg!("Transfer fee must be 0 for base tokens, got: {:?}", ext);
                    return err!(FarmError::UnsupportedTokenExtension);
                }
            } else if mint_ext == ExtensionType::TransferHook {
                let ext =
                    mint.get_extension::<spl_token_2022::extension::transfer_hook::TransferHook>()?;
                let hook_program_id: Option<Pubkey> = ext.program_id.into();
                if hook_program_id.is_some() {
                    msg!(
                        "Transfer hook program id must not be set for base tokens, got {:?}",
                        ext
                    );
                    return err!(FarmError::UnsupportedTokenExtension);
                }
            }
        }
        Ok(())
    }
}
