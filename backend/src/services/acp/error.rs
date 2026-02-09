//! Error types for ACP client operations

use thiserror::Error;

/// Result type for ACP operations
pub type AcpResult<T> = Result<T, AcpError>;

/// Errors that can occur during ACP client operations
#[derive(Debug, Error)]
pub enum AcpError {
    /// Protocol error from the ACP SDK
    #[error("ACP protocol error: {0}")]
    Protocol(#[from] agent_client_protocol::Error),

    /// I/O error during communication
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Timeout waiting for response
    #[error("Operation timed out after {0} seconds")]
    Timeout(u64),

    /// Agent subprocess crashed
    #[error("Agent subprocess crashed with exit code: {0:?}")]
    ProcessCrashed(Option<i32>),

    /// Invalid response from agent
    #[error("Invalid response from agent: {0}")]
    InvalidResponse(String),

    /// Agent not initialized
    #[error("Agent not initialized - call initialize() first")]
    NotInitialized,

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for AcpError {
    fn from(s: String) -> Self {
        AcpError::Other(s)
    }
}

impl From<&str> for AcpError {
    fn from(s: &str) -> Self {
        AcpError::Other(s.to_string())
    }
}
