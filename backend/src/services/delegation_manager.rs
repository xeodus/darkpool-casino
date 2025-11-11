use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use solana_program::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

pub struct DelegationManager {
    program_id: Pubkey,
}

impl DelegationManager {
    pub fn new(program_id: &str) -> Result<Self> {
        let program_id = Pubkey::from_str(program_id)
            .map_err(|_| anyhow!("Invalid program id: {}", program_id))?;
        Ok(Self { program_id })
    }

    #[inline]
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    pub fn derive_vault_pda(&self, parent_wallet: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", parent_wallet.as_ref()], &self.program_id)
    }

    pub fn derive_delegation_pda(&self, vault: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"delegation", vault.as_ref()], &self.program_id)
    }

    pub fn build_create_vault_ix(
        &self,
        parent_wallet: Pubkey,
        approved_amount: u64,
        session_duration: i64,
    ) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&self.discriminator("create_ephemeral_vault"));
        data.extend_from_slice(&approved_amount.to_le_bytes());
        data.extend_from_slice(&session_duration.to_le_bytes());

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(parent_wallet, true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        })
    }

    pub fn build_approve_delegate_ix(
        &self,
        parent_wallet: Pubkey,
        delegate: Pubkey,
    ) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);
        let (delegation_pda, _) = self.derive_delegation_pda(&vault_pda);

        let mut data = Vec::with_capacity(8 + 32);
        data.extend_from_slice(&self.discriminator("approve_delegate"));
        data.extend_from_slice(delegate.as_ref());

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(parent_wallet, true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(delegation_pda, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        })
    }

    pub fn build_auto_deposit_ix(&self, parent_wallet: Pubkey, amount: u64) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);

        let mut data = Vec::with_capacity(8 + 8);
        data.extend_from_slice(&self.discriminator("auto_deposit_for_trade"));
        data.extend_from_slice(&amount.to_le_bytes());

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(parent_wallet, true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data,
        })
    }

    pub fn build_execute_trade_ix(
        &self,
        parent_wallet: Pubkey,
        delegate: Pubkey,
        trade_amount: u64,
        trading_fee: u64,
    ) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);
        let (delegation_pda, _) = self.derive_delegation_pda(&vault_pda);

        let mut data = Vec::with_capacity(8 + 8 + 8);
        data.extend_from_slice(&self.discriminator("execute_trade"));
        data.extend_from_slice(&trade_amount.to_le_bytes());
        data.extend_from_slice(&trading_fee.to_le_bytes());

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(delegate, true),
                AccountMeta::new_readonly(parent_wallet, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(delegation_pda, false),
            ],
            data,
        })
    }

    pub fn build_revoke_access_ix(&self, parent_wallet: Pubkey) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);
        let (delegation_pda, _) = self.derive_delegation_pda(&vault_pda);

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(parent_wallet, true),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(delegation_pda, false),
            ],
            data: self.discriminator("revoke_access").to_vec(),
        })
    }

    pub fn build_cleanup_vault_ix(
        &self,
        cleaner: Pubkey,
        parent_wallet: Pubkey,
    ) -> Result<Instruction> {
        let (vault_pda, _) = self.derive_vault_pda(&parent_wallet);
        let (delegation_pda, _) = self.derive_delegation_pda(&vault_pda);

        Ok(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(cleaner, true),
                AccountMeta::new(parent_wallet, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(delegation_pda, false),
            ],
            data: self.discriminator("cleanup_vault").to_vec(),
        })
    }

    fn discriminator(&self, name: &str) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(format!("global:{}", name));
        let hash = hasher.finalize();
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&hash[..8]);
        disc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM_ID: &str = "9N97GnZ47zpk8VXBq7wRdKQEUbJT19r7XFQK2XahJYa3";

    #[test]
    fn create_vault_instruction_layout() {
        let manager = DelegationManager::new(PROGRAM_ID).expect("delegation manager");
        let parent_wallet = Pubkey::new_unique();
        let (vault_pda, _) = manager.derive_vault_pda(&parent_wallet);

        let ix = manager
            .build_create_vault_ix(parent_wallet, 1_000_000, 3600)
            .expect("instruction");

        assert_eq!(ix.accounts.len(), 3);
        assert_eq!(ix.accounts[0], AccountMeta::new(parent_wallet, true));
        assert_eq!(ix.accounts[1], AccountMeta::new(vault_pda, false));
        assert_eq!(
            ix.accounts[2],
            AccountMeta::new_readonly(system_program::ID, false)
        );

        let mut hasher = Sha256::new();
        hasher.update(b"global:create_ephemeral_vault");
        assert_eq!(&ix.data[..8], &hasher.finalize()[..8]);
    }

    #[test]
    fn execute_trade_instruction_layout() {
        let manager = DelegationManager::new(PROGRAM_ID).expect("delegation manager");
        let parent_wallet = Pubkey::new_unique();
        let delegate = Pubkey::new_unique();
        let (vault_pda, _) = manager.derive_vault_pda(&parent_wallet);
        let (delegation_pda, _) = manager.derive_delegation_pda(&vault_pda);

        let ix = manager
            .build_execute_trade_ix(parent_wallet, delegate, 500, 50)
            .expect("instruction");

        assert_eq!(ix.accounts.len(), 4);
        assert_eq!(ix.accounts[0], AccountMeta::new(delegate, true));
        assert_eq!(
            ix.accounts[1],
            AccountMeta::new_readonly(parent_wallet, false)
        );
        assert_eq!(ix.accounts[2], AccountMeta::new(vault_pda, false));
        assert_eq!(ix.accounts[3], AccountMeta::new(delegation_pda, false));

        let mut hasher = Sha256::new();
        hasher.update(b"global:execute_trade");
        assert_eq!(&ix.data[..8], &hasher.finalize()[..8]);
    }
}
