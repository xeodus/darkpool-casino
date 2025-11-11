use std::env;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub db_url: String,
    pub solana_rpc_url: String,
    pub program_id: String,
    pub encryption_id: String
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server_addr: env::var("SERVER_ADDR").expect("Server address not set.."),
            db_url: env::var("DATABASE_URL").expect("Database url not set.."),
            solana_rpc_url: env::var("SOLANA_RPC_URL").expect("Solana rpc url not set.."),
            program_id: env::var("PROGRAM_ID").expect("Program ID not set.."),
            encryption_id: env::var("ENCRYPTION_ID").expect("Encryption ID not found..")
        })
    }
}
