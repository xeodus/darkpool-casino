use anchor_lang::prelude::*;
use crate::{error::VaultError, 
    instructions::create_vault::EphemeralVault, 
    state::vault::VaultCleanup
};

#[derive(Accounts)]
pub struct CleanupVault<'info> {
    #[account(mut)]
    pub cleanup_caller: AccountInfo<'info>,
    #[account(mut)]
    pub parent_wallet: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref(), vault.ephemeral_wallet.key().as_ref()],
        bump = vault.bump,
        close = parent_wallet
    )]
    pub vault: Account<'info, EphemeralVault>
}

pub fn handler(ctx: Context<CleanupVault>) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

    vault.is_active = false;
    let vault_lamports = vault.to_account_info().lamports();
    let rent = Rent::get()?.minimum_balance(vault.to_account_info().data_len());

    if vault_lamports > rent {
        let total_refund = vault_lamports.checked_sub(rent).ok_or(VaultError::ArithmeticOverflow)?;
        let caller_reward = total_refund / 100;
        let parent_refund = total_refund.checked_sub(caller_reward).ok_or(VaultError::ArithmeticOverflow)?;

        **vault.to_account_info().try_borrow_mut_lamports()? -= total_refund;
        **ctx.accounts.parent_wallet.to_account_info().try_borrow_mut_lamports()? += parent_refund;
        **ctx.accounts.cleanup_caller.to_account_info().try_borrow_mut_lamports()? += caller_reward;
    }

    emit!({
        VaultCleanup {
            vault: vault.key(),
            caller: ctx.accounts.cleanup_caller.key(),
            timestamp: clock.unix_timestamp
        }
    });

    Ok(())
}