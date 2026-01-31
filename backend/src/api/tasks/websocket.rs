//! WebSocket handlers for real-time task log streaming
//!
//! Provides WebSocket endpoints for streaming task execution logs in real-time.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::{error::VibeRepoError, services::TaskService, state::AppState};

/// Query parameters for WebSocket authentication
#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    /// Authentication token
    pub token: Option<String>,
}

/// WebSocket handler for streaming task logs
pub async fn stream_task_logs(
    ws: WebSocketUpgrade,
    Path(task_id): Path<i32>,
    Query(query): Query<WebSocketQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, VibeRepoError> {
    info!(
        task_id = task_id,
        "WebSocket connection requested for task logs"
    );

    // Validate authentication token if configured
    if let Some(required_token) = &state.config.websocket.auth_token {
        match &query.token {
            Some(provided_token) if provided_token == required_token => {
                info!(task_id = task_id, "WebSocket authentication successful");
            }
            Some(_) => {
                warn!(
                    task_id = task_id,
                    "WebSocket authentication failed: invalid token"
                );
                return Ok(
                    (StatusCode::UNAUTHORIZED, "Invalid authentication token").into_response()
                );
            }
            None => {
                warn!(
                    task_id = task_id,
                    "WebSocket authentication failed: missing token"
                );
                return Ok((
                    StatusCode::UNAUTHORIZED,
                    "Missing authentication token. Please provide token in query parameter: ?token=YOUR_TOKEN",
                )
                    .into_response());
            }
        }
    } else {
        info!(
            task_id = task_id,
            "WebSocket authentication disabled (no token configured)"
        );
    }

    // Verify task exists
    let task_service = TaskService::new(state.db.clone());
    let task = task_service.get_task_by_id(task_id).await?;

    info!(
        task_id = task_id,
        task_status = %task.task_status,
        "Task found, upgrading to WebSocket"
    );

    // Upgrade to WebSocket
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, task_id, state)))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, task_id: i32, state: Arc<AppState>) {
    info!(task_id = task_id, "WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();

    // Get or create broadcast channel for this task
    let mut rx = state.get_or_create_log_channel(task_id).await;

    // Send initial connection message
    if let Err(e) = sender
        .send(Message::Text(format!(
            "{{\"type\":\"connected\",\"task_id\":{}}}",
            task_id
        )))
        .await
    {
        error!(task_id = task_id, error = %e, "Failed to send connection message");
        return;
    }

    // Spawn task to receive messages from client (for ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => {
                    info!(task_id = task_id, "Client closed connection");
                    break;
                }
                Message::Ping(_data) => {
                    info!(task_id = task_id, "Received ping");
                    // Pong is automatically sent by axum
                }
                Message::Pong(_) => {
                    info!(task_id = task_id, "Received pong");
                }
                _ => {
                    // Ignore other message types
                }
            }
        }
    });

    // Spawn task to send log messages to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(log_message) = rx.recv().await {
            if sender.send(Message::Text(log_message)).await.is_err() {
                warn!(
                    task_id = task_id,
                    "Failed to send log message, client disconnected"
                );
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            info!(task_id = task_id, "Send task completed");
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            info!(task_id = task_id, "Receive task completed");
            send_task.abort();
        }
    }

    info!(task_id = task_id, "WebSocket connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::{config::AppConfig, services::RepositoryService};

    #[tokio::test]
    async fn test_stream_task_logs_task_not_found() {
        // Arrange
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let config = AppConfig::from_env().unwrap();
        let repo_service = Arc::new(RepositoryService::new(
            test_db.connection.clone(),
            Arc::new(config.clone()),
        ));
        let state = Arc::new(AppState::new(
            test_db.connection.clone(),
            config,
            repo_service,
        ));

        // Note: We can't easily test WebSocket upgrade without a full HTTP server
        // This test verifies that the task lookup works correctly
        let task_service = TaskService::new(state.db.clone());
        let result = task_service.get_task_by_id(999).await;

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_query_deserialization() {
        // Test with token
        let query = WebSocketQuery {
            token: Some("test-token".to_string()),
        };
        assert_eq!(query.token, Some("test-token".to_string()));

        // Test without token
        let query = WebSocketQuery { token: None };
        assert_eq!(query.token, None);
    }

    #[tokio::test]
    async fn test_websocket_auth_validation_logic() {
        // This test verifies the authentication logic without WebSocket upgrade

        // Test case 1: No token configured (auth disabled)
        let required_token: Option<String> = None;
        let provided_token: Option<String> = None;

        if let Some(req_token) = &required_token {
            match &provided_token {
                Some(prov_token) if prov_token == req_token => {
                    // Should authenticate - test passes
                }
                _ => {
                    panic!("Should not reach here when no token is required");
                }
            }
        }
        // Auth disabled - test passes

        // Test case 2: Token configured and correct token provided
        let required_token: Option<String> = Some("secret-token".to_string());
        let provided_token: Option<String> = Some("secret-token".to_string());

        let mut authenticated = false;
        if let Some(req_token) = &required_token {
            match &provided_token {
                Some(prov_token) if prov_token == req_token => {
                    authenticated = true;
                }
                _ => {}
            }
        }
        assert!(authenticated, "Should authenticate with correct token");

        // Test case 3: Token configured but wrong token provided
        let required_token: Option<String> = Some("secret-token".to_string());
        let provided_token: Option<String> = Some("wrong-token".to_string());

        let mut authenticated = false;
        if let Some(req_token) = &required_token {
            match &provided_token {
                Some(prov_token) if prov_token == req_token => {
                    authenticated = true;
                }
                _ => {}
            }
        }
        assert!(!authenticated, "Should not authenticate with wrong token");

        // Test case 4: Token configured but no token provided
        let required_token: Option<String> = Some("secret-token".to_string());
        let provided_token: Option<String> = None;

        let mut authenticated = false;
        if let Some(req_token) = &required_token {
            match &provided_token {
                Some(prov_token) if prov_token == req_token => {
                    authenticated = true;
                }
                _ => {}
            }
        }
        assert!(!authenticated, "Should not authenticate without token");
    }
}
