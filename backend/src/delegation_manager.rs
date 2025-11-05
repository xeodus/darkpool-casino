use std::{str::FromStr, sync::Arc};
use anyhow::Result;
use postgres::Client;
use solana_sdk::pubkey::Pubkey;
use sqlx::PgPool;
use anchor_lang::prelude::*;
use uuid::Uuid;

pub struct DelegationManager {
    pub db: PgPool,
    pub client: Arc<Client>,
    pub program_id: Pubkey
}

impl DelegationManager {
    pub fn new(db: PgPool, client: Arc<Client>, program_id: Pubkey) -> Self {
        Self {
            db,
            client,
            program_id
        }
    }

    pub async fn build_delegation_transactions(&self, session_id: Uuid) -> Result<Vec<u8>> {
        let session = sqlx::query!(
            r#"
            SELECT user_pubkey, vault_pubkey
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let user_key = Pubkey::from_str(&session.user_pubkey).expect("Failed to extract user pubkey!");
        let vault_key = Pubkey::from_str(&session.vault_pubkey).expect("Failed to extract vault pubkey!");
        let (delegation_pda, _) = Pubkey::find_program_address(
            &[b"delegation", vault_key.as_ref()], &self.program_id
        );

        let instruction_data = self.encode_approve_delegate_instruction(
            vault_key.clone(), 
            user_key.clone(),
            delegation_pda.clone()
        ).await?;

        let accounts = vec![
        ];
        let ix = solana_sdk::instruction::Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data
        };

        let recent_blockhash = self.client.get_latest_blockhash();


        Ok(())
    }

    pub async fn encode_approve_delegate_instruction(&self, vault_key: Pubkey, user_key: Pubkey, delegate_key: Pubkey) -> Result<Vec<u8>> {
        let session_id = Uuid::new_v4();
        let (_, bump) = Pubkey::find_program_address(&[b"delegation", vault_key.as_ref()], &self.program_id);

        let session = sqlx::query!(
            r#"
            SELECT created_at, expires_at
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let vault_delegation = VaultDelegation {
            vault: vault_key.clone(),
            delegate: delegate_key.clone(),
            approved_at: session.created_at,
            revoked_at: Some(session.expires_at),
            bump
        };

        let instruction = ApproveDelegates {
            user_wallet: user_key,
            vault: vault_key,
            delegation: vault_delegation,
            system_program: system_program::ID
        };

        Ok(())
    }
}