use std::sync::Arc;
use anyhow::anyhow;

use crate::{config::{Configuration, Database},
    services::{delegation_manager::DelegationManager, 
    deposit_calculator::DepositCalculator, key_manager::KeyManager, 
    session_manager::SessionManager, transaction_manager::TransactionSigner}
};

pub mod routers;
pub mod handlers;

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub config: Configuration,
    pub session_manager: Arc<SessionManager>,
    pub delegation_manager: Arc<DelegationManager>,
    pub deposit_calculator: Arc<DepositCalculator>,
    pub transaction_signer: Arc<TransactionSigner>,
    pub key_manager: Arc<KeyManager>
}

impl AppState {
    pub async fn new(config: Configuration, database: Database) -> Self {
        let key_manager = KeyManager::new(&config.encryption_id)
            .map_err(|e| anyhow!("Failed to get key manager: {}", e)).unwrap();

        let session_manager = Arc::new(SessionManager::new(database.pool.clone(), key_manager));
        let deposit_calculator = Arc::new(DepositCalculator::new());
        let delegation_manager = Arc::new(DelegationManager::new());
        let transaction_signer = Arc::new(TransactionSigner::new(&config.solana_rpc_url));
        let key_manager = Arc::new(KeyManager::new(&config.encryption_id).unwrap());

        Self {
            config,
            session_manager,
            delegation_manager,
            deposit_calculator,
            transaction_signer,
            key_manager
        }
    }
}
