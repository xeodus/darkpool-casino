use anchor_lang::prelude::*;
use anchor_lang::prelude::Context;
use crate::solana_core::data::{CreateEphemeralVault, VaultCreated};
use anchor_lang::Result;

pub mod ephemeral_vault {
    use crate::solana_core::data::{AccessRevoked, ApproveDelegates, AutoDeposite, 
        CleanupVault, DelegateApproved, ExecuteTrade, FundDeposited, RevokeAccess, 
        TradeExecuted, VaultCleanup, VaultError
    };

    use super::*;

    pub fn create_ephemeral_vault(ctx: Context<CreateEphemeralVault>, approved_amount: u64, session_duration: i64) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;

        require!(session_duration > 0 && session_duration <= 43200, VaultError::SessionExpired);
        require!(approved_amount > 0, VaultError::InvalidAmount);

        vault.parent_wallet = ctx.accounts.user_wallet.key();
        vault.vault_pda = vault.key();
        vault.created_at = clock.unix_timestamp;
        vault.last_activity = clock.unix_timestamp;
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

    /*pub fn create_vault(ctx: Context<CreateEphemeralVault>, session_duration: i64) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;
        let max_spending_limit = 0;

        require!(session_duration > 0 && session_duration <= 3600, VaultError::InvalidSessionDuration);
        require!(max_spending_limit > 0, VaultError::InvalidAmount);

        vault.parent_wallet = ctx.accounts.user_wallet.key();
        vault.vault_pda = vault.key();
        vault.created_at = clock.unix_timestamp;
        vault.last_activity = clock.unix_timestamp;
        vault.approved_amount = 0;
        vault.used_amount = 0;
        vault.available_amount = 0;
        vault.is_active = true;
        vault.bump = ctx.bumps.vault;

        emit!({
            VaultCreated {
                user_wallet: ctx.accounts.user_wallet.key(),
                vault_pda: vault.key(),
                approved_amount: 0,
                timestamp: clock.unix_timestamp
            }
        });

        Ok(())
    }*/

    pub fn approved_delegates(ctx: Context<ApproveDelegates>, delegate: Pubkey) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;
        let delegates = &mut ctx.accounts.delegation;

        require!(vault.is_active, VaultError::VaultInactive);
        require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

        delegates.vault = vault.key();
        delegates.delegate = delegate;
        delegates.approved_at = clock.unix_timestamp;
        delegates.revoked_at = None;

        emit!({
            DelegateApproved {
                vault: vault.key(),
                delegate,
                timestamp: clock.unix_timestamp
            }
        });

        Ok(())
    }

    pub fn auto_deposite(ctx: Context<AutoDeposite>, trade_fee_estimate: u64) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;

        require!(vault.is_active, VaultError::VaultInactive);
        require!(trade_fee_estimate > 0, VaultError::InvalidAmount);
        require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

        // Transaction, sending SOL from parent wallet to ephemeral wallet
        let ix = anchor_lang::solana_program::system_instruction::transfer(&ctx.accounts.user_wallet.key(), 
            &vault.key(), trade_fee_estimate);
        
        anchor_lang::solana_program::program::invoke(
            &ix, 
            &[
                ctx.accounts.user_wallet.to_account_info(),
                vault.to_account_info(),
                ctx.accounts.system_program.to_account_info()
            ]
        );

        vault.approved_amount = vault.approved_amount.checked_add(trade_fee_estimate).ok_or(VaultError::ArithmeticOverflow)?;

        emit!({
            FundDeposited {
               vault: vault.key(),
               amount: trade_fee_estimate,
               total_deposited: vault.approved_amount
            }
        });

        Ok(())
    }

    // Execute order
    pub fn execute_trade(ctx: Context<ExecuteTrade>, trade_amount: u64, trading_fee: u64) -> Result<()> {
        let clock = Clock::get()?;
        let delegate = &ctx.accounts.delegation;
        let vault = &mut ctx.accounts.vault;

        require!(trade_amount > 0, VaultError::InvalidAmount);
        require!(vault.is_active, VaultError::VaultInactive);
        require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);
        require!(ctx.accounts.ephemeral_wallet.key() == vault.vault_pda, VaultError::UnauthorizedAccess);
        require!(delegate.revoked_at.is_none(), VaultError::UnauthorizedDelegation);

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
    
    // Revoke access and cleanup vault
    pub fn revoke_access(ctx: Context<RevokeAccess>) -> Result<()> {
        let clock = Clock::get()?;
        let vault = &mut ctx.accounts.vault;
        let delegate = &mut ctx.accounts.delegation;

        require!(clock.unix_timestamp < vault.last_activity, VaultError::SessionExpired);

        vault.is_active = false;
        delegate.revoked_at = Some(clock.unix_timestamp);

        let vault_lamports = vault.to_account_info().lamports();
        let rent = Rent::get()?.minimum_balance(vault.to_account_info().data_len());

        if vault_lamports > rent {
            let refund_amount = vault_lamports.checked_sub(rent)
                .ok_or(VaultError::ArithmeticOverflow)?;
            **vault.to_account_info().try_borrow_mut_lamports()? -= refund_amount;
            **ctx.accounts.user_wallet.to_account_info().try_borrow_mut_lamports()? += refund_amount;
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

    pub fn cleanup_expired_vault(ctx: Context<CleanupVault>) -> Result<()> {
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
            **ctx.accounts.user_wallet.to_account_info().try_borrow_mut_lamports()? += parent_refund;
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
}
