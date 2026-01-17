//! Provider API handlers
//!
//! HTTP request handlers for RepoProvider CRUD operations.

use crate::{
    entities::{
        prelude::{RepoProvider, Repository},
        repo_provider::{ActiveModel, Entity as RepoProviderEntity, ProviderType},
    },
    error::{VibeRepoError, Result},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

use super::models::{
    CreateProviderRequest, ProviderResponse, UpdateProviderRequest, ValidationResponse,
};
use super::validation;

/// List all providers
///
/// Returns all RepoProvider records with masked tokens.
#[utoipa::path(
    get,
    path = "/api/settings/providers",
    responses(
        (status = 200, description = "List of providers", body = Vec<ProviderResponse>)
    ),
    tag = "providers"
)]
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProviderResponse>>> {
    let providers = RepoProvider::find().all(&state.db).await?;

    let responses = providers
        .into_iter()
        .map(ProviderResponse::from_model)
        .collect();

    Ok(Json(responses))
}

/// Create a new provider
///
/// Creates a new RepoProvider with the given configuration.
#[utoipa::path(
    post,
    path = "/api/settings/providers",
    request_body = CreateProviderRequest,
    responses(
        (status = 201, description = "Provider created", body = ProviderResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Provider with same name, base_url, and access_token already exists")
    ),
    tag = "providers"
)]
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<ProviderResponse>)> {
    // Validate input
    if req.name.trim().is_empty() {
        return Err(VibeRepoError::Validation("Name cannot be empty".into()));
    }
    if req.access_token.trim().is_empty() {
        return Err(VibeRepoError::Validation(
            "Access token cannot be empty".into(),
        ));
    }
    if req.provider_type != ProviderType::Gitea {
        return Err(VibeRepoError::Validation(
            "Only 'gitea' provider type is supported in v0.1.0".into(),
        ));
    }
    if req.base_url.trim().is_empty() {
        return Err(VibeRepoError::Validation(
            "Base URL is required for Gitea providers".into(),
        ));
    }

    // Create provider
    let provider = ActiveModel {
        name: Set(req.name),
        provider_type: Set(req.provider_type),
        base_url: Set(req.base_url),
        access_token: Set(req.access_token),
        locked: Set(false),
        ..Default::default()
    };

    let provider = match provider.insert(&state.db).await {
        Ok(p) => p,
        Err(sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(
            db_err,
        )))) => {
            // Check if it's a unique constraint violation
            if db_err.message().contains("UNIQUE constraint failed")
                || db_err.message().contains("duplicate key")
            {
                return Err(VibeRepoError::Conflict(
                    "A provider with the same name, base_url, and access_token already exists"
                        .into(),
                ));
            }
            return Err(VibeRepoError::Database(sea_orm::DbErr::Exec(
                sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(db_err)),
            )));
        }
        Err(e) => return Err(e.into()),
    };

    // Spawn background task to process the new provider
    let service = state.repository_service.clone();
    let provider_id = provider.id;
    tokio::spawn(async move {
        if let Err(e) = service.process_provider(provider_id).await {
            tracing::error!("Failed to process provider {}: {}", provider_id, e);
        }
    });

    Ok((
        StatusCode::CREATED,
        Json(ProviderResponse::from_model(provider)),
    ))
}

/// Get provider by ID
///
/// Returns a specific RepoProvider record with masked token.
#[utoipa::path(
    get,
    path = "/api/settings/providers/{id}",
    params(
        ("id" = i32, Path, description = "Provider ID")
    ),
    responses(
        (status = 200, description = "Provider details", body = ProviderResponse),
        (status = 404, description = "Provider not found")
    ),
    tag = "providers"
)]
pub async fn get_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<ProviderResponse>> {
    let provider = RepoProvider::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Provider {} not found", id)))?;

    Ok(Json(ProviderResponse::from_model(provider)))
}

/// Update provider
///
/// Updates an existing RepoProvider with partial data.
#[utoipa::path(
    put,
    path = "/api/settings/providers/{id}",
    params(
        ("id" = i32, Path, description = "Provider ID")
    ),
    request_body = UpdateProviderRequest,
    responses(
        (status = 200, description = "Provider updated", body = ProviderResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Provider not found"),
        (status = 409, description = "Update would create duplicate provider (same name, base_url, and access_token)")
    ),
    tag = "providers"
)]
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ProviderResponse>> {
    // Fetch existing provider
    let provider = RepoProvider::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Provider {} not found", id)))?;

    // Validate provider type if provided
    if let Some(ref provider_type) = req.provider_type {
        if *provider_type != ProviderType::Gitea {
            return Err(VibeRepoError::Validation(
                "Only 'gitea' provider type is supported in v0.1.0".into(),
            ));
        }
    }

    // Track if token or base_url changed (for re-sync notification)
    let token_changed = req.access_token.is_some();
    let base_url_changed = req.base_url.is_some();

    // Build update
    let mut active_model: ActiveModel = provider.into();

    if let Some(name) = req.name {
        active_model.name = Set(name);
    }
    if let Some(provider_type) = req.provider_type {
        active_model.provider_type = Set(provider_type);
    }
    if let Some(base_url) = req.base_url {
        active_model.base_url = Set(base_url);
    }
    if let Some(access_token) = req.access_token {
        active_model.access_token = Set(access_token);
    }
    if let Some(locked) = req.locked {
        active_model.locked = Set(locked);
    }

    let provider = match active_model.update(&state.db).await {
        Ok(p) => p,
        Err(sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(
            db_err,
        )))) => {
            // Check if it's a unique constraint violation
            if db_err.message().contains("UNIQUE constraint failed")
                || db_err.message().contains("duplicate key")
            {
                return Err(VibeRepoError::Conflict(
                    "A provider with the same name, base_url, and access_token already exists"
                        .into(),
                ));
            }
            return Err(VibeRepoError::Database(sea_orm::DbErr::Exec(
                sea_orm::RuntimeErr::SqlxError(sqlx::Error::Database(db_err)),
            )));
        }
        Err(e) => return Err(e.into()),
    };

    // Spawn background task to re-sync if token or base_url changed
    if token_changed || base_url_changed {
        let service = state.repository_service.clone();
        let provider_id = provider.id;
        tokio::spawn(async move {
            if let Err(e) = service.process_provider(provider_id).await {
                tracing::error!("Failed to process provider {}: {}", provider_id, e);
            }
        });
    }

    Ok(Json(ProviderResponse::from_model(provider)))
}

/// Delete provider
///
/// Deletes a RepoProvider if it's not locked. Cascades to repositories.
#[utoipa::path(
    delete,
    path = "/api/settings/providers/{id}",
    params(
        ("id" = i32, Path, description = "Provider ID")
    ),
    responses(
        (status = 204, description = "Provider deleted"),
        (status = 404, description = "Provider not found"),
        (status = 409, description = "Provider is locked")
    ),
    tag = "providers"
)]
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<StatusCode> {
    // Fetch provider
    let provider = RepoProvider::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Provider {} not found", id)))?;

    // Check if locked
    if provider.locked {
        return Err(VibeRepoError::Conflict(
            "Cannot delete locked provider".into(),
        ));
    }

    // Get all repositories for this provider
    let repos = Repository::find()
        .filter(crate::entities::repository::Column::ProviderId.eq(id))
        .all(&state.db)
        .await?;

    // Delete webhooks for each repository
    for repo in repos {
        if let Err(e) = state.repository_service.delete_repository(repo.id).await {
            tracing::error!(
                repository_id = repo.id,
                error = %e,
                "Failed to delete repository during provider cleanup"
            );
            // Continue with other repositories
        }
    }

    // Now delete provider (cascade will clean up any remaining DB records)
    RepoProviderEntity::delete_by_id(id).exec(&state.db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Validate provider token
///
/// Tests the provider token against the Git provider API.
#[utoipa::path(
    post,
    path = "/api/settings/providers/{id}/validate",
    params(
        ("id" = i32, Path, description = "Provider ID")
    ),
    responses(
        (status = 200, description = "Validation result", body = ValidationResponse),
        (status = 404, description = "Provider not found"),
        (status = 503, description = "Provider API unreachable")
    ),
    tag = "providers"
)]
pub async fn validate_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<ValidationResponse>> {
    // Fetch provider
    let provider = RepoProvider::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Provider {} not found", id)))?;

    // Validate token
    let provider_type_str = match provider.provider_type {
        ProviderType::Gitea => "gitea",
    };

    match validation::validate_token(
        &provider.base_url,
        &provider.access_token,
        provider_type_str,
    )
    .await
    {
        Ok((valid, message, user)) => {
            let user_info = user.map(|u| super::models::UserInfo {
                username: u.username,
                id: u.id.parse::<i64>().unwrap_or(0),
                email: u.email,
            });

            Ok(Json(ValidationResponse {
                valid,
                message,
                user_info,
            }))
        }
        Err(e) => Err(e),
    }
}

/// Sync provider repositories
///
/// Triggers repository synchronization for the provider.
#[utoipa::path(
    post,
    path = "/api/settings/providers/{id}/sync",
    params(
        ("id" = i32, Path, description = "Provider ID")
    ),
    responses(
        (status = 202, description = "Sync triggered"),
        (status = 404, description = "Provider not found")
    ),
    tag = "providers"
)]
pub async fn sync_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<StatusCode> {
    // Verify provider exists
    let provider = RepoProvider::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| VibeRepoError::NotFound(format!("Provider {} not found", id)))?;

    // Spawn background task to sync the provider
    let service = state.repository_service.clone();
    let provider_id = provider.id;
    tokio::spawn(async move {
        if let Err(e) = service.process_provider(provider_id).await {
            tracing::error!("Failed to sync provider {}: {}", provider_id, e);
        }
    });

    Ok(StatusCode::ACCEPTED)
}
