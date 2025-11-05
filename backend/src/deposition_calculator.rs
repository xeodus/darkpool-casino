use std::sync::Arc;
use anyhow::Result;
use chrono::Utc;
use postgres::Client;
use solana_sdk::signature::Signature;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub struct AutoDepositeCalculator {
    pub db: PgPool,
    pub solana_client: Arc<Client>
}

pub struct EstimatedDeposite {
    pub recommended_amount: u64,
    pub estimated_trade: u32,
    pub trade_fee: u64,
    pub buffer_percent: f64
}

impl AutoDepositeCalculator {
    pub fn new(db: PgPool, solana_client: Arc<Client>) -> Self {
        Self {
            db,
            solana_client
        }
    }

    pub fn estimated_trading_fee(&self, num_trades: u32) -> Result<u64> {
        const BASE_FEE_PER_TRADE: u64 = 5_000;
        const COMPUTE_FEE: u64 = 2_000;
        const SIGNATURE_PER_TRADES: u64 = 2;
        const PRIORITY_FEE_BUFFER: u64 = 3_000;

        let fee_per_trade = (BASE_FEE_PER_TRADE * SIGNATURE_PER_TRADES) + COMPUTE_FEE + PRIORITY_FEE_BUFFER;
        let total_trade_fee = fee_per_trade * num_trades as u64;
        Ok(total_trade_fee)
    }

    pub async fn check_optimal_deposite(&self, session_id: Uuid, expected_trades: u32) -> Result<EstimatedDeposite> {
        let base_fee = self.estimated_trading_fee(expected_trades)?;
        let buffer_pct = 0.25;
        let recommended_amount = base_fee * (1 + buffer_pct as u64);

        let session = sqlx::query!(
            r#"
            SELECT total_deposited, total_spent 
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let current_balance = (session.total_deposited - session.total_spent) as u64;

        let final_amount = if current_balance < recommended_amount {
            recommended_amount - current_balance
        }
        else {
            0
        };

        Ok(EstimatedDeposite {
            recommended_amount: final_amount,
            estimated_trade: expected_trades,
            trade_fee: base_fee / expected_trades as u64,
            buffer_percent: buffer_pct
        })
    }

    pub async fn check_balance_and_topup(&self, session_id: Uuid, min_balance: u64) -> Result<Option<u64>> {
        let session = sqlx::query!(
            r#"
            SELECT total_deposited, total_spent
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_one(&self.db)
        .await?;

        let current_balance = (session.total_deposited - session.total_spent) as u64;

        if current_balance < min_balance {
            let estimate = self.check_optimal_deposite(session_id, 100).await?;
            return Ok(Some(estimate.recommended_amount));
        }

        Ok(None)
    }

    pub async fn validate_deposite(&self, amount: u64, max_per_session: u64, current_amount: u64) -> Result<()> {
        if amount > max_per_session {
            tracing::error!("Maximum amount exceeded for current session");
        }

        if (amount + current_amount) > max_per_session * 2 {
            tracing::error!("Total deposite exceeded safety limit: {}", max_per_session * 2);
        }

        const MIN_DEPOSITE: u64 = 1_000_000;

        if amount < MIN_DEPOSITE {
            tracing::error!("Amount too small, must be greater than: {}", MIN_DEPOSITE);
        }

        Ok(())
    }

    pub async fn record_deposite(&self, amount: i64, session_id: Uuid, signature: &Signature, deposit_type: &str) -> Result<()> {
        let timestamp = Utc::now().timestamp_millis();
        let sign = signature.to_string();

        sqlx::query!(
            r#"
            INSERT INTO vault_deposits (session_id, amount, signature, timestamp, deposit_type)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            session_id,
            amount,
            &sign,
            timestamp,
            deposit_type
        )
        .execute(&self.db)
        .await?;

        sqlx::query!(
            r#"
            UPDATE sessions
            SET total_deposited = total_deposited + $1
            WHERE id = $2
            "#,
            amount,
            session_id
        )
        .execute(&self.db)
        .await?;

        info!("Recorded deposite of {} lamports for session: {}", amount, session_id);

        Ok(())
    }


}
