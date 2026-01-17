use async_trait::async_trait;
use chrono::Utc;
use gitautodev::git_provider::{
    error::GitProviderError,
    models::*,
    traits::GitProvider,
};

/// Mock implementation to test trait compilation
struct MockGitProvider;

#[async_trait]
impl GitProvider for MockGitProvider {
    // Implement all existing methods with minimal implementations
    async fn validate_token(&self) -> Result<(bool, Option<GitUser>), GitProviderError> {
        Ok((true, None))
    }

    async fn get_current_user(&self) -> Result<GitUser, GitProviderError> {
        Ok(GitUser {
            id: "1".to_string(),
            username: "test".to_string(),
            email: None,
            avatar_url: None,
        })
    }

    async fn list_repositories(&self) -> Result<Vec<GitRepository>, GitProviderError> {
        Ok(vec![])
    }

    async fn get_repository(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<GitRepository, GitProviderError> {
        unimplemented!()
    }

    async fn list_branches(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitBranch>, GitProviderError> {
        Ok(vec![])
    }

    async fn get_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<GitBranch, GitProviderError> {
        unimplemented!()
    }

    async fn create_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateBranchRequest,
    ) -> Result<GitBranch, GitProviderError> {
        unimplemented!()
    }

    async fn delete_branch(
        &self,
        _owner: &str,
        _repo: &str,
        _branch: &str,
    ) -> Result<(), GitProviderError> {
        unimplemented!()
    }

    async fn list_issues(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<IssueFilter>,
    ) -> Result<Vec<GitIssue>, GitProviderError> {
        Ok(vec![])
    }

    async fn get_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitIssue, GitProviderError> {
        unimplemented!()
    }

    async fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        unimplemented!()
    }

    async fn update_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdateIssueRequest,
    ) -> Result<GitIssue, GitProviderError> {
        unimplemented!()
    }

    async fn add_issue_labels(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _labels: Vec<String>,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Ok(vec![])
    }

    async fn remove_issue_label(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _label: &str,
    ) -> Result<(), GitProviderError> {
        Ok(())
    }

    async fn list_pull_requests(
        &self,
        _owner: &str,
        _repo: &str,
        _filter: Option<PullRequestFilter>,
    ) -> Result<Vec<GitPullRequest>, GitProviderError> {
        Ok(vec![])
    }

    async fn get_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
    ) -> Result<GitPullRequest, GitProviderError> {
        unimplemented!()
    }

    async fn create_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        unimplemented!()
    }

    async fn update_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _req: UpdatePullRequestRequest,
    ) -> Result<GitPullRequest, GitProviderError> {
        unimplemented!()
    }

    async fn merge_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: i64,
        _opts: MergeOptions,
    ) -> Result<(), GitProviderError> {
        Ok(())
    }

    async fn list_labels(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitLabel>, GitProviderError> {
        Ok(vec![])
    }

    async fn create_label(
        &self,
        _owner: &str,
        _repo: &str,
        _req: CreateLabelRequest,
    ) -> Result<GitLabel, GitProviderError> {
        unimplemented!()
    }

    async fn delete_label(
        &self,
        _owner: &str,
        _repo: &str,
        _name: &str,
    ) -> Result<(), GitProviderError> {
        Ok(())
    }

    // NEW: Webhook methods
    async fn create_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        req: CreateWebhookRequest,
    ) -> Result<GitWebhook, GitProviderError> {
        Ok(GitWebhook {
            id: "1".to_string(),
            url: req.url,
            active: req.active,
            events: req.events,
            created_at: Utc::now(),
        })
    }

    async fn delete_webhook(
        &self,
        _owner: &str,
        _repo: &str,
        _webhook_id: &str,
    ) -> Result<(), GitProviderError> {
        Ok(())
    }

    async fn list_webhooks(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<GitWebhook>, GitProviderError> {
        Ok(vec![])
    }

    fn provider_type(&self) -> &'static str {
        "mock"
    }

    fn base_url(&self) -> &str {
        "https://mock.example.com"
    }
}

#[tokio::test]
async fn test_mock_provider_implements_webhook_methods() {
    let provider = MockGitProvider;

    let req = CreateWebhookRequest {
        url: "https://example.com/webhook".to_string(),
        secret: "secret".to_string(),
        events: vec![WebhookEvent::IssueComment],
        active: true,
    };

    let webhook = provider
        .create_webhook("owner", "repo", req)
        .await
        .unwrap();
    assert_eq!(webhook.id, "1");
    assert!(webhook.active);
}

#[tokio::test]
async fn test_mock_provider_delete_webhook() {
    let provider = MockGitProvider;
    let result = provider.delete_webhook("owner", "repo", "1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_provider_list_webhooks() {
    let provider = MockGitProvider;
    let webhooks = provider.list_webhooks("owner", "repo").await.unwrap();
    assert_eq!(webhooks.len(), 0);
}
