use crate::entities::{prelude::*, init_script};
use crate::error::{VibeRepoError, Result};
use crate::services::DockerService;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set, QueryFilter, ColumnTrait};

#[derive(Clone)]
pub struct InitScriptService {
    db: DatabaseConnection,
    docker: Option<DockerService>,
}

impl InitScriptService {
    pub fn new(db: DatabaseConnection, docker: Option<DockerService>) -> Self {
        Self { db, docker }
    }

    pub async fn create_init_script(
        &self,
        workspace_id: i32,
        script_content: String,
        timeout_seconds: i32,
    ) -> Result<init_script::Model> {
        // Verify workspace exists
        let _workspace = Workspace::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!("Workspace {} not found", workspace_id))
            })?;

        // Create script
        let script = init_script::ActiveModel {
            workspace_id: Set(workspace_id),
            script_content: Set(script_content),
            timeout_seconds: Set(timeout_seconds),
            status: Set("Pending".to_string()),
            ..Default::default()
        };

        let script = InitScript::insert(script)
            .exec_with_returning(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        tracing::info!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Created init script for workspace"
        );

        Ok(script)
    }

    pub async fn get_init_script_by_workspace_id(
        &self,
        workspace_id: i32,
    ) -> Result<Option<init_script::Model>> {
        let script = InitScript::find()
            .filter(init_script::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(VibeRepoError::Database)?;

        Ok(script)
    }

    pub async fn update_init_script(
        &self,
        workspace_id: i32,
        script_content: String,
        timeout_seconds: i32,
    ) -> Result<init_script::Model> {
        let script = self
            .get_init_script_by_workspace_id(workspace_id)
            .await?
            .ok_or_else(|| {
                VibeRepoError::NotFound(format!(
                    "Init script for workspace {} not found",
                    workspace_id
                ))
            })?;

        let mut script: init_script::ActiveModel = script.into();
        script.script_content = Set(script_content);
        script.timeout_seconds = Set(timeout_seconds);
        script.status = Set("Pending".to_string()); // Reset status
        script.updated_at = Set(Utc::now());

        let script = script.update(&self.db).await.map_err(VibeRepoError::Database)?;

        tracing::info!(
            workspace_id = workspace_id,
            script_id = script.id,
            "Updated init script"
        );

        Ok(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestDatabase;
    use crate::entities::prelude::{RepoProvider, Repository};

    #[tokio::test]
    async fn test_create_init_script_success() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;

        // Create test workspace
        let workspace = create_test_workspace(db).await;

        let service = InitScriptService::new(db.clone(), None);

        // Act
        let result = service
            .create_init_script(
                workspace.id,
                "#!/bin/bash\necho 'test'".to_string(),
                300,
            )
            .await;

        // Assert
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.workspace_id, workspace.id);
        assert_eq!(script.status, "Pending");
        assert_eq!(script.timeout_seconds, 300);
    }

    #[tokio::test]
    async fn test_get_init_script_by_workspace_id() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let workspace = create_test_workspace(db).await;
        let service = InitScriptService::new(db.clone(), None);

        // Create script
        let created = service
            .create_init_script(workspace.id, "test".to_string(), 300)
            .await
            .unwrap();

        // Act
        let result = service.get_init_script_by_workspace_id(workspace.id).await;

        // Assert
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.is_some());
        assert_eq!(script.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_update_init_script() {
        let test_db = TestDatabase::new()
            .await
            .expect("Failed to create test database");
        let db = &test_db.connection;
        let workspace = create_test_workspace(db).await;
        let service = InitScriptService::new(db.clone(), None);

        // Create script
        let created = service
            .create_init_script(workspace.id, "original".to_string(), 300)
            .await
            .unwrap();

        // Act: Update the script
        let result = service
            .update_init_script(workspace.id, "updated".to_string(), 600)
            .await;

        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.script_content, "updated");
        assert_eq!(updated.timeout_seconds, 600);
        assert_eq!(updated.status, "Pending"); // Status should be reset
    }

    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        // Create a test provider first
        let provider = crate::entities::repo_provider::ActiveModel {
            name: Set("Test Provider".to_string()),
            provider_type: Set(crate::entities::repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider)
            .exec(db)
            .await
            .expect("Failed to create test provider");

        // Create a test repository
        let repo = crate::entities::repository::ActiveModel {
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        let repo = Repository::insert(repo)
            .exec(db)
            .await
            .expect("Failed to create test repository");

        // Create a test workspace
        let workspace = workspace::ActiveModel {
            repository_id: Set(repo.last_insert_id),
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(5),
            cpu_limit: Set(1.0),
            memory_limit: Set("512m".to_string()),
            disk_limit: Set("10g".to_string()),
            ..Default::default()
        };

        Workspace::insert(workspace)
            .exec_with_returning(db)
            .await
            .expect("Failed to create test workspace")
    }
}
