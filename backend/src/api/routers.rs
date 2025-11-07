use axum::routing::{delete, post};
use axum::{Router, routing::get};
use super::handlers;
use super::AppState;

pub fn create_routers(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::check_health))
        .route("/session/create", post(handlers::create_session))
        .route("/session/approve", post(handlers::approve_delegation))
        .route("/session/revoke", delete(handlers::revoke_access))
        .route("/session/status/{session_id}", get(handlers::get_session_status))
        .route("/session/sign", post(handlers::sign_and_send))
        .route("/session/deposit", post(handlers::trigger_deposit))
        .route("/session/stats", get(handlers::get_stats))
        .with_state(state)
}
