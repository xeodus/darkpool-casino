use anchor_lang::prelude::*;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;
use anyhow::Result;
use solana_sdk::{signature::Keypair, signer::Signer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashMap, str::FromStr, sync::{Arc, RwLock}};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, KeyInit, aead::{Aead, OsRng}};

pub struct Session {
    pub id: Uuid,
    pub user_pubkey: Pubkey,
    pub ephemeral_pubkey: Pubkey,
    pub vault_pubkey: Pubkey,
    pub opened_at: i64,
    pub closed_at: i64,
    pub is_active: bool
}

pub struct SessionManager {
    pub db: PgPool,
    pub encryption_key: [u8; 32],
    pub active_sessions: Arc<RwLock<HashMap<Uuid, EncryptedKeypair>>>
}

pub struct EncryptedKeypair {
    pub encrypted_keypair: Vec<u8>,
    pub nonce: [u8; 12]
}

impl SessionManager {
    pub async fn new(database_url: &str, encryption_key: [u8; 32]) -> Self {
        let db = PgPoolOptions::new().max_connections(5).connect(database_url).await.unwrap();
        Self {
            db,
            encryption_key,
            active_sessions: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub fn generate_ephemeral_keypair(&self) -> Keypair {
        Keypair::new()
    }

    pub fn encrypt_keypair(&self, keypair: &Keypair) -> Result<EncryptedKeypair> {
        let cipher = ChaCha20Poly1305::new(&self.encryption_key.into());
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let encrypt = cipher.encrypt(&nonce, keypair.to_bytes().as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to decrypt the ephemeral keypair: {}", e)).unwrap();

        Ok(EncryptedKeypair {
            encrypted_keypair: encrypt,
            nonce: nonce.into()
        })
    }

    pub fn decrypt_keypair(&self, encrypted: &EncryptedKeypair) -> Result<Keypair> {
        let cipher = ChaCha20Poly1305::new(&self.encryption_key.into());
        let nonce = encrypted.nonce.into();

        let decrypt = cipher.decrypt(&nonce, encrypted.encrypted_keypair.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to decrypt keypair: {}", e))?;

        let decrypted = decrypt.as_slice();

        let keypair = Keypair::try_from(decrypted)
            .map_err(|e| anyhow::anyhow!("Failed to decrypt keypair: {}", e))?;

        Ok(keypair)
    }

    pub async fn create_session(&self, user_pubkey: Pubkey, session_duration: i64) -> Result<(Uuid, Keypair, Pubkey)> {
        let ephemeral_keypair = self.generate_ephemeral_keypair();
        let ephemeral_pubkey = ephemeral_keypair.pubkey();

        let session_id = Uuid::new_v4();
        let program_id = Pubkey::from_str("8ZfWqfDz1oV7E9jX9iZVrMknkLvCcz8mZ2ZbWrd3mBqR").unwrap();
        let (vault_pubkey, _) = Pubkey::find_program_address(&[b"vault", 
            user_pubkey.as_ref(), 
            ephemeral_pubkey.as_ref()], 
            &program_id
        );

        let now = Utc::now().timestamp_millis();
        let expires_at = now + session_duration;
        let encrypted = self.encrypt_keypair(&ephemeral_keypair)?;

        let mut new_active_session = self.active_sessions.write().unwrap();
        new_active_session.insert(session_id.clone(), encrypted);

        sqlx::query!(
            r#"
            INSERT INTO sessions (id, user_pubkey, ephemeral_pubkey, vault_pubkey, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            session_id,
            user_pubkey.to_string(),
            ephemeral_pubkey.to_string(),
            vault_pubkey.to_string(),
            now,
            expires_at
        )
        .execute(&self.db)
        .await?;

        info!("Generated new session with id: {}", session_id);

        Ok((session_id, ephemeral_keypair, vault_pubkey))
    }

    pub async fn get_session(&self, session_id: Uuid) -> Result<Session> {
        let row = sqlx::query!(
            r#"
            SELECT id, user_pubkey, ephemeral_pubkey, vault_pubkey, is_active, created_at, expires_at
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let user_key = Pubkey::from_str(&row.user_pubkey).expect("User pubkey string not found!");
        let ephemeral_key = Pubkey::from_str(&row.ephemeral_pubkey).expect("Ephemeral pubkey string not found!");
        let vault_key = Pubkey::from_str(&row.vault_pubkey).expect("Vault pubkey string not found!");

        Ok(Session {
            id: row.id,
            user_pubkey: user_key,
            ephemeral_pubkey: ephemeral_key,
            vault_pubkey: vault_key,
            is_active: true,
            opened_at: row.created_at,
            closed_at: row.expires_at
        })
    }

    pub async fn expire_session(&self, session_id: Uuid) -> Result<()> {
        sqlx::query!(
            "UPDATE sessions SET is_active = false WHERE id = $1",
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        self.active_sessions.write().unwrap().remove(&session_id);
        info!("Expired session with id: {}", session_id);

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<Vec<Uuid>> {
        let now = Utc::now().timestamp_millis();
        let expires = sqlx::query_scalar!(
            r#"
            SELECT id
            FROM sessions
            WHERE is_active = true AND expires_at < $1
            "#,
            now
        )
        .fetch_all(&self.db)
        .await?;

        for session_id in &expires {
            if let Err(e) = self.expire_session(*session_id).await {
                tracing::error!("Failed to expire session for {}: {}", session_id, e);
            }
        }

        if !expires.is_empty() {
            info!("Cleaned up expired sessions of size: {}", expires.len());
        }

        Ok(expires)
    }
}
