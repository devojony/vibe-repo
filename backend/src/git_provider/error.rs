use thiserror::Error;

/// Errors that can occur when interacting with Git providers
#[derive(Debug, Error)]
pub enum GitProviderError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),

    #[error("Client creation error: {0}")]
    ClientCreationError(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Not mergeable: {0}")]
    NotMergeable(String),

    #[error("Branch already exists: {0}")]
    BranchAlreadyExists(String),

    #[error("Label already exists: {0}")]
    LabelAlreadyExists(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl GitProviderError {
    /// Create error from HTTP status code
    pub fn from_status(status: u16, message: String) -> Self {
        match status {
            401 => Self::Unauthorized(message),
            403 => Self::Forbidden(message),
            404 => Self::NotFound(message),
            409 => Self::Conflict(message),
            422 => Self::ValidationError(message),
            429 => Self::RateLimitExceeded(message),
            _ => Self::Internal(format!("HTTP {}: {}", status, message)),
        }
    }
}
