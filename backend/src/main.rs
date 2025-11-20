use std::error::Error;
use axum::{Router, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use crate::{api::{AppState, routers::create_routers}, config::{Configuration, Database}};

mod api;
mod services;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_env_filter("darkpool_backend=debug,tower_http=debug").init();
    dotenv::dotenv().ok();
    let config = Configuration::from_env()?;

    info!("Starting ephemeral vault backend on {}", config.server_addr);

    let database = Database::new(&config.db_url).await?;

    let app_state = AppState::new(config.clone(), database);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);

    let nest = Router::new()
        .route("/ping", axum::routing::get(|| async { "pong" }))
        .nest("/api", create_routers(app_state.await))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.server_addr).await?;

    info!("Server listening on {}", config.server_addr);
    axum::serve(listener, nest).await?;

    Ok(())
}
