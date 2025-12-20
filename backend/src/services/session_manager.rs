use serde::Serialize;
use uuid::Uuid;
use chrono::Utc;
use deadpool_postgres::Pool;
use solana_sdk::signer::Signer;
use anyhow::Result;
use crate::services::key_manager::KeyManager;

pub struct SessionManager {
    pub pool: Pool,
    pub key_manager: KeyManager
}

impl SessionManager {
    pub fn new(pool: Pool, key_manager: KeyManager) -> Self {
        Self {
            pool,
            key_manager
        }
    }

    pub async fn create_sessions(
        &self, 
        parent_wallet: &str, 
        vault_address: &str, 
        session_duration: i64
    ) -> Result<SessionInfo> 
    {
        let keypair = self.key_manager.generate_keypair()?; 
        let ephemeral_wallet = keypair.pubkey().to_string();
        let encrypted_keypair = self.key_manager.encrypt_keypair(&keypair).await?;
        let session_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().timestamp();
        let expires_at = created_at + session_duration;
        let vault_address = vault_address.to_string();
        let client = self.pool.get().await?;
        let is_active = true;

        client.execute(
            "INSERT INTO sessions (session_id, parent_wallet, ephemeral_wallet, 
            encrypted_keypair, vault_address, created_at, expires_at, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)", 
            &[&session_id, &parent_wallet, &ephemeral_wallet, &encrypted_keypair, &vault_address, &created_at, &expires_at, &is_active]
        )
        .await?;

        Ok(SessionInfo {
            session_id,
            parent_wallet: parent_wallet.to_string(),
            ephemeral_wallet,
            vault_address,
            created_at,
            expires_at,
            is_active: true
        })
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let client = self.pool.get().await?;
        let rows = client.query(
            "SELECT session_id, parent_wallet, ephemeral_wallet, vault_address, created_at, expires_at, is_active
            FROM sessions
            WHERE session_id = $1",
            &[&session_id.to_string()]
        )
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];

        Ok(Some(SessionInfo {
            session_id: row.get(0),
            parent_wallet: row.get(1),
            ephemeral_wallet: row.get(2),
            vault_address: row.get(3),
            created_at: row.get(4),
            expires_at: row.get(5),
            is_active: row.get(6)
        }))
    }

    pub async fn deactive_session(&self, session_id: &str) -> Result<()> {
        let client = self.pool.get().await?;

        client.execute(
            "UPDATE sessions SET is_active = false WHERE session_id = $1",
            &[&session_id.to_string()]
        )
        .await?;

        Ok(())
    }

    pub async fn get_active_sessions_count(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client.query_one("SELECT COUNT(*) FROM sessions WHERE is_active = true",
            &[]
        )
        .await?;

        Ok(row.get(0))
    }
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub is_active: bool
}
