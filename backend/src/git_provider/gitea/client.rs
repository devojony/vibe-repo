use async_trait::async_trait;
use reqwest::Response;
use serde::de::DeserializeOwned;

use crate::git_provider::{
    error::GitProviderError,
    models::{
        CreateBranchRequest, CreateIssueRequest, CreateLabelRequest, CreatePullRequestRequest,
        CreateWebhookRequest, GitBranch, GitIssue, GitLabel, GitPullRequest, GitRepository,
        GitUser, GitWebhook, IssueFilter, MergeOptions, PullRequestFilter, UpdateIssueRequest,
        UpdatePullRequestRequest,
    },
    traits::GitProvider,
};

use super::models::{GiteaRepository, GiteaUser};

/// Gitea API client implementation
pub struct GiteaClient {
    base_url: String,
    access_token: String,
    http_client: reqwest::Client,
}

impl GiteaClient {
    /// Create a new Gitea client
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the Gitea instance (e.g., "https://gitea.example.com")
    /// * `access_token` - Personal access token for authentication
    pub fn new(base_url: &str, access_token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            access_token: access_token.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Build API URL from path
    ///
    /// # Arguments
    /// * `path` - API path (should start with "/")
    ///
    /// # Returns
    /// Full API URL (e.g., "https://gitea.example.com/api/v1/user")
    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    /// Get authorization header value
    ///
    /// # Returns
    /// Authorization header value in format "token {access_token}"
    fn auth_header(&self) -> String {
        format!("token {}", self.access_token)
    }

    /// Handle API response and deserialize JSON
    ///
    /// # Arguments
    /// * `response` - HTTP response from reqwest
    ///
    /// # Returns
    /// Deserialized response body or GitProviderError
    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, GitProviderError> {
        let status = response.status().as_u16();

        if (200..300).contains(&status) {
            response
                .json::<T>()
                .await
                .map_err(|e| GitProviderError::ParseError(e.to_string()))
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }
}

#[async_trait]
impl GitProvider for GiteaClient {
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError> {
        let url = self.api_url("/user");

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 200 {
            let gitea_user: GiteaUser = response
                .json()
                .await
                .map_err(|e| GitProviderError::ParseError(e.to_string()))?;
            Ok((true, Some(gitea_user.into())))
        } else if status == 401 {
            Ok((false, None))
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn get_current_user(&self) -> Result<GitUser, GitProviderError> {
        let url = self.api_url("/user");

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_user: GiteaUser = self.handle_response(response).await?;
        Ok(gitea_user.into())
    }

    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError> {
        let url = self.api_url("/user/repos");

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_repos: Vec<GiteaRepository> = self.handle_response(response).await?;
        Ok(gitea_repos.into_iter().map(|r| r.into()).collect())
    }

    async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<GitRepository, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}", owner, repo));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_repo: GiteaRepository = self.handle_response(response).await?;
        Ok(gitea_repo.into())
    }

    async fn list_branches(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/branches", owner, repo));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        // Handle null response (empty repository with no branches)
        let gitea_branches: Option<Vec<super::models::GiteaBranch>> =
            self.handle_response(response).await?;
        Ok(gitea_branches
            .unwrap_or_default()
            .into_iter()
            .map(|b| b.into())
            .collect())
    }

    async fn get_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<GitBranch, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/branches/{}", owner, repo, branch));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_branch: super::models::GiteaBranch = self.handle_response(response).await?;
        Ok(gitea_branch.into())
    }

    async fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/branches", owner, repo));

        // Build request body matching Gitea API format
        let body = serde_json::json!({
            "new_branch_name": req.name,
            "old_ref_name": req.source,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        // Handle 409 Conflict as BranchAlreadyExists
        if status == 409 {
            let message = response.text().await.unwrap_or_default();
            return Err(GitProviderError::BranchAlreadyExists(message));
        }

        // Handle 404 as NotFound (source branch doesn't exist)
        if status == 404 {
            let message = response.text().await.unwrap_or_default();
            return Err(GitProviderError::NotFound(message));
        }

        if (200..300).contains(&status) {
            let gitea_branch: super::models::GiteaBranch = response
                .json()
                .await
                .map_err(|e| GitProviderError::ParseError(e.to_string()))?;
            Ok(gitea_branch.into())
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn delete_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<(), GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/branches/{}", owner, repo, branch));

        let response = self
            .http_client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if (200..300).contains(&status) {
            Ok(())
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError> {
        let mut url = self.api_url(&format!("/repos/{}/{}/issues", owner, repo));

        // Build query parameters
        let mut query_params = Vec::new();

        if let Some(filter) = filter {
            if let Some(state) = filter.state {
                let state_str = match state {
                    crate::git_provider::models::IssueState::Open => "open",
                    crate::git_provider::models::IssueState::Closed => "closed",
                };
                query_params.push(format!("state={}", state_str));
            }

            if let Some(labels) = filter.labels {
                if !labels.is_empty() {
                    query_params.push(format!("labels={}", labels.join(",")));
                }
            }

            if let Some(assignee) = filter.assignee {
                query_params.push(format!("assignee={}", assignee));
            }
        }

        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_issues: Vec<super::models::GiteaIssue> = self.handle_response(response).await?;
        Ok(gitea_issues.into_iter().map(|i| i.into()).collect())
    }

    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitIssue, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/issues/{}", owner, repo, number));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_issue: super::models::GiteaIssue = self.handle_response(response).await?;
        Ok(gitea_issue.into())
    }

    async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/issues", owner, repo));

        // Build request body matching Gitea API format
        let mut body = serde_json::json!({
            "title": req.title,
        });

        if let Some(issue_body) = req.body {
            body["body"] = serde_json::json!(issue_body);
        }

        if let Some(labels) = req.labels {
            body["labels"] = serde_json::json!(labels);
        }

        if let Some(assignees) = req.assignees {
            body["assignees"] = serde_json::json!(assignees);
        }

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_issue: super::models::GiteaIssue = self.handle_response(response).await?;
        Ok(gitea_issue.into())
    }

    async fn update_issue(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/issues/{}", owner, repo, number));

        // Build request body with only provided fields
        let mut body = serde_json::json!({});

        if let Some(title) = req.title {
            body["title"] = serde_json::json!(title);
        }

        if let Some(issue_body) = req.body {
            body["body"] = serde_json::json!(issue_body);
        }

        if let Some(state) = req.state {
            let state_str = match state {
                crate::git_provider::models::IssueState::Open => "open",
                crate::git_provider::models::IssueState::Closed => "closed",
            };
            body["state"] = serde_json::json!(state_str);
        }

        if let Some(labels) = req.labels {
            body["labels"] = serde_json::json!(labels);
        }

        if let Some(assignees) = req.assignees {
            body["assignees"] = serde_json::json!(assignees);
        }

        let response = self
            .http_client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_issue: super::models::GiteaIssue = self.handle_response(response).await?;
        Ok(gitea_issue.into())
    }

    async fn add_issue_labels(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        let url = self.api_url(&format!(
            "/repos/{}/{}/issues/{}/labels",
            owner, repo, number
        ));

        // Gitea API expects { "labels": [label_ids] } but we're using label names
        // According to Gitea API docs, we can use label names directly
        let body = serde_json::json!({
            "labels": labels,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_labels: Vec<super::models::GiteaLabel> = self.handle_response(response).await?;
        Ok(gitea_labels.into_iter().map(|l| l.into()).collect())
    }

    async fn remove_issue_label(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        label: &str,
    ) -> Result<(), GitProviderError> {
        let url = self.api_url(&format!(
            "/repos/{}/{}/issues/{}/labels/{}",
            owner, repo, number, label
        ));

        let response = self
            .http_client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if (200..300).contains(&status) {
            Ok(())
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError> {
        let mut url = self.api_url(&format!("/repos/{}/{}/pulls", owner, repo));

        // Build query parameters
        let mut query_params = Vec::new();

        if let Some(filter) = filter {
            if let Some(state) = filter.state {
                let state_str = match state {
                    crate::git_provider::models::PullRequestState::Open => "open",
                    crate::git_provider::models::PullRequestState::Closed => "closed",
                    crate::git_provider::models::PullRequestState::Merged => "closed", // Gitea uses "closed" for merged PRs
                };
                query_params.push(format!("state={}", state_str));
            }

            if let Some(labels) = filter.labels {
                if !labels.is_empty() {
                    query_params.push(format!("labels={}", labels.join(",")));
                }
            }
        }

        if !query_params.is_empty() {
            url = format!("{}?{}", url, query_params.join("&"));
        }

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_prs: Vec<super::models::GiteaPullRequest> =
            self.handle_response(response).await?;
        Ok(gitea_prs.into_iter().map(|pr| pr.into()).collect())
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
    ) -> Result<GitPullRequest, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/pulls/{}", owner, repo, number));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_pr: super::models::GiteaPullRequest = self.handle_response(response).await?;
        Ok(gitea_pr.into())
    }

    async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/pulls", owner, repo));

        // Build request body matching Gitea API format
        let mut body = serde_json::json!({
            "title": req.title,
            "head": req.head,
            "base": req.base,
        });

        if let Some(pr_body) = req.body {
            body["body"] = serde_json::json!(pr_body);
        }

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_pr: super::models::GiteaPullRequest = self.handle_response(response).await?;
        Ok(gitea_pr.into())
    }

    async fn update_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/pulls/{}", owner, repo, number));

        // Build request body with only provided fields
        let mut body = serde_json::json!({});

        if let Some(title) = req.title {
            body["title"] = serde_json::json!(title);
        }

        if let Some(pr_body) = req.body {
            body["body"] = serde_json::json!(pr_body);
        }

        if let Some(state) = req.state {
            let state_str = match state {
                crate::git_provider::models::PullRequestState::Open => "open",
                crate::git_provider::models::PullRequestState::Closed => "closed",
                crate::git_provider::models::PullRequestState::Merged => "closed", // Gitea uses "closed" for merged PRs
            };
            body["state"] = serde_json::json!(state_str);
        }

        let response = self
            .http_client
            .patch(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_pr: super::models::GiteaPullRequest = self.handle_response(response).await?;
        Ok(gitea_pr.into())
    }

    async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: i64,
        opts: MergeOptions,
    ) -> Result<(), GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/pulls/{}/merge", owner, repo, number));

        // Build request body with merge options
        let merge_method = match opts.strategy {
            crate::git_provider::models::MergeStrategy::Merge => "merge",
            crate::git_provider::models::MergeStrategy::Rebase => "rebase",
            crate::git_provider::models::MergeStrategy::Squash => "squash",
        };

        let body = serde_json::json!({
            "Do": merge_method,
            "delete_branch_after_merge": opts.delete_branch,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if (200..300).contains(&status) {
            Ok(())
        } else if status == 405 {
            // 405 Method Not Allowed typically means the PR is not mergeable
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::NotMergeable(message))
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn list_labels(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/labels", owner, repo));

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let gitea_labels: Vec<super::models::GiteaLabel> = self.handle_response(response).await?;
        Ok(gitea_labels.into_iter().map(|l| l.into()).collect())
    }

    async fn create_label(
        &self,
        owner: &str,
        repo: &str,
        req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/labels", owner, repo));

        // Build request body matching Gitea API format
        let mut body = serde_json::json!({
            "name": req.name,
            "color": req.color,
        });

        if let Some(description) = req.description {
            body["description"] = serde_json::json!(description);
        }

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        // Handle 409 Conflict as LabelAlreadyExists
        if status == 409 {
            let message = response.text().await.unwrap_or_default();
            return Err(GitProviderError::LabelAlreadyExists(message));
        }

        if (200..300).contains(&status) {
            let gitea_label: super::models::GiteaLabel = response
                .json()
                .await
                .map_err(|e| GitProviderError::ParseError(e.to_string()))?;
            Ok(gitea_label.into())
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn delete_label(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> Result<(), GitProviderError> {
        let url = self.api_url(&format!("/repos/{}/{}/labels/{}", owner, repo, name));

        let response = self
            .http_client
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| GitProviderError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if (200..300).contains(&status) {
            Ok(())
        } else {
            let message = response.text().await.unwrap_or_default();
            Err(GitProviderError::from_status(status, message))
        }
    }

    async fn create_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateWebhookRequest,
    ) -> Result<GitWebhook, GitProviderError> {
        // TODO: Implement in Task 2.3
        Err(GitProviderError::UnsupportedOperation(
            "Webhook operations not yet implemented for Gitea".to_string(),
        ))
    }

    async fn delete_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _webhook_id: &str,
    ) -> Result<(), GitProviderError> {
        // TODO: Implement in Task 2.3
        Err(GitProviderError::UnsupportedOperation(
            "Webhook operations not yet implemented for Gitea".to_string(),
        ))
    }

    async fn list_webhooks(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitWebhook>, GitProviderError> {
        // TODO: Implement in Task 2.3
        Err(GitProviderError::UnsupportedOperation(
            "Webhook operations not yet implemented for Gitea".to_string(),
        ))
    }

    fn provider_type(&self) -> &'static str {
        "gitea"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let client = GiteaClient::new("https://gitea.example.com", "test_token");
        assert_eq!(client.base_url, "https://gitea.example.com");
        assert_eq!(client.access_token, "test_token");
    }

    #[test]
    fn test_new_client_trims_trailing_slash() {
        let client = GiteaClient::new("https://gitea.example.com/", "test_token");
        assert_eq!(client.base_url, "https://gitea.example.com");
    }

    #[test]
    fn test_api_url() {
        let client = GiteaClient::new("https://gitea.example.com", "test_token");
        assert_eq!(
            client.api_url("/user"),
            "https://gitea.example.com/api/v1/user"
        );
        assert_eq!(
            client.api_url("/repos/owner/repo"),
            "https://gitea.example.com/api/v1/repos/owner/repo"
        );
    }

    #[test]
    fn test_auth_header() {
        let client = GiteaClient::new("https://gitea.example.com", "test_token_123");
        assert_eq!(client.auth_header(), "token test_token_123");
    }

    #[test]
    fn test_provider_type() {
        let client = GiteaClient::new("https://gitea.example.com", "test_token");
        assert_eq!(client.provider_type(), "gitea");
    }

    #[test]
    fn test_base_url_getter() {
        let client = GiteaClient::new("https://gitea.example.com", "test_token");
        assert_eq!(client.base_url(), "https://gitea.example.com");
    }
}
