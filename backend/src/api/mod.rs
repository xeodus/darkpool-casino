use std::{sync::Arc, time::Duration};
use anyhow::Context;

use crate::{
    config::Config,
    db::Database,
    services::{
        deposit_calculator::DepositCalculator, delegation_manager::DelegationManager,
        key_manager::KeyManager, session_manager::SessionManager,
        transaction_manager::TransactionSigner, vault_monitor::VaultMonitor,
    },
};

pub mod routers;
pub mod handlers;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub session_manager: Arc<SessionManager>,
    pub deposit_calculator: Arc<DepositCalculator>,
    pub transaction_signer: Arc<TransactionSigner>,
    pub key_manager: Arc<KeyManager>,
    pub delegation_manager: Arc<DelegationManager>,
}

impl AppState {
    pub async fn new(config: Config, database: Database) -> anyhow::Result<Self> {
        let key_manager = Arc::new(
            KeyManager::new(&config.encryption_id)
                .context("failed to initialise key manager")?,
        );

        let session_manager =
            Arc::new(SessionManager::new(database.pool.clone(), key_manager.clone()));
        let deposit_calculator = Arc::new(DepositCalculator::new());
        let transaction_signer = Arc::new(TransactionSigner::new(&config.solana_rpc_url));
        let delegation_manager = Arc::new(
            DelegationManager::new(&config.program_id)
                .context("failed to initialise delegation manager")?,
        );

        let monitor = Arc::new(VaultMonitor::new(
            session_manager.clone(),
            deposit_calculator.clone(),
            Duration::from_secs(30),
        ));
        monitor.spawn();

        Ok(Self {
            config,
            session_manager,
            deposit_calculator,
            transaction_signer,
            key_manager,
            delegation_manager,
        })
    }
}
