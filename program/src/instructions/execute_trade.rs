use anchor_lang::prelude::*;
use crate::{
    error::VaultError,
    state::vault::{EphemeralVault, TradeExecuted, VaultDelegation},
};

#[derive(Accounts)]
pub struct ExecuteTrade<'info> {
    pub delegate: Signer<'info>,
    /// CHECK: Parent wallet is used to validate vault seed; no data access
    pub parent_wallet: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        constraint = vault.parent_wallet == parent_wallet.key() @ VaultError::UnauthorizedAccess
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        mut,
        seeds = [b"delegation", vault.key().as_ref()],
        bump = vault_delegation.bump,
        constraint = vault_delegation.is_active @ VaultError::UnauthorizedDelegation,
        constraint = vault_delegation.delegate == delegate.key() @ VaultError::UnauthorizedAccess
    )]
    pub vault_delegation: Account<'info, VaultDelegation>,
}

pub fn handler(
    ctx: Context<ExecuteTrade>,
    trade_amount: u64,
    trading_fee: u64,
) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(!vault.has_expired(clock.unix_timestamp), VaultError::SessionExpired);
    require_keys_eq!(
        ctx.accounts.delegate.key(),
        vault.ephemeral_wallet,
        VaultError::UnauthorizedAccess
    );
    require!(trade_amount > 0, VaultError::InvalidAmount);

    let total_cost = trade_amount
        .checked_add(trading_fee)
        .ok_or(VaultError::ArithmeticOverflow)?;

    let new_total_spent = vault
        .total_spent
        .checked_add(total_cost)
        .ok_or(VaultError::ArithmeticOverflow)?;
    require!(
        new_total_spent <= vault.approved_amount,
        VaultError::InvalidSpendingLimit
    );
    require!(
        new_total_spent <= vault.total_deposited,
        VaultError::InvalidAmount
    );

    vault.total_spent = new_total_spent;
    vault.last_activity = clock.unix_timestamp;

    emit!(TradeExecuted {
        vault: vault.key(),
        ephemeral_wallet: ctx.accounts.delegate.key(),
        trade_amount,
        trading_fee,
        total_spent: vault.total_spent,
    });

    Ok(())
}
