//! Integration tests for webhook payload parsing

use vibe_repo::api::webhooks::models::*;

/// Test parsing valid Gitea issue comment payload
#[test]
fn test_parse_gitea_issue_comment_payload_valid() {
    let payload_json = r#"{
        "action": "created",
        "issue": {
            "id": 123,
            "number": 42,
            "title": "Test Issue",
            "body": "Issue description",
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "@vibe implement this feature",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": "test@example.com",
                "avatar_url": "https://example.com/avatar.png"
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": "test@example.com",
            "avatar_url": "https://example.com/avatar.png"
        }
    }"#;

    let payload: GiteaIssueCommentPayload = serde_json::from_str(payload_json)
        .expect("Failed to parse valid Gitea issue comment payload");

    assert_eq!(payload.action, "created");
    assert_eq!(payload.issue.number, 42);
    assert_eq!(payload.issue.title, "Test Issue");
    assert_eq!(payload.comment.id, 456);
    assert_eq!(payload.comment.body, "@vibe implement this feature");
    assert_eq!(payload.comment.user.login, "testuser");
    assert_eq!(payload.repository.full_name, "testorg/test-repo");
}

/// Test parsing valid Gitea PR comment payload
#[test]
fn test_parse_gitea_pr_comment_payload_valid() {
    let payload_json = r#"{
        "action": "created",
        "pull_request": {
            "id": 123,
            "number": 42,
            "title": "Test PR",
            "body": "PR description",
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "@vibe review this code",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": "test@example.com",
                "avatar_url": "https://example.com/avatar.png"
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": "test@example.com",
            "avatar_url": "https://example.com/avatar.png"
        }
    }"#;

    let payload: GiteaPullRequestCommentPayload = serde_json::from_str(payload_json)
        .expect("Failed to parse valid Gitea PR comment payload");

    assert_eq!(payload.action, "created");
    assert_eq!(payload.pull_request.number, 42);
    assert_eq!(payload.pull_request.title, "Test PR");
    assert_eq!(payload.comment.id, 456);
    assert_eq!(payload.comment.body, "@vibe review this code");
}

/// Test extracting CommentInfo from issue comment payload
#[test]
fn test_extract_comment_info_from_issue_comment() {
    let payload_json = r#"{
        "action": "created",
        "issue": {
            "id": 123,
            "number": 42,
            "title": "Test Issue",
            "body": "Issue description",
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "@vibe implement this feature",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": "test@example.com",
                "avatar_url": "https://example.com/avatar.png"
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": "test@example.com",
            "avatar_url": "https://example.com/avatar.png"
        }
    }"#;

    let payload: GiteaIssueCommentPayload = serde_json::from_str(payload_json).unwrap();
    let comment_info = payload.extract_comment_info().expect("Failed to extract comment info");

    assert_eq!(comment_info.comment_id, "456");
    assert_eq!(comment_info.comment_body, "@vibe implement this feature");
    assert_eq!(comment_info.comment_author, "testuser");
    assert_eq!(comment_info.issue_or_pr_number, 42);
    assert_eq!(comment_info.repository_full_name, "testorg/test-repo");
    assert_eq!(comment_info.action, "created");
    assert_eq!(comment_info.comment_type, CommentType::Issue);
    assert_eq!(comment_info.created_at, "2024-01-15T10:30:00Z");
    assert_eq!(comment_info.updated_at, "2024-01-15T10:30:00Z");
}

/// Test extracting CommentInfo from PR comment payload
#[test]
fn test_extract_comment_info_from_pr_comment() {
    let payload_json = r#"{
        "action": "created",
        "pull_request": {
            "id": 123,
            "number": 42,
            "title": "Test PR",
            "body": "PR description",
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "@vibe review this code",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": "test@example.com",
                "avatar_url": "https://example.com/avatar.png"
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": "test@example.com",
            "avatar_url": "https://example.com/avatar.png"
        }
    }"#;

    let payload: GiteaPullRequestCommentPayload = serde_json::from_str(payload_json).unwrap();
    let comment_info = payload.extract_comment_info().expect("Failed to extract comment info");

    assert_eq!(comment_info.comment_id, "456");
    assert_eq!(comment_info.comment_body, "@vibe review this code");
    assert_eq!(comment_info.comment_author, "testuser");
    assert_eq!(comment_info.issue_or_pr_number, 42);
    assert_eq!(comment_info.repository_full_name, "testorg/test-repo");
    assert_eq!(comment_info.action, "created");
    assert_eq!(comment_info.comment_type, CommentType::PullRequest);
}

/// Test handling invalid JSON
#[test]
fn test_parse_invalid_json() {
    let invalid_json = r#"{"invalid": json}"#;

    let result: Result<GiteaIssueCommentPayload, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
}

/// Test handling missing required fields
#[test]
fn test_parse_missing_fields() {
    let incomplete_json = r#"{
        "action": "created",
        "issue": {
            "id": 123,
            "number": 42
        }
    }"#;

    let result: Result<GiteaIssueCommentPayload, _> = serde_json::from_str(incomplete_json);
    assert!(result.is_err());
}

/// Test different action types
#[test]
fn test_different_action_types() {
    let actions = vec!["created", "edited", "deleted"];

    for action in actions {
        let payload_json = format!(
            r#"{{
                "action": "{}",
                "issue": {{
                    "id": 123,
                    "number": 42,
                    "title": "Test Issue",
                    "body": "Issue description",
                    "state": "open"
                }},
                "comment": {{
                    "id": 456,
                    "body": "Test comment",
                    "user": {{
                        "id": 789,
                        "login": "testuser",
                        "email": null,
                        "avatar_url": null
                    }},
                    "created_at": "2024-01-15T10:30:00Z",
                    "updated_at": "2024-01-15T10:30:00Z"
                }},
                "repository": {{
                    "id": 111,
                    "name": "test-repo",
                    "full_name": "testorg/test-repo",
                    "owner": {{
                        "id": 222,
                        "login": "testorg",
                        "email": null,
                        "avatar_url": null
                    }}
                }},
                "sender": {{
                    "id": 789,
                    "login": "testuser",
                    "email": null,
                    "avatar_url": null
                }}
            }}"#,
            action
        );

        let payload: GiteaIssueCommentPayload = serde_json::from_str(&payload_json)
            .unwrap_or_else(|_| panic!("Failed to parse payload with action: {}", action));

        let comment_info = payload.extract_comment_info()
            .unwrap_or_else(|_| panic!("Failed to extract comment info for action: {}", action));

        assert_eq!(comment_info.action, action);
    }
}

/// Test action validation - invalid action should fail
#[test]
fn test_invalid_action_validation() {
    let payload_json = r#"{
        "action": "invalid_action",
        "issue": {
            "id": 123,
            "number": 42,
            "title": "Test Issue",
            "body": "Issue description",
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "Test comment",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": null,
                "avatar_url": null
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": null,
            "avatar_url": null
        }
    }"#;

    let payload: GiteaIssueCommentPayload = serde_json::from_str(payload_json).unwrap();
    let result = payload.extract_comment_info();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid action"));
}

/// Test handling optional fields (email, avatar_url)
#[test]
fn test_optional_fields() {
    let payload_json = r#"{
        "action": "created",
        "issue": {
            "id": 123,
            "number": 42,
            "title": "Test Issue",
            "body": null,
            "state": "open"
        },
        "comment": {
            "id": 456,
            "body": "Test comment",
            "user": {
                "id": 789,
                "login": "testuser",
                "email": null,
                "avatar_url": null
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "repository": {
            "id": 111,
            "name": "test-repo",
            "full_name": "testorg/test-repo",
            "owner": {
                "id": 222,
                "login": "testorg",
                "email": null,
                "avatar_url": null
            }
        },
        "sender": {
            "id": 789,
            "login": "testuser",
            "email": null,
            "avatar_url": null
        }
    }"#;

    let payload: GiteaIssueCommentPayload = serde_json::from_str(payload_json)
        .expect("Failed to parse payload with optional fields");

    assert_eq!(payload.issue.body, None);
    assert_eq!(payload.comment.user.email, None);
    assert_eq!(payload.comment.user.avatar_url, None);
}
