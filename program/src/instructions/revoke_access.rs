use anchor_lang::prelude::*;
use crate::{
    error::VaultError,
    state::vault::{AccessRevoked, EphemeralVault, VaultDelegation},
};

#[derive(Accounts)]
pub struct RevokeAccess<'info> {
    #[account(mut)]
    pub parent_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        has_one = parent_wallet @ VaultError::UnauthorizedDelegation
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        mut,
        seeds = [b"delegation", vault.key().as_ref()],
        bump = vault_delegation.bump
    )]
    pub vault_delegation: Account<'info, VaultDelegation>,
}

pub fn handler(ctx: Context<RevokeAccess>) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;
    let delegation = &mut ctx.accounts.vault_delegation;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(vault.parent_wallet == ctx.accounts.parent_wallet.key(), VaultError::UnauthorizedAccess);

    let vault_info = vault.to_account_info();
    let parent_info = ctx.accounts.parent_wallet.to_account_info();

    let rent = Rent::get()?.minimum_balance(vault_info.data_len());
    let current_balance = vault_info.lamports();
    let refund_amount = current_balance.saturating_sub(rent);

    if refund_amount > 0 {
        **vault_info.try_borrow_mut_lamports()? -= refund_amount;
        **parent_info.try_borrow_mut_lamports()? += refund_amount;
    }

    vault.is_active = false;
    vault.session_expiry = clock.unix_timestamp;
    vault.last_activity = clock.unix_timestamp;
    vault.ephemeral_wallet = Pubkey::default();
    vault.total_deposited = vault.total_spent;

    delegation.is_active = false;
    delegation.revoked_at = clock.unix_timestamp;

    emit!(AccessRevoked {
        vault: vault.key(),
        refund_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
