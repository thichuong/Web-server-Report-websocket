use crate::api::state::AppState;
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::info;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: Arc<AppState>) {
    use std::sync::atomic::Ordering;

    state.active_ws_connections.fetch_add(1, Ordering::SeqCst);
    let current_connections = state.active_connections();
    info!(
        "➕ New WebSocket connection (total: {})",
        current_connections
    );

    let mut rx = state.broadcaster.subscribe();

    if socket
        .send(Message::Text(
            "Connected to WebSocket service".to_string().into(),
        ))
        .await
        .is_err()
    {
        info!("Failed to send initial message");
        state.active_ws_connections.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    // Send latest market data immediately upon connection
    if let Some(initial_msg) = state.get_latest_market_data_message().await
        && socket.send(Message::Text(initial_msg.into())).await.is_err()
    {
        info!("Failed to send initial market data payload");
        state.active_ws_connections.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        let text_trimmed = text.trim();
                        if text_trimmed == "ping" {
                            if socket.send(Message::Text("pong".to_string().into())).await.is_err() {
                                break;
                            }
                        } else if (text_trimmed == "request_update"
                            || text_trimmed.contains("request_dashboard_data")
                            || text_trimmed.contains("request_market_data"))
                            && let Some(data_msg) = state.get_latest_market_data_message().await
                            && socket.send(Message::Text(data_msg.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(Message::Ping(bytes)) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }

    state.active_ws_connections.fetch_sub(1, Ordering::SeqCst);
    let current_connections = state.active_connections();
    info!(
        "➖ WebSocket connection closed (total: {})",
        current_connections
    );
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (is_healthy, health_details) = state.health_check_detailed().await;

    let status = if is_healthy { "healthy" } else { "unhealthy" };
    let status_code = if is_healthy {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        axum::Json(serde_json::json!({
            "status": status,
            "service": "web-server-report-websocket",
            "active_connections": state.active_connections(),
            "details": health_details,
        })),
    )
}
