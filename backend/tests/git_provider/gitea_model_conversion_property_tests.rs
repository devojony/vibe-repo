//! Property-based tests for Gitea model conversion
//!
//! Tests that conversion from Gitea models to unified models preserves data.

use chrono::Utc;
use vibe_repo::git_provider::gitea::models::{
    GiteaBranch, GiteaCommit, GiteaIssue, GiteaLabel, GiteaPRBranch, GiteaPermissions,
    GiteaPullRequest, GiteaRepository, GiteaUser,
};
use vibe_repo::git_provider::models::{
    GitBranch, GitIssue, GitLabel, GitPullRequest, GitRepository, GitUser, IssueState,
    PullRequestState, RepositoryPermissions,
};
use proptest::prelude::*;

// ============================================
// Property 2: Model Conversion Preserves Data
// Validates: Requirements 2.7, 4.3
// ============================================

/// Generate arbitrary Gitea user
fn arb_gitea_user() -> impl Strategy<Value = GiteaUser> {
    (
        1i64..1_000_000,
        "[a-z]{3,15}",
        prop::option::of("[a-z]{3,10}@[a-z]{3,10}\\.[a-z]{2,3}"),
        prop::option::of("https://[a-z]{3,10}\\.[a-z]{2,3}/avatar/[a-z0-9]{10}"),
        prop::option::of("[A-Z][a-z]{3,15} [A-Z][a-z]{3,15}"),
    )
        .prop_map(|(id, login, email, avatar_url, full_name)| GiteaUser {
            id,
            login,
            email,
            avatar_url,
            full_name,
        })
}

/// Generate arbitrary Gitea permissions
fn arb_gitea_permissions() -> impl Strategy<Value = GiteaPermissions> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(|(admin, push, pull)| GiteaPermissions {
        admin,
        push,
        pull,
    })
}

/// Generate arbitrary Gitea repository
fn arb_gitea_repository() -> impl Strategy<Value = GiteaRepository> {
    (
        1i64..1_000_000,
        "[a-z]{3,15}",
        "[a-z]{3,15}/[a-z]{3,15}",
        prop::option::of("[A-Z][a-z]{5,30}"),
        "https://[a-z]{3,10}\\.[a-z]{2,3}/[a-z]{3,15}/[a-z]{3,15}\\.git",
        prop::option::of("git@[a-z]{3,10}\\.[a-z]{2,3}:[a-z]{3,15}/[a-z]{3,15}\\.git"),
        "(main|master|develop)",
        any::<bool>(),
        prop::option::of(arb_gitea_permissions()),
    )
        .prop_map(
            |(
                id,
                name,
                full_name,
                description,
                clone_url,
                ssh_url,
                default_branch,
                private,
                permissions,
            )| {
                GiteaRepository {
                    id,
                    name,
                    full_name,
                    description,
                    clone_url,
                    ssh_url,
                    default_branch,
                    private,
                    permissions,
                }
            },
        )
}

/// Generate arbitrary Gitea commit
fn arb_gitea_commit() -> impl Strategy<Value = GiteaCommit> {
    "[a-f0-9]{40}".prop_map(|id| GiteaCommit { id })
}

/// Generate arbitrary Gitea branch
fn arb_gitea_branch() -> impl Strategy<Value = GiteaBranch> {
    (
        "(main|master|develop|feature/[a-z]{3,10})",
        arb_gitea_commit(),
        any::<bool>(),
    )
        .prop_map(|(name, commit, protected)| GiteaBranch {
            name,
            commit,
            protected,
        })
}

/// Generate arbitrary Gitea label
fn arb_gitea_label() -> impl Strategy<Value = GiteaLabel> {
    (
        1i64..1_000_000,
        "[a-z]{3,15}",
        "[0-9a-fA-F]{6}",
        prop::option::of("[A-Z][a-z]{5,30}"),
    )
        .prop_map(|(id, name, color, description)| GiteaLabel {
            id,
            name,
            color,
            description,
        })
}

/// Generate arbitrary Gitea issue
fn arb_gitea_issue() -> impl Strategy<Value = GiteaIssue> {
    (
        1i64..1_000_000,
        "[A-Z][a-z]{5,30}",
        prop::option::of("[A-Z][a-z]{10,50}"),
        prop::sample::select(vec!["open", "closed"]),
        prop::collection::vec(arb_gitea_label(), 0..5),
        prop::collection::vec(arb_gitea_user(), 0..3),
    )
        .prop_map(|(number, title, body, state, labels, assignees)| {
            let created_at = Utc::now();
            let updated_at = Utc::now();
            GiteaIssue {
                number,
                title,
                body,
                state: state.to_string(),
                labels,
                assignees,
                created_at,
                updated_at,
            }
        })
}

/// Generate arbitrary Gitea PR branch
fn arb_gitea_pr_branch() -> impl Strategy<Value = GiteaPRBranch> {
    ("(main|master|develop|feature/[a-z]{3,10})", "[a-f0-9]{40}")
        .prop_map(|(ref_name, sha)| GiteaPRBranch { ref_name, sha })
}

/// Generate arbitrary Gitea pull request
fn arb_gitea_pull_request() -> impl Strategy<Value = GiteaPullRequest> {
    (
        1i64..1_000_000,
        "[A-Z][a-z]{5,30}",
        prop::option::of("[A-Z][a-z]{10,50}"),
        prop::sample::select(vec!["open", "closed"]),
        arb_gitea_pr_branch(),
        arb_gitea_pr_branch(),
        prop::option::of(any::<bool>()),
        any::<bool>(),
        prop::collection::vec(arb_gitea_label(), 0..5),
    )
        .prop_map(
            |(number, title, body, state, head, base, mergeable, merged, labels)| {
                let created_at = Utc::now();
                let updated_at = Utc::now();
                GiteaPullRequest {
                    number,
                    title,
                    body,
                    state: state.to_string(),
                    head,
                    base,
                    mergeable,
                    merged,
                    labels,
                    created_at,
                    updated_at,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaUser, converting to GitUser should preserve id, username, email, and avatar_url
    #[test]
    fn prop_gitea_user_conversion_preserves_data(gitea_user in arb_gitea_user()) {
        let git_user: GitUser = gitea_user.clone().into();

        prop_assert_eq!(git_user.id, gitea_user.id.to_string(), "ID should be preserved");
        prop_assert_eq!(git_user.username, gitea_user.login, "Username should be preserved");
        prop_assert_eq!(git_user.email, gitea_user.email, "Email should be preserved");
        prop_assert_eq!(git_user.avatar_url, gitea_user.avatar_url, "Avatar URL should be preserved");
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaRepository, converting to GitRepository should preserve all fields
    #[test]
    fn prop_gitea_repository_conversion_preserves_data(gitea_repo in arb_gitea_repository()) {
        let git_repo: GitRepository = gitea_repo.clone().into();

        prop_assert_eq!(git_repo.id, gitea_repo.id.to_string(), "ID should be preserved");
        prop_assert_eq!(git_repo.name, gitea_repo.name, "Name should be preserved");
        prop_assert_eq!(git_repo.full_name, gitea_repo.full_name, "Full name should be preserved");
        prop_assert_eq!(git_repo.description, gitea_repo.description, "Description should be preserved");
        prop_assert_eq!(git_repo.clone_url, gitea_repo.clone_url, "Clone URL should be preserved");
        prop_assert_eq!(git_repo.ssh_url, gitea_repo.ssh_url, "SSH URL should be preserved");
        prop_assert_eq!(git_repo.default_branch, gitea_repo.default_branch, "Default branch should be preserved");
        prop_assert_eq!(git_repo.private, gitea_repo.private, "Private flag should be preserved");

        // Check permissions
        if let Some(gitea_perms) = gitea_repo.permissions {
            prop_assert_eq!(git_repo.permissions.admin, gitea_perms.admin, "Admin permission should be preserved");
            prop_assert_eq!(git_repo.permissions.push, gitea_perms.push, "Push permission should be preserved");
            prop_assert_eq!(git_repo.permissions.pull, gitea_perms.pull, "Pull permission should be preserved");
        } else {
            // Default permissions when None
            prop_assert!(!git_repo.permissions.admin, "Default admin should be false");
            prop_assert!(!git_repo.permissions.push, "Default push should be false");
            prop_assert!(git_repo.permissions.pull, "Default pull should be true");
        }
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaBranch, converting to GitBranch should preserve name, commit SHA, and protected status
    #[test]
    fn prop_gitea_branch_conversion_preserves_data(gitea_branch in arb_gitea_branch()) {
        let git_branch: GitBranch = gitea_branch.clone().into();

        prop_assert_eq!(git_branch.name, gitea_branch.name, "Name should be preserved");
        prop_assert_eq!(git_branch.commit_sha, gitea_branch.commit.id, "Commit SHA should be preserved");
        prop_assert_eq!(git_branch.protected, gitea_branch.protected, "Protected status should be preserved");
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaLabel, converting to GitLabel should preserve all fields
    #[test]
    fn prop_gitea_label_conversion_preserves_data(gitea_label in arb_gitea_label()) {
        let git_label: GitLabel = gitea_label.clone().into();

        prop_assert_eq!(git_label.id, gitea_label.id, "ID should be preserved");
        prop_assert_eq!(git_label.name, gitea_label.name, "Name should be preserved");
        prop_assert_eq!(git_label.color, gitea_label.color, "Color should be preserved");
        prop_assert_eq!(git_label.description, gitea_label.description, "Description should be preserved");
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaIssue, converting to GitIssue should preserve all fields
    #[test]
    fn prop_gitea_issue_conversion_preserves_data(gitea_issue in arb_gitea_issue()) {
        let git_issue: GitIssue = gitea_issue.clone().into();

        prop_assert_eq!(git_issue.number, gitea_issue.number, "Number should be preserved");
        prop_assert_eq!(git_issue.title, gitea_issue.title, "Title should be preserved");
        prop_assert_eq!(git_issue.body, gitea_issue.body, "Body should be preserved");

        // Check state conversion
        let expected_state = match gitea_issue.state.to_lowercase().as_str() {
            "closed" => IssueState::Closed,
            _ => IssueState::Open,
        };
        prop_assert_eq!(git_issue.state, expected_state, "State should be correctly converted");

        // Check labels
        prop_assert_eq!(git_issue.labels.len(), gitea_issue.labels.len(), "Label count should be preserved");
        for (i, label) in gitea_issue.labels.iter().enumerate() {
            prop_assert_eq!(&git_issue.labels[i], &label.name, "Label name should be preserved");
        }

        // Check assignees
        prop_assert_eq!(git_issue.assignees.len(), gitea_issue.assignees.len(), "Assignee count should be preserved");
        for (i, assignee) in gitea_issue.assignees.iter().enumerate() {
            prop_assert_eq!(&git_issue.assignees[i], &assignee.login, "Assignee username should be preserved");
        }

        prop_assert_eq!(git_issue.created_at, gitea_issue.created_at, "Created timestamp should be preserved");
        prop_assert_eq!(git_issue.updated_at, gitea_issue.updated_at, "Updated timestamp should be preserved");
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaPullRequest, converting to GitPullRequest should preserve all fields
    #[test]
    fn prop_gitea_pull_request_conversion_preserves_data(gitea_pr in arb_gitea_pull_request()) {
        let git_pr: GitPullRequest = gitea_pr.clone().into();

        prop_assert_eq!(git_pr.number, gitea_pr.number, "Number should be preserved");
        prop_assert_eq!(git_pr.title, gitea_pr.title, "Title should be preserved");
        prop_assert_eq!(git_pr.body, gitea_pr.body, "Body should be preserved");

        // Check state conversion
        let expected_state = if gitea_pr.merged {
            PullRequestState::Merged
        } else {
            match gitea_pr.state.to_lowercase().as_str() {
                "closed" => PullRequestState::Closed,
                _ => PullRequestState::Open,
            }
        };
        prop_assert_eq!(git_pr.state, expected_state, "State should be correctly converted");

        prop_assert_eq!(git_pr.source_branch, gitea_pr.head.ref_name, "Source branch should be preserved");
        prop_assert_eq!(git_pr.target_branch, gitea_pr.base.ref_name, "Target branch should be preserved");
        prop_assert_eq!(git_pr.mergeable, gitea_pr.mergeable, "Mergeable status should be preserved");
        prop_assert_eq!(git_pr.merged, gitea_pr.merged, "Merged status should be preserved");

        // Check labels
        prop_assert_eq!(git_pr.labels.len(), gitea_pr.labels.len(), "Label count should be preserved");
        for (i, label) in gitea_pr.labels.iter().enumerate() {
            prop_assert_eq!(&git_pr.labels[i], &label.name, "Label name should be preserved");
        }

        prop_assert_eq!(git_pr.created_at, gitea_pr.created_at, "Created timestamp should be preserved");
        prop_assert_eq!(git_pr.updated_at, gitea_pr.updated_at, "Updated timestamp should be preserved");
    }

    /// Feature: git-provider-abstraction, Property 2: Model Conversion Preserves Data
    /// For any GiteaPermissions, converting to RepositoryPermissions should preserve all flags
    #[test]
    fn prop_gitea_permissions_conversion_preserves_data(gitea_perms in arb_gitea_permissions()) {
        let repo_perms: RepositoryPermissions = gitea_perms.clone().into();

        prop_assert_eq!(repo_perms.admin, gitea_perms.admin, "Admin permission should be preserved");
        prop_assert_eq!(repo_perms.push, gitea_perms.push, "Push permission should be preserved");
        prop_assert_eq!(repo_perms.pull, gitea_perms.pull, "Pull permission should be preserved");
    }
}
