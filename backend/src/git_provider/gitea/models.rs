use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Gitea user API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
}

/// Gitea repository API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub clone_url: String,
    pub ssh_url: Option<String>,
    pub default_branch: String,
    pub private: bool,
    pub permissions: Option<GiteaPermissions>,
}

/// Gitea repository permissions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaPermissions {
    pub admin: bool,
    pub push: bool,
    pub pull: bool,
}

/// Gitea branch API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaBranch {
    pub name: String,
    pub commit: GiteaCommit,
    pub protected: bool,
}

/// Gitea commit information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaCommit {
    #[serde(alias = "sha")]
    pub id: String,
}

/// Gitea issue API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaIssue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String, // "open" or "closed"
    #[serde(default)]
    pub labels: Vec<GiteaLabel>,
    #[serde(default)]
    pub assignees: Vec<GiteaUser>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Gitea pull request API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaPullRequest {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String, // "open", "closed"
    pub head: GiteaPRBranch,
    pub base: GiteaPRBranch,
    pub mergeable: Option<bool>,
    pub merged: bool,
    #[serde(default)]
    pub labels: Vec<GiteaLabel>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Gitea pull request branch information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaPRBranch {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// Gitea label API response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GiteaLabel {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

// Conversion implementations from Gitea models to unified models
use crate::git_provider::models::{
    GitBranch, GitIssue, GitLabel, GitPullRequest, GitRepository, GitUser, IssueState,
    PullRequestState, RepositoryPermissions,
};

impl From<GiteaUser> for GitUser {
    fn from(gitea_user: GiteaUser) -> Self {
        GitUser {
            id: gitea_user.id.to_string(),
            username: gitea_user.login,
            email: gitea_user.email,
            avatar_url: gitea_user.avatar_url,
        }
    }
}

impl From<GiteaRepository> for GitRepository {
    fn from(gitea_repo: GiteaRepository) -> Self {
        GitRepository {
            id: gitea_repo.id.to_string(),
            name: gitea_repo.name,
            full_name: gitea_repo.full_name,
            description: gitea_repo.description,
            clone_url: gitea_repo.clone_url,
            ssh_url: gitea_repo.ssh_url,
            default_branch: gitea_repo.default_branch,
            private: gitea_repo.private,
            permissions: gitea_repo.permissions.map(|p| p.into()).unwrap_or(
                RepositoryPermissions {
                    admin: false,
                    push: false,
                    pull: true,
                },
            ),
        }
    }
}

impl From<GiteaPermissions> for RepositoryPermissions {
    fn from(gitea_perms: GiteaPermissions) -> Self {
        RepositoryPermissions {
            admin: gitea_perms.admin,
            push: gitea_perms.push,
            pull: gitea_perms.pull,
        }
    }
}

impl From<GiteaBranch> for GitBranch {
    fn from(gitea_branch: GiteaBranch) -> Self {
        GitBranch {
            name: gitea_branch.name,
            commit_sha: gitea_branch.commit.id,
            protected: gitea_branch.protected,
        }
    }
}

impl From<GiteaIssue> for GitIssue {
    fn from(gitea_issue: GiteaIssue) -> Self {
        let state = match gitea_issue.state.to_lowercase().as_str() {
            "closed" => IssueState::Closed,
            _ => IssueState::Open,
        };

        GitIssue {
            number: gitea_issue.number,
            title: gitea_issue.title,
            body: gitea_issue.body,
            state,
            labels: gitea_issue.labels.into_iter().map(|l| l.name).collect(),
            assignees: gitea_issue.assignees.into_iter().map(|u| u.login).collect(),
            created_at: gitea_issue.created_at,
            updated_at: gitea_issue.updated_at,
        }
    }
}

impl From<GiteaPullRequest> for GitPullRequest {
    fn from(gitea_pr: GiteaPullRequest) -> Self {
        let state = if gitea_pr.merged {
            PullRequestState::Merged
        } else {
            match gitea_pr.state.to_lowercase().as_str() {
                "closed" => PullRequestState::Closed,
                _ => PullRequestState::Open,
            }
        };

        GitPullRequest {
            number: gitea_pr.number,
            title: gitea_pr.title,
            body: gitea_pr.body,
            state,
            source_branch: gitea_pr.head.ref_name,
            target_branch: gitea_pr.base.ref_name,
            mergeable: gitea_pr.mergeable,
            merged: gitea_pr.merged,
            labels: gitea_pr.labels.into_iter().map(|l| l.name).collect(),
            created_at: gitea_pr.created_at,
            updated_at: gitea_pr.updated_at,
        }
    }
}

impl From<GiteaLabel> for GitLabel {
    fn from(gitea_label: GiteaLabel) -> Self {
        GitLabel {
            id: gitea_label.id,
            name: gitea_label.name,
            color: gitea_label.color,
            description: gitea_label.description,
        }
    }
}

/// Gitea webhook request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaWebhookRequest {
    pub r#type: String,
    pub config: GiteaWebhookConfig,
    pub events: Vec<String>,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_filter: Option<String>,
}

/// Gitea webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaWebhookConfig {
    pub url: String,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_method: Option<String>,
}

/// Gitea webhook response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaWebhookResponse {
    pub id: i64,
    pub r#type: String,
    pub config: GiteaWebhookConfig,
    pub events: Vec<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
