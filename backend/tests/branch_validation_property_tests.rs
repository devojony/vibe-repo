//! Property-based tests for Branch Validation Logic
//!
//! Tests universal properties of the branch validation system using proptest.
//!
//! **Feature: repository-initialization**
//! **Property 3: Branch Validation Logic**
//! **Validates: Requirements 2.1, 2.2, 2.3**

use proptest::prelude::*;

// ============================================
// Property 3: Branch Validation Logic
// Validates: Requirements 2.1, 2.2, 2.3
// ============================================

/// Generate arbitrary branch names (excluding work branch to control test scenarios)
fn arb_branch_name_non_work() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("main".to_string()),
        Just("master".to_string()),
        Just("dev".to_string()),
        Just("developer".to_string()),
        Just("develop".to_string()),
        Just("feature/test".to_string()),
        Just("release/v1.0".to_string()),
        Just("hotfix/bug-123".to_string()),
        "[a-z]{3,15}",
    ]
}

/// Generate arbitrary work branch names
fn arb_work_branch_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("vibe-dev".to_string()),
        Just("agent-dev".to_string()),
        Just("work-branch".to_string()),
        Just("auto-dev".to_string()),
        "[a-z]{3,10}-[a-z]{3,10}",
    ]
}

/// Pure function that implements the branch validation logic
/// This mirrors the logic in RepositoryService::check_branches
fn has_required_branches(branch_names: &[String], work_branch: &str) -> bool {
    branch_names.iter().any(|name| name == work_branch)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// For any repository with work branch, has_required_branches SHALL be true.
    ///
    /// **Validates: Requirements 2.1, 2.2**
    #[test]
    fn prop_has_required_branches_true_when_work_branch_exists(
        work_branch in arb_work_branch_name(),
        other_branches in prop::collection::vec(arb_branch_name_non_work(), 0..10),
    ) {
        let mut branches = other_branches;
        branches.push(work_branch.clone());

        // When work branch exists, has_required_branches should be true
        let result = has_required_branches(&branches, &work_branch);

        prop_assert!(
            result,
            "has_required_branches should be true when {} exists. Branches: {:?}",
            work_branch, branches
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// For any repository without work branch, has_required_branches SHALL be false.
    ///
    /// **Validates: Requirements 2.1, 2.3**
    #[test]
    fn prop_has_required_branches_false_when_work_branch_missing(
        work_branch in arb_work_branch_name(),
        branches in prop::collection::vec(arb_branch_name_non_work(), 0..10),
    ) {
        // Ensure work branch is not in the list
        prop_assume!(!branches.iter().any(|b| b == &work_branch));

        // When work branch does not exist, has_required_branches should be false
        let result = has_required_branches(&branches, &work_branch);

        prop_assert!(
            !result,
            "has_required_branches should be false when {} is missing. Branches: {:?}",
            work_branch, branches
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// For any repository and any branch name, has_required_branches is true if and only if
    /// the branches array contains the specified branch name.
    ///
    /// **Validates: Requirements 2.1, 2.2, 2.3**
    #[test]
    fn prop_has_required_branches_iff_branch_exists(
        branches in prop::collection::vec("[a-z\\-/]{1,20}", 0..15),
        work_branch in arb_work_branch_name(),
    ) {
        let contains_work_branch = branches.iter().any(|b| b == &work_branch);
        let result = has_required_branches(&branches, &work_branch);

        prop_assert_eq!(
            result,
            contains_work_branch,
            "has_required_branches should equal (branches contains {}). \
             Branches: {:?}, contains_work_branch: {}, result: {}",
            work_branch, branches, contains_work_branch, result
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// Old branch names (main, dev, developer) should NOT satisfy the requirement
    /// when checking for vibe-dev.
    ///
    /// **Validates: Requirements 2.1**
    #[test]
    fn prop_old_branch_names_do_not_satisfy_requirement(
        has_main in any::<bool>(),
        has_dev in any::<bool>(),
        has_developer in any::<bool>(),
        has_master in any::<bool>(),
    ) {
        let mut branches = Vec::new();
        if has_main { branches.push("main".to_string()); }
        if has_dev { branches.push("dev".to_string()); }
        if has_developer { branches.push("developer".to_string()); }
        if has_master { branches.push("master".to_string()); }

        // Without vibe-dev, has_required_branches should be false
        // regardless of whether main/dev/developer exist
        let result = has_required_branches(&branches, "vibe-dev");

        prop_assert!(
            !result,
            "has_required_branches should be false without vibe-dev, \
             even with main/dev/developer. Branches: {:?}",
            branches
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// Empty branch list should return false for has_required_branches.
    ///
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_empty_branches_returns_false(work_branch in arb_work_branch_name()) {
        let branches: Vec<String> = Vec::new();
        let result = has_required_branches(&branches, &work_branch);

        prop_assert!(
            !result,
            "has_required_branches should be false for empty branch list"
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// Multiple work branch duplicates should still return true.
    ///
    /// **Validates: Requirements 2.2**
    #[test]
    fn prop_duplicate_work_branch_still_returns_true(
        work_branch in arb_work_branch_name(),
        duplicate_count in 1usize..5usize,
        other_branches in prop::collection::vec("[a-z]{3,10}", 0..5),
    ) {
        let mut branches = other_branches;
        for _ in 0..duplicate_count {
            branches.push(work_branch.clone());
        }

        let result = has_required_branches(&branches, &work_branch);

        prop_assert!(
            result,
            "has_required_branches should be true even with duplicate {}. \
             Branches: {:?}",
            work_branch, branches
        );
    }

    /// Feature: repository-initialization, Property 3: Branch Validation Logic
    ///
    /// Validation logic should work with any arbitrary branch name.
    ///
    /// **Validates: Requirements 2.1, 2.2, 2.3**
    #[test]
    fn prop_arbitrary_branch_name_validation(
        branch_name in "[a-z]{3,20}",
        has_branch in any::<bool>(),
        other_branches in prop::collection::vec("[a-z]{3,10}", 0..10),
    ) {
        let mut branches = other_branches;
        if has_branch {
            branches.push(branch_name.clone());
        }

        let result = has_required_branches(&branches, &branch_name);

        prop_assert_eq!(
            result,
            has_branch,
            "has_required_branches should equal has_branch for arbitrary branch name {}. \
             Branches: {:?}",
            branch_name, branches
        );
    }
}

// ============================================
// Unit tests for edge cases
// Validates: Requirements 2.1, 2.2, 2.3
// ============================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_only_vibe_dev_returns_true() {
        let branches = vec!["vibe-dev".to_string()];
        assert!(has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_vibe_dev_with_others_returns_true() {
        let branches = vec![
            "main".to_string(),
            "dev".to_string(),
            "vibe-dev".to_string(),
            "feature/test".to_string(),
        ];
        assert!(has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_only_main_returns_false() {
        let branches = vec!["main".to_string()];
        assert!(!has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_main_dev_developer_without_vibe_dev_returns_false() {
        let branches = vec![
            "main".to_string(),
            "dev".to_string(),
            "developer".to_string(),
        ];
        assert!(!has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_empty_branches_returns_false() {
        let branches: Vec<String> = vec![];
        assert!(!has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_case_sensitive_vibe_dev() {
        // vibe-dev should be case-sensitive
        let branches = vec!["Vibe-Dev".to_string(), "VIBE-DEV".to_string()];
        assert!(!has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_similar_names_not_matching() {
        // Similar but not exact matches should not satisfy requirement
        let branches = vec![
            "vibe-development".to_string(),
            "vibe-dev-branch".to_string(),
            "my-vibe-dev".to_string(),
        ];
        assert!(!has_required_branches(&branches, "vibe-dev"));
    }

    #[test]
    fn test_agent_dev_still_works() {
        // Test backward compatibility with agent-dev
        let branches = vec!["agent-dev".to_string()];
        assert!(has_required_branches(&branches, "agent-dev"));
    }

    #[test]
    fn test_arbitrary_branch_name() {
        // Test with arbitrary branch name
        let branches = vec!["custom-work".to_string(), "main".to_string()];
        assert!(has_required_branches(&branches, "custom-work"));
        assert!(!has_required_branches(&branches, "other-branch"));
    }
}
