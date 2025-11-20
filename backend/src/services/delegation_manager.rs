use std::str::FromStr;
use sha2::{Digest, Sha256};
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{message::{AccountMeta, Instruction}, pubkey::Pubkey};
use anyhow::{anyhow, Result};
use std::env;

pub struct DelegationManager {
    pub parent_wallet: Pubkey
}

impl DelegationManager {
    pub fn new() -> Self {
        Self {
            parent_wallet: Pubkey::new_unique()
        }
    }

    #[inline]
    fn program_id() -> Pubkey {
        let program_id = Pubkey::from_str(&env::var("PROGRAM_ID")
            .expect("Program ID is not set..")).unwrap();

        program_id
    }

    // Derive the vault PDA your program expects: seeds = [b"vault", parent_wallet]
    pub fn derive_vault_pda(&self) -> (Pubkey, u8) {
        let vault_address = Pubkey::find_program_address(&[b"vault", self.parent_wallet.as_ref()], &Self::program_id()) ;
        tracing::debug!("Vault address derived: {:?}", &vault_address);
        vault_address
    }

    // Build the auto_deposit instruction
    pub async fn build_deposit_ix(
        &self,  
        vault_address: Pubkey,
        trade_fee_estimate: u64,
    ) -> Result<Instruction> 
    {
        let (expected, _) = self.derive_vault_pda(); 
        if expected != vault_address {
            return Err(anyhow!("vault PDA mismatch: expected {}, got {}", expected, vault_address));
        }

        let accounts = vec![
            AccountMeta::new(self.parent_wallet, true),
            AccountMeta::new(vault_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ];
        
        let mut hasher = Sha256::new(); 
        hasher.update(b"global:auto_deposit");
        let disc = &hasher.finalize()[..8];

        let mut data = Vec::with_capacity(8 + 8);
        data.extend_from_slice(disc);
        data.extend_from_slice(&trade_fee_estimate.to_le_bytes()); // single u64 arg

        Ok(Instruction {
            program_id: Self::program_id(),
            accounts,
            data,
        })
    }
}
