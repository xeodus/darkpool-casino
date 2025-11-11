pub mod state;
pub mod error;
pub mod instructions;
use anchor_lang::prelude::*;

use instructions::{
    create_vault::CreateEphemeralVault,
    auto_deposit::AutoDeposit,
    approve_delegate::ApproveDelegate,
    execute_trade::ExecuteTrade,
    revoke_access::RevokeAccess,
    cleanup_vault::CleanupVault,
};

declare_id!("9N97GnZ47zpk8VXBq7wRdKQEUbJT19r7XFQK2XahJYa3");

pub mod ephemeral_vault {
    use super::*;

    pub fn create_ephemeral_vault(ctx: Context<CreateEphemeralVault>, approved_amount: u64, session_duration: i64) -> Result<()> {
        instructions::create_vault::handler(ctx, approved_amount, session_duration)
    }

    pub fn approve_delegate(ctx: Context<ApproveDelegate>, delegate: Pubkey) -> Result<()> {
        instructions::approve_delegate::handler(ctx, delegate)
    }

    pub fn auto_deposit_for_trade(ctx: Context<AutoDeposit>, amount: u64) -> Result<()> {
        instructions::auto_deposit::handler(ctx, amount)
    }

    pub fn execute_trade(
        ctx: Context<ExecuteTrade>,
        trade_amount: u64,
        trading_fee: u64,
    ) -> Result<()> {
        instructions::execute_trade::handler(ctx, trade_amount, trading_fee)
    }

    pub fn revoke_access(ctx: Context<RevokeAccess>) -> Result<()> {
        instructions::revoke_access::handler(ctx)
    }

    pub fn cleanup_vault(ctx: Context<CleanupVault>) -> Result<()> {
        instructions::cleanup_vault::handler(ctx)
    }
}
