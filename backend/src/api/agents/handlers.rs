use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::{api::agents::models::*, error::Result, services::AgentService, state::AppState};

/// Create a new agent
#[utoipa::path(
    post,
    path = "/api/agents",
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent created successfully", body = AgentResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>)> {
    let service = AgentService::new(state.db.clone());

    let agent = service
        .create_agent(
            req.workspace_id,
            &req.name,
            &req.tool_type,
            &req.command,
            req.env_vars,
            req.timeout,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(agent.into())))
}

/// Get agent by ID
#[utoipa::path(
    get,
    path = "/api/agents/{id}",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent found", body = AgentResponse),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<AgentResponse>> {
    let service = AgentService::new(state.db.clone());

    let agent = service.get_agent_by_id(id).await?;

    Ok(Json(agent.into()))
}

/// List agents by workspace
#[utoipa::path(
    get,
    path = "/api/workspaces/{workspace_id}/agents",
    params(
        ("workspace_id" = i32, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "List of agents", body = Vec<AgentResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn list_agents_by_workspace(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<i32>,
) -> Result<Json<Vec<AgentResponse>>> {
    let service = AgentService::new(state.db.clone());

    let agents = service.list_agents_by_workspace(workspace_id).await?;

    let responses: Vec<AgentResponse> = agents.into_iter().map(|a| a.into()).collect();

    Ok(Json(responses))
}

/// Update agent enabled status
#[utoipa::path(
    patch,
    path = "/api/agents/{id}/enabled",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    request_body = UpdateAgentEnabledRequest,
    responses(
        (status = 200, description = "Agent updated", body = AgentResponse),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn update_agent_enabled(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateAgentEnabledRequest>,
) -> Result<Json<AgentResponse>> {
    let service = AgentService::new(state.db.clone());

    let agent = service.update_agent_enabled(id, req.enabled).await?;

    Ok(Json(agent.into()))
}

/// Delete agent
#[utoipa::path(
    delete,
    path = "/api/agents/{id}",
    params(
        ("id" = i32, Path, description = "Agent ID")
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 404, description = "Agent not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "agents"
)]
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<StatusCode> {
    let service = AgentService::new(state.db.clone());

    service.delete_agent(id).await?;

    Ok(StatusCode::NO_CONTENT)
}
