use anchor_lang::prelude::*;
use crate::{error::VaultError, instructions::create_vault::EphemeralVault, state::vault::TradeExecuted};

#[derive(Accounts)]
pub struct ExecuteTrade<'info> {
    pub ephemeral_wallet: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault.parent_wallet.key().as_ref(), ephemeral_wallet.key().as_ref()],
        bump = vault.bump
    )]
    pub vault: Account<'info, EphemeralVault>,
    #[account(
        seeds = [b"delegation", vault.key().as_ref()],
        bump = vault.bump
    )]
    pub system_program: Program<'info, System>
}

pub fn execution_handler(ctx: Context<ExecuteTrade>, trade_amount: u64, trading_fee: u64) -> Result<()> {
    let clock = Clock::get()?;
    let vault = &mut ctx.accounts.vault;

    require!(trade_amount > 0, VaultError::InvalidAmount);
    require!(vault.is_active, VaultError::VaultInactive);
    require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);
    require!(ctx.accounts.ephemeral_wallet.key() == vault.ephemeral_wallet, VaultError::UnauthorizedAccess);

    let total_cost = trade_amount.checked_add(trading_fee).ok_or(VaultError::ArithmeticOverflow)?;
    let new_total_spent = vault.used_amount.checked_add(total_cost).ok_or(VaultError::ArithmeticOverflow)?;

    require!(new_total_spent <= vault.approved_amount, VaultError::ArithmeticOverflow);

    vault.used_amount = new_total_spent;
    emit!({
        TradeExecuted {
            vault: vault.key(),
            ephemeral_wallet: ctx.accounts.ephemeral_wallet.key(),
            trade_amount,
            trading_fee,
            total_spent: vault.used_amount
        }
    });

    Ok(())
}
