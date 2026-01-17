//! Tests for Gitea API null response handling
//!
//! Gitea API returns null instead of empty arrays for some endpoints
//! when there are no items. These tests verify that the client handles
//! this correctly.

use serde::Deserialize;

/// Test that Option<Vec<T>> correctly deserializes null as None
#[test]
fn test_null_deserializes_to_none() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestBranch {
        name: String,
    }

    // Simulate Gitea returning null for empty branches
    let json = "null";
    let result: Option<Vec<TestBranch>> = serde_json::from_str(json).unwrap();
    assert_eq!(result, None);
}

/// Test that empty array deserializes to Some(empty vec)
#[test]
fn test_empty_array_deserializes_to_some_empty() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestBranch {
        name: String,
    }

    let json = "[]";
    let result: Option<Vec<TestBranch>> = serde_json::from_str(json).unwrap();
    assert_eq!(result, Some(vec![]));
}

/// Test that array with items deserializes correctly
#[test]
fn test_array_with_items_deserializes_correctly() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestBranch {
        name: String,
    }

    let json = r#"[{"name": "main"}, {"name": "develop"}]"#;
    let result: Option<Vec<TestBranch>> = serde_json::from_str(json).unwrap();
    assert_eq!(
        result,
        Some(vec![
            TestBranch {
                name: "main".to_string()
            },
            TestBranch {
                name: "develop".to_string()
            }
        ])
    );
}

/// Test unwrap_or_default pattern used in list_branches
#[test]
fn test_unwrap_or_default_handles_null() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestBranch {
        name: String,
    }

    // Simulate null response
    let json = "null";
    let result: Option<Vec<TestBranch>> = serde_json::from_str(json).unwrap();
    let branches = result.unwrap_or_default();
    assert!(branches.is_empty());
}

/// Test unwrap_or_default pattern with actual data
#[test]
fn test_unwrap_or_default_preserves_data() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct TestBranch {
        name: String,
    }

    let json = r#"[{"name": "main"}]"#;
    let result: Option<Vec<TestBranch>> = serde_json::from_str(json).unwrap();
    let branches = result.unwrap_or_default();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "main");
}

/// Test with actual GiteaBranch model structure
#[test]
fn test_gitea_branch_null_response() {
    use vibe_repo::git_provider::gitea::models::GiteaBranch;

    // Simulate null response from Gitea API
    let json = "null";
    let result: Option<Vec<GiteaBranch>> = serde_json::from_str(json).unwrap();
    let branches = result.unwrap_or_default();
    assert!(
        branches.is_empty(),
        "Null response should result in empty vector"
    );
}

/// Test with actual GiteaBranch model and valid data
#[test]
fn test_gitea_branch_valid_response() {
    use vibe_repo::git_provider::gitea::models::GiteaBranch;

    let json = r#"[
        {
            "name": "main",
            "commit": {"id": "abc123def456"},
            "protected": true
        },
        {
            "name": "develop",
            "commit": {"id": "789xyz000111"},
            "protected": false
        }
    ]"#;

    let result: Option<Vec<GiteaBranch>> = serde_json::from_str(json).unwrap();
    let branches = result.unwrap_or_default();

    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].name, "main");
    assert_eq!(branches[0].commit.id, "abc123def456");
    assert!(branches[0].protected);
    assert_eq!(branches[1].name, "develop");
    assert_eq!(branches[1].commit.id, "789xyz000111");
    assert!(!branches[1].protected);
}

/// Test conversion from GiteaBranch to GitBranch with empty list
#[test]
fn test_empty_branch_list_conversion() {
    use vibe_repo::git_provider::gitea::models::GiteaBranch;
    use vibe_repo::git_provider::models::GitBranch;

    let gitea_branches: Vec<GiteaBranch> = vec![];
    let git_branches: Vec<GitBranch> = gitea_branches.into_iter().map(|b| b.into()).collect();

    assert!(git_branches.is_empty());
}
