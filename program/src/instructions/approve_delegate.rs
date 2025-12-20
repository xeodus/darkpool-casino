use anchor_lang::prelude::*;
use crate::{error::VaultError, 
    instructions::create_vault::EphemeralVault, 
    state::vault::DelegateApproved
};

#[derive(Accounts)]
pub struct ApproveDelegates<'info> {
    #[account(mut)]
    pub parent_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        has_one = parent_wallet @ VaultError::UnauthorizedDelegation
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub system_program: Program<'info, System>
}

pub fn delegation_handler(ctx: Context<ApproveDelegates>, delegate: Pubkey) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(vault.parent_wallet == ctx.accounts.parent_wallet.key(), VaultError::SessionExpired);
    require!(vault.approved_amount > 0, VaultError::InvalidAmount);
    require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

    vault.ephemeral_wallet = delegate;
    vault.delegate_approved = true;

    emit!({
        DelegateApproved {
            vault: vault.key(),
            delegate,
            timestamp: clock.unix_timestamp
        }
    });

    Ok(())
}
