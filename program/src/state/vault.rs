use anchor_lang::prelude::*;

pub const MAX_SESSION_DURATION_SECONDS: i64 = 24 * 60 * 60; // 24h upper-bound
pub const CLEANUP_REWARD_BPS: u64 = 100; // 1%

#[account]
pub struct EphemeralVault {
    pub parent_wallet: Pubkey,
    pub ephemeral_wallet: Pubkey,
    pub session_start: i64,
    pub session_expiry: i64,
    pub last_activity: i64,
    pub approved_amount: u64,
    pub total_deposited: u64,
    pub total_spent: u64,
    pub bump: u8,
    pub is_active: bool,
}

impl EphemeralVault {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 1 + 1;

    pub fn remaining_allowance(&self) -> Option<u64> {
        self.approved_amount.checked_sub(self.total_spent)
    }

    pub fn has_expired(&self, now: i64) -> bool {
        now > self.session_expiry
    }
}

#[account]
pub struct VaultDelegation {
    pub vault: Pubkey,
    pub delegate: Pubkey,
    pub approved_at: i64,
    pub revoked_at: i64,
    pub is_active: bool,
    pub bump: u8,
}

impl VaultDelegation {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 1 + 1;
}

#[event]
pub struct DelegateApproved {
    pub vault: Pubkey,
    pub delegate: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct VaultCreated {
    pub parent_wallet: Pubkey,
    pub vault_pda: Pubkey,
    pub approved_amount: u64,
    pub session_expiry: i64,
    pub timestamp: i64,
}

#[event]
pub struct FundDeposited {
    pub vault: Pubkey,
    pub amount: u64,
    pub total_deposited: u64,
}

#[event]
pub struct TradeExecuted {
    pub vault: Pubkey,
    pub ephemeral_wallet: Pubkey,
    pub trade_amount: u64,
    pub trading_fee: u64,
    pub total_spent: u64,
}

#[event]
pub struct AccessRevoked {
    pub vault: Pubkey,
    pub refund_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct VaultCleaned {
    pub vault: Pubkey,
    pub caller: Pubkey,
    pub parent_refund: u64,
    pub caller_reward: u64,
    pub timestamp: i64,
}
