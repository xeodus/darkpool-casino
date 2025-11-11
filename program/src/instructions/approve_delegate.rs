use anchor_lang::prelude::*;
use crate::{
    error::VaultError,
    state::vault::{DelegateApproved, EphemeralVault, VaultDelegation},
};

#[derive(Accounts)]
pub struct ApproveDelegate<'info> {
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
        init_if_needed,
        payer = parent_wallet,
        space = 8 + VaultDelegation::LEN,
        seeds = [b"delegation", vault.key().as_ref()],
        bump
    )]
    pub vault_delegation: Account<'info, VaultDelegation>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ApproveDelegate>, delegate: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;
    let delegation = &mut ctx.accounts.vault_delegation;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(!vault.has_expired(clock.unix_timestamp), VaultError::SessionExpired);
    require!(delegate != Pubkey::default(), VaultError::UnauthorizedDelegation);

    vault.ephemeral_wallet = delegate;
    vault.last_activity = clock.unix_timestamp;

    delegation.vault = vault.key();
    delegation.delegate = delegate;
    delegation.approved_at = clock.unix_timestamp;
    delegation.revoked_at = 0;
    delegation.is_active = true;
    delegation.bump = *ctx
        .bumps
        .get("vault_delegation")
        .ok_or(VaultError::ArithmeticOverflow)?;

    emit!(DelegateApproved {
        vault: vault.key(),
        delegate,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
