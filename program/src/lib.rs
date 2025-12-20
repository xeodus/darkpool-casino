pub mod state;
pub mod error;
pub mod instructions;
use anchor_lang::prelude::*;
use instructions::*;

declare_id!("SFoL2roUqW3DwqzrTm4UP15dh2daApS76RrhQkXZHyo");

#[program]
pub mod ephemeral_vault {
    use super::*;

    pub fn create_ephemeral_vault(ctx: Context<CreateEphemeralVault>, approved_amount: u64, session_duration: i64) -> Result<()> {
        instructions::create_vault::vault_handler(ctx, approved_amount, session_duration)
    }

    pub fn auto_deposit(ctx: Context<AutoDeposit>, amount: u64) -> Result<()> {
        instructions::auto_deposit::deposition_handler(ctx, amount)
    }

    pub fn approve_delegate(ctx: Context<ApproveDelegates>, delegate: Pubkey) -> Result<()> {
        instructions::approve_delegate::delegation_handler(ctx, delegate)
    }

    pub fn execute_trade(ctx: Context<ExecuteTrade>, trade_amount: u64, trading_fee: u64) -> Result<()> {
        instructions::execute_trade::execution_handler(ctx, trade_amount, trading_fee)
    }

    pub fn revoke_access(ctx: Context<RevokeAccess>) -> Result<()> {
        instructions::revoke_access::access_handler(ctx)
    }

    pub fn cleanup_vault(ctx: Context<CleanupVault>) -> Result<()> {
        instructions::cleanup_vault::cleanup_handler(ctx)
    }
}
