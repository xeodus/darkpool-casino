use std::env;
use anyhow::Result;
use sqlx::postgres::PgPoolOptions;

mod backend;
mod solana_core;

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = env::var("DATABASE_URL").expect("Database url not set!");
    let pool = PgPoolOptions::new().max_connections(5).connect(&database_url).await?;

    println!("Connected to the database!");
    Ok(())
}
