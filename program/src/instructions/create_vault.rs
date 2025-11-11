use anchor_lang::prelude::*;
use crate::{
    error::VaultError,
    state::vault::{EphemeralVault, VaultCreated, MAX_SESSION_DURATION_SECONDS},
};

#[derive(Accounts)]
pub struct CreateEphemeralVault<'info> {
    #[account(mut)]
    pub parent_wallet: Signer<'info>,
    #[account(
        init,
        payer = parent_wallet,
        space = 8 + EphemeralVault::LEN,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateEphemeralVault>,
    approved_amount: u64,
    session_duration: i64,
) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(session_duration > 0, VaultError::InvalidSessionDuration);
    require!(
        session_duration <= MAX_SESSION_DURATION_SECONDS,
        VaultError::InvalidSessionDuration
    );
    require!(approved_amount > 0, VaultError::InvalidSpendingLimit);

    let session_expiry = clock
        .unix_timestamp
        .checked_add(session_duration)
        .ok_or(VaultError::ArithmeticOverflow)?;

    vault.parent_wallet = ctx.accounts.parent_wallet.key();
    vault.ephemeral_wallet = Pubkey::default();
    vault.session_start = clock.unix_timestamp;
    vault.session_expiry = session_expiry;
    vault.last_activity = clock.unix_timestamp;
    vault.approved_amount = approved_amount;
    vault.total_deposited = 0;
    vault.total_spent = 0;
    vault.bump = *ctx
        .bumps
        .get("vault")
        .ok_or(VaultError::ArithmeticOverflow)?;
    vault.is_active = true;

    emit!(VaultCreated {
        parent_wallet: ctx.accounts.parent_wallet.key(),
        vault_pda: ctx.accounts.vault.key(),
        approved_amount,
        session_expiry,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
