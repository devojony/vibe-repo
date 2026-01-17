use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Response model for agent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub name: String,
    pub tool_type: String,
    pub enabled: bool,
    pub command: String,
    pub env_vars: JsonValue,
    pub timeout: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Request model for creating agent
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub workspace_id: i32,
    pub name: String,
    pub tool_type: String,
    pub command: String,
    #[serde(default = "default_env_vars")]
    pub env_vars: JsonValue,
    #[serde(default = "default_timeout")]
    pub timeout: i32,
}

fn default_env_vars() -> JsonValue {
    serde_json::json!({})
}

fn default_timeout() -> i32 {
    1800 // 30 minutes
}

/// Request model for updating agent enabled status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAgentEnabledRequest {
    pub enabled: bool,
}

impl From<crate::entities::agent::Model> for AgentResponse {
    fn from(model: crate::entities::agent::Model) -> Self {
        Self {
            id: model.id,
            workspace_id: model.workspace_id,
            name: model.name,
            tool_type: model.tool_type,
            enabled: model.enabled,
            command: model.command,
            env_vars: model.env_vars,
            timeout: model.timeout,
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}
