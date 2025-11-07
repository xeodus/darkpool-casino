use std::sync::Arc;
use anyhow::anyhow;

use crate::{config::Config, db::Database, 
    services::{deposit_calculator::DepositCalculator, 
    key_manager::KeyManager, session_manager::SessionManager, 
    transaction_manager::TransactionSigner}
};

pub mod routers;
pub mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub session_manager: Arc<SessionManager>,
    pub deposit_calculator: Arc<DepositCalculator>,
    pub transaction_signer: Arc<TransactionSigner>,
    pub key_manager: Arc<KeyManager>
}

impl AppState {
    pub async fn new(config: Config, database: Database) -> Self {
        let key_manager = KeyManager::new(&config.encryption_id)
            .map_err(|e| anyhow!("Failed to get key manager: {}", e)).unwrap();

        let session_manager = Arc::new(SessionManager::new(database.pool.clone(), key_manager));
        let deposit_calculator = Arc::new(DepositCalculator::new());
        let transaction_signer = Arc::new(TransactionSigner::new(&config.solana_rpc_url));
        let key_manager = Arc::new(KeyManager::new(&config.encryption_id).unwrap());

        Self {
            config,
            session_manager,
            deposit_calculator,
            transaction_signer,
            key_manager
        }
    }
}
