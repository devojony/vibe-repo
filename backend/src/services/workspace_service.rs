use sea_orm::{DatabaseConnection, EntityTrait, Set};
use crate::entities::{workspace, prelude::*};
use crate::error::{GitAutoDevError, Result};

#[derive(Clone)]
pub struct WorkspaceService {
    db: DatabaseConnection,
}

impl WorkspaceService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    pub async fn create_workspace(&self, repository_id: i32) -> Result<workspace::Model> {
        let workspace = workspace::ActiveModel {
            repository_id: Set(repository_id),
            workspace_status: Set("Initializing".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        
        let workspace = Workspace::insert(workspace)
            .exec_with_returning(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(workspace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::entities::prelude::{RepoProvider, Repository};
    use sea_orm::{EntityTrait, Set};

    #[tokio::test]
    async fn test_create_workspace_success() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        
        // Create a test provider first
        let provider = crate::entities::repo_provider::ActiveModel {
            name: Set("Test Provider".to_string()),
            provider_type: Set(crate::entities::repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec(db).await.unwrap();
        
        // Create a test repository
        let repo = crate::entities::repository::ActiveModel {
            name: Set("test-repo".to_string()),
            full_name: Set("owner/test-repo".to_string()),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.last_insert_id),
            ..Default::default()
        };
        let repo = Repository::insert(repo).exec(db).await.unwrap();
        
        let service = WorkspaceService::new(db.clone());
        
        // Act
        let result = service.create_workspace(repo.last_insert_id).await;
        
        // Assert
        assert!(result.is_ok());
        let workspace = result.unwrap();
        assert_eq!(workspace.repository_id, repo.last_insert_id);
        assert_eq!(workspace.workspace_status, "Initializing");
        assert_eq!(workspace.max_concurrent_tasks, 3);
    }
}
