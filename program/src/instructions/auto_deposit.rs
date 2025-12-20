use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use crate::{error::VaultError, instructions::create_vault::EphemeralVault, state::vault::FundDeposited};

#[derive(Accounts)]
pub struct AutoDeposit<'info> {
    #[account(mut)]
    pub parent_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", parent_wallet.key().as_ref()],
        bump = vault.bump,
        constraint = vault.parent_wallet == parent_wallet.key() @ VaultError::UnauthorizedAccess
    )]
    pub vault: Account<'info, EphemeralVault>,
    pub system_program: Program<'info, System>
}

pub fn deposition_handler(ctx: Context<AutoDeposit>, amount: u64) -> Result<()> {
    const MAX_DEPOSITE_PER_SESSION: u64 = 100_000_000;
    let clock = Clock::get()?;
    let vault = &ctx.accounts.vault;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(amount > 0, VaultError::InvalidAmount);
    require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);
    require!(vault.approved_amount < MAX_DEPOSITE_PER_SESSION, VaultError::UnauthorizedDelegation);

    let new_amount = vault.approved_amount.checked_add(amount).ok_or(VaultError::ArithmeticOverflow)?;

    transfer(CpiContext::new(
        ctx.accounts.system_program.to_account_info(), 
        Transfer {
            from: ctx.accounts.parent_wallet.to_account_info(),
            to: ctx.accounts.vault.to_account_info()
        }),
        amount
    )?;

    emit!({
        FundDeposited {
            vault: vault.key(),
            amount,
            total_deposited: new_amount
        }
    });

    Ok(())
}
