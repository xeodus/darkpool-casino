use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::interval;
use tracing::{debug, info, warn};

use super::{deposit_calculator::DepositCalculator, session_manager::SessionManager};

pub struct VaultMonitor {
    session_manager: Arc<SessionManager>,
    deposit_calculator: Arc<DepositCalculator>,
    poll_interval: Duration,
}

impl VaultMonitor {
    pub fn new(
        session_manager: Arc<SessionManager>,
        deposit_calculator: Arc<DepositCalculator>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            session_manager,
            deposit_calculator,
            poll_interval,
        }
    }

    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                warn!("vault monitor terminated: {e:?}");
            }
        });
    }

    async fn run(&self) -> Result<()> {
        let mut ticker = interval(self.poll_interval);

        loop {
            ticker.tick().await;
            if let Err(e) = self.tick().await {
                warn!("vault monitor tick failed: {e:?}");
            }
        }
    }

    async fn tick(&self) -> Result<()> {
        let expired = self.session_manager.expire_stale_sessions().await?;
        if expired > 0 {
            info!("expired {expired} stale sessions");
        } else {
            debug!("no stale sessions detected");
        }

        // Placeholder for future low-balance / anomaly detection hooks
        let _target_buffer = self.deposit_calculator.calculate_default_deposit();

        Ok(())
    }
}
