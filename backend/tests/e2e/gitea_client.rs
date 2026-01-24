//! Gitea API client for E2E testing
//!
//! This module provides a client for interacting with Gitea API during end-to-end tests.
//! It supports repository management, issue creation, pull request operations, and branch management.

use serde::{Deserialize, Serialize};

/// Gitea API client for test operations
#[derive(Clone)]
pub struct GiteaClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

/// Gitea repository response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: String,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub owner: GiteaUser,
    pub default_branch: String,
}

/// Gitea user model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaUser {
    pub id: i64,
    pub login: String,
    pub full_name: String,
}

/// Gitea issue response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaIssue {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub labels: Vec<GiteaLabel>,
    pub html_url: String,
}

/// Gitea label model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaLabel {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// Gitea pull request response model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaPullRequest {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: String,
    pub state: String,
    pub html_url: String,
    pub head: GiteaBranch,
    pub base: GiteaBranch,
    pub merged: bool,
}

/// Gitea branch info model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaBranch {
    pub label: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// Request body for creating a repository
#[derive(Debug, Serialize)]
struct CreateRepoRequest {
    name: String,
    description: String,
    auto_init: bool,
    default_branch: String,
}

/// Request body for creating an issue
#[derive(Debug, Serialize)]
struct CreateIssueRequest {
    title: String,
    body: String,
    labels: Vec<i64>,
}

/// Request body for updating pull request state
#[derive(Debug, Serialize)]
struct UpdatePullRequestRequest {
    state: String,
}

impl GiteaClient {
    /// Creates a new Gitea API client
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the Gitea instance (e.g., "https://gitea.example.com")
    /// * `token` - API token for authentication
    pub fn new(base_url: String, token: String) -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url,
            token,
            client,
        }
    }

    /// Creates a new repository in Gitea
    ///
    /// # Arguments
    ///
    /// * `name` - Repository name
    /// * `description` - Repository description
    ///
    /// # Returns
    ///
    /// Returns the created repository information or an error message
    pub async fn create_repository(
        &self,
        name: &str,
        description: &str,
    ) -> Result<GiteaRepository, String> {
        let url = format!("{}/api/v1/user/repos", self.base_url);
        
        let request_body = CreateRepoRequest {
            name: name.to_string(),
            description: description.to_string(),
            auto_init: true,
            default_branch: "main".to_string(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send create repository request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to create repository (status {}): {}",
                status, error_text
            ));
        }

        response
            .json::<GiteaRepository>()
            .await
            .map_err(|e| format!("Failed to parse repository response: {}", e))
    }

    /// Deletes a repository from Gitea
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner username
    /// * `repo` - Repository name
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    pub async fn delete_repository(&self, owner: &str, repo: &str) -> Result<(), String> {
        let url = format!("{}/api/v1/repos/{}/{}", self.base_url, owner, repo);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send delete repository request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to delete repository (status {}): {}",
                status, error_text
            ));
        }

        Ok(())
    }

    /// Creates an issue in a repository
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner username
    /// * `repo` - Repository name
    /// * `title` - Issue title
    /// * `body` - Issue body/description
    /// * `labels` - List of label names to apply
    ///
    /// # Returns
    ///
    /// Returns the created issue information or an error message
    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        labels: Vec<String>,
    ) -> Result<GiteaIssue, String> {
        // First, get label IDs from label names
        let label_ids = self.get_label_ids(owner, repo, labels).await?;

        let url = format!("{}/api/v1/repos/{}/{}/issues", self.base_url, owner, repo);

        let request_body = CreateIssueRequest {
            title: title.to_string(),
            body: body.to_string(),
            labels: label_ids,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send create issue request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to create issue (status {}): {}",
                status, error_text
            ));
        }

        response
            .json::<GiteaIssue>()
            .await
            .map_err(|e| format!("Failed to parse issue response: {}", e))
    }

    /// Gets a pull request by number
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner username
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    ///
    /// # Returns
    ///
    /// Returns the pull request information or an error message
    pub async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GiteaPullRequest, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send get pull request request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to get pull request (status {}): {}",
                status, error_text
            ));
        }

        response
            .json::<GiteaPullRequest>()
            .await
            .map_err(|e| format!("Failed to parse pull request response: {}", e))
    }

    /// Closes a pull request
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner username
    /// * `repo` - Repository name
    /// * `number` - Pull request number
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    pub async fn close_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<(), String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );

        let request_body = UpdatePullRequestRequest {
            state: "closed".to_string(),
        };

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send close pull request request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to close pull request (status {}): {}",
                status, error_text
            ));
        }

        Ok(())
    }

    /// Deletes a branch from a repository
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner username
    /// * `repo` - Repository name
    /// * `branch` - Branch name to delete
    ///
    /// # Returns
    ///
    /// Returns Ok(()) on success or an error message
    pub async fn delete_branch(&self, owner: &str, repo: &str, branch: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/branches/{}",
            self.base_url, owner, repo, branch
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| format!("Failed to send delete branch request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Failed to delete branch (status {}): {}",
                status, error_text
            ));
        }

        Ok(())
    }

    /// Helper function to get label IDs from label names
    ///
    /// If a label doesn't exist, it will be created
    async fn get_label_ids(
        &self,
        owner: &str,
        repo: &str,
        label_names: Vec<String>,
    ) -> Result<Vec<i64>, String> {
        let mut label_ids = Vec::new();

        for label_name in label_names {
            // Try to get existing labels
            let url = format!("{}/api/v1/repos/{}/{}/labels", self.base_url, owner, repo);

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("token {}", self.token))
                .send()
                .await
                .map_err(|e| format!("Failed to get labels: {}", e))?;

            if response.status().is_success() {
                let labels: Vec<GiteaLabel> = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse labels: {}", e))?;

                if let Some(label) = labels.iter().find(|l| l.name == label_name) {
                    label_ids.push(label.id);
                    continue;
                }
            }

            // Label doesn't exist, create it
            let create_url = format!("{}/api/v1/repos/{}/{}/labels", self.base_url, owner, repo);

            #[derive(Serialize)]
            struct CreateLabelRequest {
                name: String,
                color: String,
                description: String,
            }

            let create_body = CreateLabelRequest {
                name: label_name.clone(),
                color: "00aabb".to_string(),
                description: format!("Auto-created label: {}", label_name),
            };

            let create_response = self
                .client
                .post(&create_url)
                .header("Authorization", format!("token {}", self.token))
                .header("Content-Type", "application/json")
                .json(&create_body)
                .send()
                .await
                .map_err(|e| format!("Failed to create label: {}", e))?;

            if create_response.status().is_success() {
                let new_label: GiteaLabel = create_response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse created label: {}", e))?;
                label_ids.push(new_label.id);
            } else {
                return Err(format!("Failed to create label: {}", label_name));
            }
        }

        Ok(label_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitea_client_creation() {
        let client = GiteaClient::new(
            "https://gitea.example.com".to_string(),
            "test-token".to_string(),
        );

        assert_eq!(client.base_url, "https://gitea.example.com");
        assert_eq!(client.token, "test-token");
    }
}
