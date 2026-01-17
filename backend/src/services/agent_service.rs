use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait, QueryFilter, ColumnTrait};
use crate::entities::{agent, prelude::*};
use crate::error::{GitAutoDevError, Result};
use serde_json::Value as JsonValue;
use chrono::Utc;

#[derive(Clone)]
pub struct AgentService {
    db: DatabaseConnection,
}

impl AgentService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
    
    pub async fn create_agent(
        &self,
        workspace_id: i32,
        name: &str,
        tool_type: &str,
        command: &str,
        env_vars: JsonValue,
        timeout: i32,
    ) -> Result<agent::Model> {
        let agent = agent::ActiveModel {
            workspace_id: Set(workspace_id),
            name: Set(name.to_string()),
            tool_type: Set(tool_type.to_string()),
            command: Set(command.to_string()),
            env_vars: Set(env_vars),
            timeout: Set(timeout),
            enabled: Set(true),
            ..Default::default()
        };
        
        let agent = Agent::insert(agent)
            .exec_with_returning(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(agent)
    }

    pub async fn get_agent_by_id(&self, id: i32) -> Result<agent::Model> {
        Agent::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?
            .ok_or_else(|| GitAutoDevError::NotFound(format!("Agent with id {} not found", id)))
    }

    pub async fn list_agents_by_workspace(&self, workspace_id: i32) -> Result<Vec<agent::Model>> {
        Agent::find()
            .filter(agent::Column::WorkspaceId.eq(workspace_id))
            .all(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))
    }

    pub async fn update_agent_enabled(&self, id: i32, enabled: bool) -> Result<agent::Model> {
        let agent = self.get_agent_by_id(id).await?;
        
        let mut agent: agent::ActiveModel = agent.into();
        agent.enabled = Set(enabled);
        agent.updated_at = Set(Utc::now());
        
        let agent = agent.update(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(agent)
    }

    pub async fn delete_agent(&self, id: i32) -> Result<()> {
        let agent = self.get_agent_by_id(id).await?;
        
        let agent: agent::ActiveModel = agent.into();
        agent.delete(&self.db)
            .await
            .map_err(|e| GitAutoDevError::Database(e))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::db::TestDatabase;
    use crate::entities::{workspace, repository};
    use sea_orm::{DatabaseConnection, Set};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_agent_success() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        
        // Create workspace
        let workspace = create_test_workspace(db).await;
        let service = AgentService::new(db.clone());
        
        let env_vars = json!({"API_KEY": "test-key"});
        
        // Act
        let result = service.create_agent(
            workspace.id,
            "OpenCode Primary",
            "opencode",
            "opencode --model claude-3.5",
            env_vars,
            1800,
        ).await;
        
        // Assert
        assert!(result.is_ok());
        let agent = result.unwrap();
        assert_eq!(agent.workspace_id, workspace.id);
        assert_eq!(agent.name, "OpenCode Primary");
        assert_eq!(agent.tool_type, "opencode");
        assert_eq!(agent.enabled, true);
    }

    #[tokio::test]
    async fn test_list_agents_by_workspace() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        let service = AgentService::new(db.clone());
        let workspace = create_test_workspace(db).await;
        
        // Create multiple agents
        service.create_agent(workspace.id, "Agent 1", "opencode", "cmd1", json!({}), 1800).await.unwrap();
        service.create_agent(workspace.id, "Agent 2", "aider", "cmd2", json!({}), 1800).await.unwrap();
        
        // Act
        let result = service.list_agents_by_workspace(workspace.id).await;
        
        // Assert
        assert!(result.is_ok());
        let agents = result.unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_update_agent_enabled() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        let service = AgentService::new(db.clone());
        let workspace = create_test_workspace(db).await;
        let agent = service.create_agent(workspace.id, "Test", "opencode", "cmd", json!({}), 1800).await.unwrap();
        
        // Act
        let result = service.update_agent_enabled(agent.id, false).await;
        
        // Assert
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.enabled, false);
    }

    #[tokio::test]
    async fn test_delete_agent() {
        // Arrange
        let test_db = TestDatabase::new().await.expect("Failed to create test database");
        let db = &test_db.connection;
        let service = AgentService::new(db.clone());
        let workspace = create_test_workspace(db).await;
        let agent = service.create_agent(workspace.id, "Test", "opencode", "cmd", json!({}), 1800).await.unwrap();
        
        // Act
        let result = service.delete_agent(agent.id).await;
        
        // Assert
        assert!(result.is_ok());
        
        // Verify deleted
        let get_result = service.get_agent_by_id(agent.id).await;
        assert!(get_result.is_err());
    }
    
    async fn create_test_workspace(db: &DatabaseConnection) -> workspace::Model {
        let repo = create_test_repository(db).await;
        let ws = workspace::ActiveModel {
            repository_id: Set(repo.id),
            workspace_status: Set("Active".to_string()),
            image_source: Set("default".to_string()),
            max_concurrent_tasks: Set(3),
            cpu_limit: Set(2.0),
            memory_limit: Set("4GB".to_string()),
            disk_limit: Set("10GB".to_string()),
            ..Default::default()
        };
        Workspace::insert(ws).exec_with_returning(db).await.unwrap()
    }
    
    async fn create_test_repository(db: &DatabaseConnection) -> repository::Model {
        use crate::entities::repo_provider;
        
        // Create a provider first
        let provider = repo_provider::ActiveModel {
            name: Set("Test Provider".to_string()),
            provider_type: Set(repo_provider::ProviderType::Gitea),
            base_url: Set("https://git.example.com".to_string()),
            access_token: Set("test-token".to_string()),
            locked: Set(false),
            ..Default::default()
        };
        let provider = RepoProvider::insert(provider).exec_with_returning(db).await.unwrap();
        
        let repo = repository::ActiveModel {
            name: Set(format!("test-repo-{}", uuid::Uuid::new_v4())),
            full_name: Set(format!("owner/test-repo-{}", uuid::Uuid::new_v4())),
            clone_url: Set("https://git.example.com/owner/test-repo.git".to_string()),
            default_branch: Set("main".to_string()),
            provider_id: Set(provider.id),
            ..Default::default()
        };
        Repository::insert(repo).exec_with_returning(db).await.unwrap()
    }
}
