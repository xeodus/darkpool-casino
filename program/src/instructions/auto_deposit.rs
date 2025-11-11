use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use crate::{
    error::VaultError,
    state::vault::{EphemeralVault, FundDeposited},
};

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
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<AutoDeposit>, amount: u64) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(vault.is_active, VaultError::VaultInactive);
    require!(!vault.has_expired(clock.unix_timestamp), VaultError::SessionExpired);
    require!(amount > 0, VaultError::InvalidAmount);

    let new_total = vault
        .total_deposited
        .checked_add(amount)
        .ok_or(VaultError::ArithmeticOverflow)?;
    require!(
        new_total <= vault.approved_amount,
        VaultError::InvalidSpendingLimit
    );

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.parent_wallet.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        ),
        amount,
    )?;

    vault.total_deposited = new_total;
    vault.last_activity = clock.unix_timestamp;

    emit!(FundDeposited {
        vault: vault.key(),
        amount,
        total_deposited: new_total,
    });

    Ok(())
}
