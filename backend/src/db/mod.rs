use anyhow::Result;
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::NoTls;

pub mod models;
pub mod schema;

pub struct Database {
    pub pool: Pool
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut config = Config::new();
        config.url = Some(database_url.to_string());
        let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)?;

        Ok(Self { pool })
    }

    /*pub fn pool(&self) -> &Pool {
        &self.pool
    }*/

    pub async fn run_migrations(&self) -> Result<()> {
        let client = self.pool.get().await?;

        client.batch_execute(schema::CREATE_SESSIONS_TABLE).await?;
        client.batch_execute(schema::CREATE_DELEGATIONS_TABLE).await?;
        client.batch_execute(schema::CREATE_VAULT_TRANSACTIONS_TABLE).await?;
        client.batch_execute(schema::CREATE_CLEANUP_EVENTS_TABLE).await?;
        client
            .batch_execute(schema::CREATE_SESSION_ANALYTICS_TABLE)
            .await?;
        tracing::info!("Database migration completed!");
        Ok(())
    }
}