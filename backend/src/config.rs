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
            server_addr: env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:5000".to_string()),
            db_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://ricky:lucifer%40@localhost:5342/goquant".to_string()),
            solana_rpc_url: env::var("SOLANA_RPC")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            program_id: env::var("PROGRAM_ID")
                .unwrap_or_else(|_| "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS".to_string()),
            encryption_id: env::var("ENCRYPTION_ID")
                .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string())
        })
    }
}
