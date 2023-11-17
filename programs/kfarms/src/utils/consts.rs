pub const MAX_REWARDS_TOKENS: usize = 10;
pub const REWARD_CURVE_POINTS: usize = 20;
pub const BPS_DIV_FACTOR: u64 = 10_000;

pub const BASE_SEED_FARM_VAULT: &[u8; 6] = b"fvault";
pub const BASE_SEED_REWARD_VAULT: &[u8; 6] = b"rvault";
pub const BASE_SEED_REWARD_TREASURY_VAULT: &[u8; 6] = b"tvault";
pub const BASE_SEED_FARM_VAULTS_AUTHORITY: &[u8; 9] = b"authority";
pub const BASE_SEED_TREASURY_VAULTS_AUTHORITY: &[u8; 9] = b"authority";
pub const BASE_SEED_USER_STATE: &[u8; 4] = b"user";

pub const SIZE_GLOBAL_CONFIG: usize = 2136;
pub const SIZE_FARM_STATE: usize = 8336;
pub const SIZE_USER_STATE: usize = 920;
