use anchor_lang::prelude::*;

/*#[account]
pub struct VaultDelegation {
    pub vault: Pubkey,
    pub delegate: Pubkey,
    pub approved_at: i64,
    pub revoked_at: i64,
    pub bump: u8
}*/

#[event]
pub struct DelegateApproved {
    pub vault: Pubkey,
    pub delegate: Pubkey,
    pub timestamp: i64
}

#[event]
pub struct VaultCreated {
    pub user_wallet: Pubkey,
    pub vault_pda: Pubkey,
    pub approved_amount: u64,
    pub timestamp: i64
}

#[event]
pub struct FundDeposited {
    pub vault: Pubkey,
    pub amount: u64,
    pub total_deposited: u64
}

#[event]
pub struct TradeExecuted {
    pub vault: Pubkey,
    pub ephemeral_wallet: Pubkey,
    pub trade_amount: u64,
    pub trading_fee: u64,
    pub total_spent: u64
}

#[event]
pub struct AccessRevoked {
    pub vault: Pubkey,
    pub refund_amount: u64,
    pub timestamp: i64
}

#[event]
pub struct VaultCleanup {
    pub vault: Pubkey,
    pub caller: Pubkey,
    pub timestamp: i64
}
