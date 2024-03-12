#[derive(Debug)]
pub struct HarvestEffects {
    pub reward_user: u64,
    pub reward_treasury: u64,
}

#[derive(Debug)]
pub struct WithdrawEffects {
    pub amount_to_withdraw: u64,
}

pub struct AddRewardEffects {
    pub reward_amount: u64,
}

pub struct WithdrawRewardEffects {
    pub reward_amount: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StakeEffects {
    pub amount_to_stake: u64,
}

#[derive(Debug)]
pub struct VaultWithdrawEffects {
    pub amount_to_withdraw: u64,
    pub farm_to_freeze: bool,
}
