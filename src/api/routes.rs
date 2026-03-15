use crate::api::handlers::{health_handler, websocket_handler};
use crate::api::state::AppState;
use axum::{Router, routing::get};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}
