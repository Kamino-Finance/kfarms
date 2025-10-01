use anchor_lang::{
    prelude::{msg, Context, *},
    Discriminator, Key,
};

use crate::{
    farm_operations,
    state::UserState,
    types::{AccountLoaderState, StakeEffects, WithdrawEffects},
    utils::{
        accessors::account_discriminator,
        constraints::check_remaining_accounts,
        consts::{BASE_SEED_USER_STATE, SIZE_USER_STATE},
        scope::load_scope_price,
    },
    FarmError, FarmState, TimeUnit,
};

pub fn process(ctx: Context<TransferOwnership>) -> Result<()> {
    check_remaining_accounts(&ctx)?;

    let old_user_state = &mut ctx.accounts.old_user_state.load_mut()?;
    let old_user_state_stake = old_user_state.get_active_stake_decimal();
    let new_user_account_state =
        if let Ok(UserState::DISCRIMINATOR) = account_discriminator(&ctx.accounts.new_user_state) {
            AccountLoaderState::Initialized
        } else {
            AccountLoaderState::Zeroed
        };
    let mut new_user_state = match new_user_account_state {
        AccountLoaderState::Zeroed => ctx.accounts.new_user_state.load_init()?,
        AccountLoaderState::Initialized => ctx.accounts.new_user_state.load_mut()?,
    };
    let farm_state = &mut ctx.accounts.farm_state.load_mut()?;
    let time_unit = farm_state.time_unit;
    let new_user_state_bump = ctx.bumps.new_user_state.into();
    let new_owner = ctx.accounts.new_owner.key();
    let scope_price = load_scope_price(&ctx.accounts.scope_prices, farm_state)?;
    let timestamp = TimeUnit::now_from_clock(time_unit, &Clock::get()?);

    {
       
        require!(!farm_state.is_delegated(), FarmError::FarmDelegated);
        require_keys_eq!(
            old_user_state.delegatee,
            old_user_state.owner,
            FarmError::InvalidTransferOwnershipUserStateOwnerDelegatee
        );
       
        require_eq!(
            farm_state.locking_mode,
            0,
            FarmError::InvalidTransferOwnershipFarmStateLockingMode
        );
          
          
        require_eq!(
            farm_state.withdrawal_cooldown_period,
            0,
            FarmError::InvalidTransferOwnershipFarmStateWithdrawCooldownPeriod
        )
    }

    if matches!(new_user_account_state, AccountLoaderState::Initialized) {
        require_keys_eq!(
            new_user_state.delegatee,
            new_user_state.owner,
            FarmError::InvalidTransferOwnershipUserStateOwnerDelegatee
        );
        require_keys_eq!(
            new_user_state.owner,
            ctx.accounts.new_owner.key(),
            FarmError::InvalidTransferOwnershipNewOwner
        );
        require_keys_eq!(
            new_user_state.farm_state,
            old_user_state.farm_state,
            FarmError::InvalidTransferOwnershipFarmState
        );
    }

    msg!(
        "Transferring stake ownership of user_state {} owned by {} to user_state {} owned by {}",
        ctx.accounts.old_user_state.key(),
        ctx.accounts.old_owner.key(),
        ctx.accounts.new_user_state.key(),
        new_owner
    );

    if matches!(new_user_account_state, AccountLoaderState::Zeroed) {
       
        new_user_state.bump = new_user_state_bump;
        new_user_state.delegatee = ctx.accounts.new_owner.key();

        farm_operations::initialize_user(
            farm_state,
            &mut new_user_state,
            &new_owner,
            &ctx.accounts.farm_state.key(),
            timestamp,
        )?;
    }

   
    farm_operations::unstake(
        farm_state,
        old_user_state,
        scope_price,
        old_user_state_stake,
        timestamp,
    )?;

   
    let WithdrawEffects { amount_to_withdraw } =
        farm_operations::withdraw_unstaked_deposits(farm_state, old_user_state, timestamp)?;

   
    let StakeEffects { amount_to_stake } = farm_operations::stake(
        farm_state,
        &mut new_user_state,
        scope_price,
        amount_to_withdraw,
        timestamp,
    )?;

    require_eq!(
        amount_to_stake,
        amount_to_withdraw,
        FarmError::InvalidTransferOwnershipStakeAmount
    );

    msg!(
        "Transferring stake of {} tokens from user_state {} to user_state {}",
        amount_to_stake,
        ctx.accounts.old_user_state.key(),
        ctx.accounts.new_user_state.key()
    );

    Ok(())
}

#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    #[account(mut,
      address = old_user_state.load()?.owner @ FarmError::InvalidTransferOwnershipOldOwner,
    )]
    pub old_owner: Signer<'info>,

   
    pub new_owner: AccountInfo<'info>,

    #[account(mut)]
    pub old_user_state: AccountLoader<'info, UserState>,

    #[account(init_if_needed,
        seeds = [BASE_SEED_USER_STATE, farm_state.key().as_ref(), new_owner.key().as_ref()],
        bump,
        payer = old_owner,
        space = SIZE_USER_STATE,
    )]
    pub new_user_state: AccountLoader<'info, UserState>,

    #[account(mut,
        constraint = farm_state.key() == old_user_state.load()?.farm_state @ FarmError::InvalidTransferOwnershipFarmState,
    )]
    pub farm_state: AccountLoader<'info, FarmState>,

    /// CHECK: Farm checks this
    pub scope_prices: Option<AccountLoader<'info, scope::OraclePrices>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
