use anchor_lang::prelude::*;
use crate::{error::VaultError, 
    instructions::create_vault::EphemeralVault, 
    state::vault::{AccessRevoked}
};

#[derive(Accounts)]
pub struct RevokeAccess<'info> {
    pub parent_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        has_one = parent_wallet @ VaultError::UnauthorizedDelegation
    )]
    pub vault: Account<'info, EphemeralVault>,
}

pub fn access_handler(ctx: Context<RevokeAccess>) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

    vault.is_active = false;

    let vault_lamports = vault.to_account_info().lamports();
    let rent = Rent::get()?.minimum_balance(vault.to_account_info().data_len());

    if vault_lamports > rent {
        let refund_amount = vault_lamports.checked_sub(rent)
            .ok_or(VaultError::ArithmeticOverflow)?;
        **vault.to_account_info().try_borrow_mut_lamports()? -= refund_amount;
        **ctx.accounts.parent_wallet.to_account_info().try_borrow_mut_lamports()? += refund_amount;
    }

    emit!({
        AccessRevoked {
            vault: vault.key(),
            refund_amount: vault_lamports.saturating_sub(rent),
            timestamp: clock.unix_timestamp
        }
    });

    Ok(())
}
