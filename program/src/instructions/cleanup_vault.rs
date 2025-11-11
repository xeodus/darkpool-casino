use anchor_lang::prelude::*;
use crate::{
    error::VaultError,
    state::vault::{EphemeralVault, VaultCleaned, VaultDelegation, CLEANUP_REWARD_BPS},
};

#[derive(Accounts)]
pub struct CleanupVault<'info> {
    #[account(mut)]
    pub cleaner: Signer<'info>,
    #[account(mut)]
    pub parent_wallet: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        close = parent_wallet
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        mut,
        seeds = [b"delegation", vault.key().as_ref()],
        bump = vault_delegation.bump,
        close = parent_wallet
    )]
    pub vault_delegation: Account<'info, VaultDelegation>,
}

pub fn handler(ctx: Context<CleanupVault>) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;
    let delegation = &mut ctx.accounts.vault_delegation;

    require!(
        vault.has_expired(clock.unix_timestamp) || !vault.is_active,
        VaultError::SessionExpired
    );
    require_keys_eq!(
        ctx.accounts.parent_wallet.key(),
        vault.parent_wallet,
        VaultError::UnauthorizedAccess
    );

    let vault_info = vault.to_account_info();
    let parent_info = ctx.accounts.parent_wallet.to_account_info();
    let cleaner_info = ctx.accounts.cleaner.to_account_info();

    let rent = Rent::get()?.minimum_balance(vault_info.data_len());
    let current_balance = vault_info.lamports();
    let available_for_refund = current_balance.saturating_sub(rent);

    let caller_reward = (available_for_refund * CLEANUP_REWARD_BPS) / 10_000;
    let parent_refund = available_for_refund
        .checked_sub(caller_reward)
        .ok_or(VaultError::ArithmeticOverflow)?;

    if parent_refund > 0 {
        **vault_info.try_borrow_mut_lamports()? -= parent_refund;
        **parent_info.try_borrow_mut_lamports()? += parent_refund;
    }

    if caller_reward > 0 {
        **vault_info.try_borrow_mut_lamports()? -= caller_reward;
        **cleaner_info.try_borrow_mut_lamports()? += caller_reward;
    }

    vault.is_active = false;
    vault.total_deposited = vault.total_spent;
    vault.ephemeral_wallet = Pubkey::default();
    vault.last_activity = clock.unix_timestamp;

    delegation.is_active = false;
    delegation.revoked_at = clock.unix_timestamp;

    emit!(VaultCleaned {
        vault: vault.key(),
        caller: ctx.accounts.cleaner.key(),
        parent_refund,
        caller_reward,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
