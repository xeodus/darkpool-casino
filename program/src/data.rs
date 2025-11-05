use anchor_lang::prelude::*;

#[account]
pub struct EphemeralVault {
    pub parent_wallet: Pubkey,
    pub vault_pda: Pubkey,
    pub created_at: i64,
    pub last_activity: i64,
    pub approved_amount: u64,
    pub used_amount: u64,
    pub available_amount: u64,
    pub is_active: bool,
    pub bump: u8
}

#[account]
pub struct VaultDelegation {
    pub vault: Pubkey,
    pub delegate: Pubkey,
    pub approved_at: i64,
    pub revoked_at: Option<i64>,
    pub bump: u8
}

#[derive(Accounts)]
pub struct CreateEphemeralVault<'info> {
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    #[account(
        init,
        payer = user_wallet,
        space = 8 + std::mem::size_of::<EphemeralVault>(),
        seeds = [b"vault", user_wallet.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct ApproveDelegates<'info> {
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    #[account(
        init,
        payer = user_wallet,
        space = 8 + std::mem::size_of::<VaultDelegation>(),
        seeds = [b"delegation", vault.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub delegation: Account<'info, VaultDelegation>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct AutoDeposite<'info> {
    #[account(mut)]
    pub user_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", user_wallet.key().as_ref()],
        bump = vault.bump,
        constraint = vault.parent_wallet == user_wallet.key() @ VaultError::UnauthorizedAccess
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct ExecuteTrade<'info> {
    pub ephemeral_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.parent_wallet.key().as_ref(), ephemeral_wallet.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        seeds = [b"delegation", vault.key().as_ref()],
        bump = delegation.bump
    )]
    pub delegation: Account<'info, VaultDelegation>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct RevokeAccess<'info> {
    pub user_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", user_wallet.key().as_ref(), vault.vault_pda.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        mut,
        seeds = [b"delegation", vault.key().as_ref()],
        bump = delegation.bump
    )]
    pub delegation: Account<'info, VaultDelegation>
}

#[derive(Accounts)]
pub struct CleanupVault<'info> {
    #[account(mut)]
    pub cleanup_caller: AccountInfo<'info>,
    #[account(mut)]
    pub user_wallet: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"vault", user_wallet.key().as_ref(), vault.vault_pda.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, EphemeralVault>
}

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

#[error_code]
pub enum VaultError {
    #[msg("Invalid session duration!")]
    InvalidSessionDuration,
    #[msg("Invalid spending limit!")]
    InvalidSpendingLimit,
    #[msg("Invalid amount!")]
    InvalidAmount,
    #[msg("Vault is inactive!")]
    VaultInactive,
    #[msg("Session expired")]
    SessionExpired,
    #[msg("Access denied")]
    UnauthorizedAccess,
    #[msg("Invalid deposited amount")]
    ArithmeticOverflow,
    #[msg("Delegation revoked")]
    UnauthorizedDelegation
}
