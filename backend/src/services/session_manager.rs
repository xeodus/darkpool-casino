use std::{convert::TryFrom, sync::Arc};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Pool;
use solana_sdk::signer::Signer;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

use super::key_manager::KeyManager;

pub struct SessionManager {
    pub pool: Pool,
    key_manager: Arc<KeyManager>,
}

impl SessionManager {
    pub fn new(pool: Pool, key_manager: Arc<KeyManager>) -> Self {
        Self { pool, key_manager }
    }

    pub async fn create_session(
        &self,
        parent_wallet: &str,
        vault_address: &str,
        approved_amount: u64,
        session_duration: i64,
    ) -> Result<CreateSessionResult> {
        if session_duration <= 0 {
            return Err(anyhow!("session duration must be positive"));
        }

        let approved_amount_i64 = i64::try_from(approved_amount)
            .map_err(|_| anyhow!("approved amount exceeds i64 range"))?;

        let keypair = self.key_manager.generate_keypair()?;
        let encrypted_keypair = self.key_manager.encrypt_keypair(&keypair).await?;
        let ephemeral_wallet = keypair.pubkey().to_string();
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(session_duration);

        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO sessions (
                    session_id,
                    parent_wallet,
                    ephemeral_wallet,
                    encrypted_keypair,
                    vault_address,
                    approved_amount,
                    session_duration,
                    expires_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &session_id as &dyn ToSql,
                    &parent_wallet,
                    &ephemeral_wallet,
                    &encrypted_keypair,
                    &vault_address,
                    &approved_amount_i64,
                    &session_duration,
                    &expires_at,
                ],
            )
            .await?;

        Ok(CreateSessionResult {
            session_id,
            parent_wallet: parent_wallet.to_string(),
            ephemeral_wallet,
            vault_address: vault_address.to_string(),
            approved_amount,
            expires_at,
        })
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfo>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT
                    session_id,
                    parent_wallet,
                    ephemeral_wallet,
                    vault_address,
                    approved_amount,
                    total_deposited,
                    total_spent,
                    created_at,
                    expires_at,
                    is_active,
                    last_activity
                FROM sessions
                WHERE session_id = $1",
                &[&session_id],
            )
            .await?;

        if let Some(row) = rows.get(0) {
            Ok(Some(SessionInfo {
                session_id: row.get(0),
                parent_wallet: row.get(1),
                ephemeral_wallet: row.get(2),
                vault_address: row.get(3),
                approved_amount: row.get::<_, i64>(4) as u64,
                total_deposited: row.get::<_, i64>(5) as u64,
                total_spent: row.get::<_, i64>(6) as u64,
                created_at: row.get(7),
                expires_at: row.get(8),
                is_active: row.get(9),
                last_activity: row.get(10),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn deactivate_session(&self, session_id: &str) -> Result<()> {
        let client = self.pool.get().await?;
        client
            .execute(
                "UPDATE sessions
                 SET is_active = false, expires_at = LEAST(expires_at, NOW()), last_activity = NOW()
                 WHERE session_id = $1",
                &[&session_id],
            )
            .await?;
        Ok(())
    }

    pub async fn record_deposit(&self, session_id: &str, amount: u64) -> Result<()> {
        let client = self.pool.get().await?;
        let amount_i64 = i64::try_from(amount).map_err(|_| anyhow!("amount exceeds i64 range"))?;
        client
            .execute(
                "UPDATE sessions
                 SET total_deposited = total_deposited + $2,
                     last_activity = NOW()
                 WHERE session_id = $1",
                &[&session_id, &amount_i64],
            )
            .await?;
        Ok(())
    }

    pub async fn record_trade(&self, session_id: &str, cost: u64) -> Result<()> {
        let client = self.pool.get().await?;
        let cost_i64 = i64::try_from(cost).map_err(|_| anyhow!("cost exceeds i64 range"))?;
        client
            .execute(
                "UPDATE sessions
                 SET total_spent = total_spent + $2,
                     last_activity = NOW()
                 WHERE session_id = $1",
                &[&session_id, &cost_i64],
            )
            .await?;
        Ok(())
    }

    pub async fn record_delegation(
        &self,
        session_id: &str,
        vault_address: &str,
        ephemeral_wallet: &str,
    ) -> Result<()> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO delegations (session_id, vault_address, ephemeral_wallet)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (session_id)
                 DO UPDATE SET
                    vault_address = EXCLUDED.vault_address,
                    ephemeral_wallet = EXCLUDED.ephemeral_wallet,
                    approved_at = NOW(),
                    revoked_at = NULL,
                    is_active = true",
                &[&session_id, &vault_address, &ephemeral_wallet],
            )
            .await?;
        Ok(())
    }

    pub async fn expire_stale_sessions(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let updated = client
            .execute(
                "UPDATE sessions
                 SET is_active = false
                 WHERE expires_at <= NOW() AND is_active = true",
                &[],
            )
            .await?;
        Ok(updated as i64)
    }

    pub async fn get_active_sessions_count(&self) -> Result<i64> {
        let client = self.pool.get().await?;
        let row = client
            .query_one("SELECT COUNT(*) FROM sessions WHERE is_active = true", &[])
            .await?;
        Ok(row.get(0))
    }

    pub async fn fetch_encrypted_keypair(&self, session_id: &str) -> Result<Option<String>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT encrypted_keypair FROM sessions WHERE session_id = $1",
                &[&session_id],
            )
            .await?;
        Ok(rows.get(0).map(|row| row.get(0)))
    }
}

pub struct CreateSessionResult {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub approved_amount: u64,
    pub expires_at: DateTime<Utc>,
}

pub struct SessionInfo {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub approved_amount: u64,
    pub total_deposited: u64,
    pub total_spent: u64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
    pub last_activity: DateTime<Utc>,
}
