//! Webhook request/response models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generic webhook payload
/// This will be expanded to handle specific webhook types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookPayload {
    /// Raw JSON payload from the webhook
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Webhook response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookResponse {
    /// Success status
    pub success: bool,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
