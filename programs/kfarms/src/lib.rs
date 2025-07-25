#![allow(clippy::result_large_err)]
pub mod farm_operations;
mod handlers;
pub mod stake_operations;
pub mod state;
mod token_operations;
mod types;
pub mod utils;

use crate::handlers::*;
use anchor_lang::prelude::*;
use decimal_wad::decimal::Decimal;
use decimal_wad::error::DecimalError;
use num_derive::FromPrimitive;
use state::*;
use thiserror::Error;

#[cfg(all(
    feature = "mainnet",
    any(feature = "devnet", feature = "localnet", feature = "test-bpf",)
))]
compile_error!("feature \"mainnet\" is incompatible with any other feature");

declare_id!("FarmsPZpWu9i7Kky8tPN37rs2TpmMrAZrC7S7vJa91Hr");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Kamino Farms",
    project_url: "https://kamino.finance/",
    contacts: "email:security@kamino.finance",
    policy: "https://github.com/Kamino-Finance/audits/blob/master/docs/SECURITY.md",

       preferred_languages: "en",
    auditors: "OtterSec, Offside Labs"
}

#[program]
pub mod farms {
    use super::*;

    pub fn initialize_global_config(ctx: Context<InitializeGlobalConfig>) -> Result<()> {
        handler_initialize_global_config::process(ctx)
    }

    pub fn update_global_config(
        ctx: Context<UpdateGlobalConfig>,
        mode: u8,
        value: [u8; 32],
    ) -> Result<()> {
        let mode =
            GlobalConfigOption::try_from(mode).map_err(|_| FarmError::InvalidGlobalConfigMode)?;
        handler_update_global_config::process(ctx, mode, &value)
    }

    pub fn initialize_farm(ctx: Context<InitializeFarm>) -> Result<()> {
        handler_initialize_farm::process(ctx)
    }

    pub fn initialize_farm_delegated(ctx: Context<InitializeFarmDelegated>) -> Result<()> {
        handler_initialize_farm_delegated::process(ctx)
    }

    pub fn initialize_reward(ctx: Context<InitializeReward>) -> Result<()> {
        handler_initialize_reward::process(ctx)
    }

    pub fn add_rewards(ctx: Context<AddReward>, amount: u64, reward_index: u64) -> Result<()> {
        handler_add_reward::process(ctx, amount, reward_index)
    }

    pub fn update_farm_config(
        ctx: Context<UpdateFarmConfig>,
        mode: u16,
        data: Vec<u8>,
    ) -> Result<()> {
        handler_update_farm_config::process(ctx, mode, &data)
    }

    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        handler_initialize_user::process(ctx)
    }

    pub fn transfer_ownership(ctx: Context<TransferOwnership>) -> Result<()> {
        handler_transfer_ownership::process(ctx)
    }

    pub fn reward_user_once(
        ctx: Context<RewardUserOnce>,
        reward_index: u64,
        amount: u64,
    ) -> Result<()> {
        handler_reward_user_once::process(ctx, reward_index, amount)
    }

    pub fn refresh_farm(ctx: Context<RefreshFarm>) -> Result<()> {
        handler_refresh_farm::process(ctx)
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        handler_stake::process(ctx, amount)
    }

    pub fn set_stake_delegated(ctx: Context<SetStakeDelegated>, new_amount: u64) -> Result<()> {
        handler_set_stake_delegated::process(ctx, new_amount)
    }

    pub fn harvest_reward(ctx: Context<HarvestReward>, reward_index: u64) -> Result<()> {
        handler_harvest_reward::process(ctx, reward_index)
    }

    pub fn unstake(ctx: Context<Unstake>, stake_shares_scaled: u128) -> Result<()> {
        handler_unstake::process(ctx, Decimal::from_scaled_val(stake_shares_scaled))
    }

    pub fn refresh_user_state(ctx: Context<RefreshUserState>) -> Result<()> {
        handler_refresh_user_state::process(ctx)
    }

    pub fn withdraw_unstaked_deposits(ctx: Context<WithdrawUnstakedDeposits>) -> Result<()> {
        handler_withdraw_unstaked_deposits::process(ctx)
    }

    pub fn withdraw_treasury(ctx: Context<WithdrawTreasury>, amount: u64) -> Result<()> {
        handler_withdraw_treasury::process(ctx, amount)
    }

    pub fn deposit_to_farm_vault(ctx: Context<DepositToFarmVault>, amount: u64) -> Result<()> {
        handler_deposit_to_farm_vault::process(ctx, amount)
    }

    pub fn withdraw_from_farm_vault(
        ctx: Context<WithdrawFromFarmVault>,
        amount: u64,
    ) -> Result<()> {
        handler_withdraw_from_farm_vault::process(ctx, amount)
    }

    pub fn withdraw_slashed_amount(ctx: Context<WithdrawSlashedAmount>) -> Result<()> {
        handler_withdraw_slashed_amount::process(ctx)
    }

    pub fn update_farm_admin(ctx: Context<UpdateFarmAdmin>) -> Result<()> {
        handler_update_farm_admin::process(ctx)
    }

    pub fn update_global_config_admin(ctx: Context<UpdateGlobalConfigAdmin>) -> Result<()> {
        handler_update_global_config_admin::process(ctx)
    }

    pub fn withdraw_reward(
        ctx: Context<WithdrawReward>,
        amount: u64,
        reward_index: u64,
    ) -> Result<()> {
        handler_withdraw_reward::process(ctx, amount, reward_index)
    }

    pub fn update_second_delegated_authority(
        ctx: Context<UpdateSecondDelegatedAuthority>,
    ) -> Result<()> {
        handler_update_second_delegated_authority::process(ctx)
    }

    pub fn idl_missing_types(
        _ctx: Context<UpdateGlobalConfig>,
        _global_config_option_kind: GlobalConfigOption,
        _farm_config_option_kind: FarmConfigOption,
        _time_unit: TimeUnit,
        _locking_mode: LockingMode,
        _reward_type: RewardType,
    ) -> Result<()> {
        unreachable!("This should never be called")
    }
}

#[error_code]
#[derive(Error, PartialEq, Eq, FromPrimitive)]
pub enum FarmError {
    #[msg("Cannot stake 0 amount")]
    StakeZero,
    #[msg("Cannot unstake 0 amount")]
    UnstakeZero,
    #[msg("Nothing to unstake")]
    NothingToUnstake,
    #[msg("No reward to harvest")]
    NoRewardToHarvest,
    #[msg("Reward not present in reward list")]
    NoRewardInList,
    #[msg("Reward already initialized")]
    RewardAlreadyInitialized,
    #[msg("Max number of reward tokens reached")]
    MaxRewardNumberReached,
    #[msg("Reward does not exist")]
    RewardDoesNotExist,
    #[msg("Reward vault exists but the account is wrong")]
    WrongRewardVaultAccount,
    #[msg("Reward vault pubkey does not match staking pool vault")]
    RewardVaultMismatch,
    #[msg("Reward vault authority pubkey does not match staking pool vault")]
    RewardVaultAuthorityMismatch,
    #[msg("Nothing staked, cannot collect any rewards")]
    NothingStaked,
    #[msg("Integer overflow")]
    IntegerOverflow,
    #[msg("Conversion failure")]
    ConversionFailure,
    #[msg("Unexpected account in instruction")]
    UnexpectedAccount,
    #[msg("Operation forbidden")]
    OperationForbidden,
    #[msg("Mathematical operation with overflow")]
    MathOverflow,
    #[msg("Minimum claim duration has not been reached")]
    MinClaimDurationNotReached,
    #[msg("Reward vault has a delegate")]
    RewardsVaultHasDelegate,
    #[msg("Reward vault has a close authority")]
    RewardsVaultHasCloseAuthority,
    #[msg("Farm vault has a delegate")]
    FarmVaultHasDelegate,
    #[msg("Farm vault has a close authority")]
    FarmVaultHasCloseAuthority,
    #[msg("Reward vault has a delegate")]
    RewardsTreasuryVaultHasDelegate,
    #[msg("Reward vault has a close authority")]
    RewardsTreasuryVaultHasCloseAuthority,
    #[msg("User ata and reward vault have different mints")]
    UserAtaRewardVaultMintMissmatch,
    #[msg("User ata and farm token have different mints")]
    UserAtaFarmTokenMintMissmatch,
    #[msg("Token mint and farm token have different mints")]
    TokenFarmTokenMintMissmatch,
    #[msg("Reward ata mint is different than reward mint")]
    RewardAtaRewardMintMissmatch,
    #[msg("Reward ata owner is different than payer")]
    RewardAtaOwnerNotPayer,
    #[msg("Mode to update global_config is invalid")]
    InvalidGlobalConfigMode,
    #[msg("Reward Index is higher than number of rewards")]
    RewardIndexOutOfRange,
    #[msg("No tokens available to withdraw")]
    NothingToWithdraw,
    #[msg("user, user_ref, authority and payer must match for non-delegated farm")]
    UserDelegatedFarmNonDelegatedMissmatch,
    #[msg("Authority must match farm delegate authority")]
    AuthorityFarmDelegateMissmatch,
    #[msg("Farm not delegated, can not set stake")]
    FarmNotDelegated,
    #[msg("Operation not allowed for delegated farm")]
    FarmDelegated,
    #[msg("Unstake lockup period is not elapsed. Deposit is locked until end of unstake period")]
    UnstakeNotElapsed,
    #[msg("Pending withdrawal already exist and not withdrawn yet")]
    PendingWithdrawalNotWithdrawnYet,
    #[msg("Cannot deposit zero amount directly to farm vault")]
    DepositZero,
    #[msg("Invalid config value")]
    InvalidConfigValue,
    #[msg("Invalid penalty percentage")]
    InvalidPenaltyPercentage,
    #[msg("Early withdrawal not allowed")]
    EarlyWithdrawalNotAllowed,
    #[msg("Invalid locking timestamps")]
    InvalidLockingTimestamps,
    #[msg("Invalid reward rate curve point")]
    InvalidRpsCurvePoint,
    #[msg("Invalid timestamp")]
    InvalidTimestamp,
    #[msg("Deposit cap reached")]
    DepositCapReached,
    #[msg("Missing Scope Prices")]
    MissingScopePrices,
    #[msg("Scope Oracle Price Too Old")]
    ScopeOraclePriceTooOld,
    #[msg("Invalid Oracle Config")]
    InvalidOracleConfig,
    #[msg("Could not deserialize scope")]
    CouldNotDeserializeScope,
    #[msg("Reward ata owner is different than farm admin")]
    RewardAtaOwnerNotAdmin,
    #[msg("Cannot withdraw reward as available amount is zero")]
    WithdrawRewardZeroAvailable,
    #[msg("Cannot withdraw reward as reward schedule is set")]
    RewardScheduleCurveSet,
    #[msg("Cannot initialize farm while having a mint with token22 and requested extensions")]
    UnsupportedTokenExtension,
    #[msg("Invalid authority for updating farm config")]
    InvalidFarmConfigUpdateAuthority,
    #[msg("Invalid authority for transfer ownersip new user state initialization")]
    InvalidTransferOwnershipOldOwner,
    #[msg("Invalid farm state for transfer ownership new user state initialization")]
    InvalidTransferOwnershipFarmState,
    #[msg("Invalid user state for transfer ownership, owner must match delegatee")]
    InvalidTransferOwnershipUserStateOwnerDelegatee,
    #[msg("Invalid farm state locking mode for transfer ownership, must be 0")]
    InvalidTransferOwnershipFarmStateLockingMode,
    #[msg("Invalid farm state withdrawal cooldown period for transfer ownership, must be 0")]
    InvalidTransferOwnershipFarmStateWithdrawCooldownPeriod,
    #[msg("Invalid transfer ownership stake amount, must be equal to unstaked deposits")]
    InvalidTransferOwnershipStakeAmount,
    #[msg("Invalid authority for transfer ownersip new user state initialization")]
    InvalidTransferOwnershipNewOwner,
}

impl From<DecimalError> for FarmError {
    fn from(err: DecimalError) -> FarmError {
        match err {
            DecimalError::MathOverflow => FarmError::MathOverflow,
        }
    }
}

pub type FarmResult<T> = std::result::Result<T, FarmError>;
