use axum::{routing::get, Router};
use std::sync::Arc;
use crate::api::state::AppState;
use crate::api::handlers::{websocket_handler, health_handler};

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}
