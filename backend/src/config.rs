use std::env;
use anyhow::Result;
use deadpool_postgres::{Config, Pool, Runtime};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

#[derive(Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub server_addr: String,
    pub db_url: String,
    pub solana_rpc_url: String,
    pub program_id: String,
    pub encryption_id: String
}

impl Configuration {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server_addr: env::var("SERVER_ADDR").expect("Server address not set.."),
            db_url: env::var("DATABASE_URL").expect("Database url not set.."),
            solana_rpc_url: env::var("SOLANA_RPC").expect("Solana rpc url not set.."),
            program_id: env::var("PROGRAM_ID").expect("Program ID not set.."),
            encryption_id: env::var("ENCRYPTION_ID").expect("Encryption ID not found..")
        })
    }
}

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
}
