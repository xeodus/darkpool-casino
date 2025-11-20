use anchor_lang::error_code;

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
