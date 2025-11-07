use anchor_lang::prelude::*;
use crate::{error::VaultError, state::vault::VaultCreated};

#[account]
pub struct EphemeralVault {
    pub parent_wallet: Pubkey,
    pub ephemeral_wallet: Pubkey,
    pub created_at: i64,
    pub last_activity: i64,
    pub approved_amount: u64,
    pub delegate_approved: bool,
    pub used_amount: u64,
    pub available_amount: u64,
    pub is_active: bool,
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

pub fn handler(
    ctx: Context<CreateEphemeralVault>, 
    approved_amount: u64, session_duration: i64) -> Result<()> 
{
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(session_duration > 0 && session_duration <= 86400, VaultError::SessionExpired);
    require!(approved_amount > 0, VaultError::InvalidAmount);

    vault.parent_wallet = ctx.accounts.user_wallet.key();
    vault.ephemeral_wallet = vault.key();
    vault.created_at = clock.unix_timestamp;
    vault.last_activity = clock.unix_timestamp
        .checked_add(session_duration)
        .ok_or(VaultError::ArithmeticOverflow)?;
    vault.approved_amount = approved_amount;
    vault.used_amount = 0;
    vault.available_amount = 0;
    vault.is_active = true;
    vault.bump = ctx.bumps.vault;

    emit!({
        VaultCreated {
            user_wallet: ctx.accounts.user_wallet.key(),
            vault_pda: ctx.accounts.vault.key(),
            approved_amount,
            timestamp: clock.unix_timestamp
        }
    });

    Ok(())
}