//! Permission system for ACP agent operations
//!
//! This module implements a policy-based permission system that evaluates
//! agent requests for file operations and command execution. It provides:
//! - Path validation (workspace boundary enforcement)
//! - Command allowlist/denylist
//! - Permission logging for audit trails
//! - Repository-specific policy support

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Permission decision result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PermissionDecision {
    /// Permission granted
    Allow { reason: String },
    /// Permission denied
    Deny { reason: String },
}

impl PermissionDecision {
    /// Check if permission was allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionDecision::Allow { .. })
    }

    /// Get the reason for the decision
    pub fn reason(&self) -> &str {
        match self {
            PermissionDecision::Allow { reason } => reason,
            PermissionDecision::Deny { reason } => reason,
        }
    }
}

/// Tool kind for permission requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Execute operation
    Execute,
    /// Delete operation
    Delete,
    /// Search operation
    Search,
}

/// Permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// Tool kind
    pub tool_kind: ToolKind,
    /// Target path (optional)
    pub path: Option<PathBuf>,
    /// Command (for execute operations)
    pub command: Option<String>,
    /// Command arguments
    pub args: Option<Vec<String>>,
}

/// Permission log entry for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionLogEntry {
    /// Timestamp of the request
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// The permission request
    pub request: PermissionRequest,
    /// The decision made
    pub decision: PermissionDecision,
    /// Task ID associated with this request
    pub task_id: Option<i32>,
    /// Agent ID associated with this request
    pub agent_id: Option<i32>,
}

impl PermissionLogEntry {
    /// Create a new permission log entry
    pub fn new(
        request: PermissionRequest,
        decision: PermissionDecision,
        task_id: Option<i32>,
        agent_id: Option<i32>,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            request,
            decision,
            task_id,
            agent_id,
        }
    }

    /// Log the entry using tracing
    pub fn log(&self) {
        match &self.decision {
            PermissionDecision::Allow { reason } => {
                info!(
                    task_id = ?self.task_id,
                    agent_id = ?self.agent_id,
                    request = ?self.request,
                    "Permission ALLOWED: {}",
                    reason
                );
            }
            PermissionDecision::Deny { reason } => {
                warn!(
                    task_id = ?self.task_id,
                    agent_id = ?self.agent_id,
                    request = ?self.request,
                    "Permission DENIED: {}",
                    reason
                );
            }
        }
    }
}

/// Permission policy configuration
#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    /// Workspace root directory
    workspace_root: PathBuf,
    /// Allow read operations
    allow_read: bool,
    /// Allow search operations
    allow_search: bool,
    /// Allow write operations within workspace
    allow_workspace_write: bool,
    /// Allow delete operations
    allow_delete: bool,
    /// Command allowlist
    command_allowlist: Vec<String>,
    /// Command denylist
    command_denylist: Vec<String>,
    /// Protected paths that cannot be written (relative to workspace_root)
    protected_paths: Vec<PathBuf>,
}

impl PermissionPolicy {
    /// Create a new permission policy with default settings
    ///
    /// Default policy:
    /// - Allow read operations
    /// - Allow write operations within workspace
    /// - Deny delete operations
    /// - Allow safe commands (git, cargo, npm, etc.)
    /// - Deny dangerous commands (rm -rf, dd, mkfs, etc.)
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            allow_read: true,
            allow_search: true,
            allow_workspace_write: true,
            allow_delete: false,
            command_allowlist: Self::default_command_allowlist(),
            command_denylist: Self::default_command_denylist(),
            protected_paths: Self::default_protected_paths(),
        }
    }

    /// Create a restrictive policy (read-only)
    pub fn restrictive(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            allow_read: true,
            allow_search: true,
            allow_workspace_write: false,
            allow_delete: false,
            command_allowlist: vec![],
            command_denylist: Self::default_command_denylist(),
            protected_paths: Self::default_protected_paths(),
        }
    }

    /// Create a permissive policy (allow most operations within workspace)
    pub fn permissive(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            allow_read: true,
            allow_search: true,
            allow_workspace_write: true,
            allow_delete: true,
            command_allowlist: vec![], // Allow all except denylist
            command_denylist: Self::default_command_denylist(),
            protected_paths: Self::default_protected_paths(),
        }
    }

    /// Default command allowlist (safe development commands)
    fn default_command_allowlist() -> Vec<String> {
        vec![
            // Version control
            "git".to_string(),
            // Rust toolchain
            "cargo".to_string(),
            "rustc".to_string(),
            "rustup".to_string(),
            "rustfmt".to_string(),
            "clippy".to_string(),
            // Node.js ecosystem
            "node".to_string(),
            "npm".to_string(),
            "npx".to_string(),
            "yarn".to_string(),
            "pnpm".to_string(),
            "bun".to_string(),
            // Python
            "python".to_string(),
            "python3".to_string(),
            "pip".to_string(),
            "pip3".to_string(),
            // Build tools
            "make".to_string(),
            "cmake".to_string(),
            // Testing tools
            "pytest".to_string(),
            "jest".to_string(),
            "mocha".to_string(),
            // Code quality tools
            "eslint".to_string(),
            "prettier".to_string(),
            // File operations (safe)
            "ls".to_string(),
            "cat".to_string(),
            "grep".to_string(),
            "find".to_string(),
            "sed".to_string(),
            "awk".to_string(),
            "head".to_string(),
            "tail".to_string(),
            "wc".to_string(),
            "sort".to_string(),
            "uniq".to_string(),
            // Directory operations
            "mkdir".to_string(),
            "cd".to_string(),
            "pwd".to_string(),
            // Text editors
            "vim".to_string(),
            "nano".to_string(),
            "emacs".to_string(),
            // Utilities
            "echo".to_string(),
            "printf".to_string(),
            "date".to_string(),
            "which".to_string(),
            "env".to_string(),
        ]
    }

    /// Default command denylist (dangerous operations)
    fn default_command_denylist() -> Vec<String> {
        vec![
            // Destructive file operations
            "rm".to_string(),
            "rmdir".to_string(),
            "shred".to_string(),
            // Disk operations
            "dd".to_string(),
            "mkfs".to_string(),
            "fdisk".to_string(),
            "parted".to_string(),
            "format".to_string(),
            // System modification
            "chmod".to_string(),
            "chown".to_string(),
            "chgrp".to_string(),
            "mount".to_string(),
            "umount".to_string(),
            "sudo".to_string(),
            "su".to_string(),
            // Network operations
            "curl".to_string(),
            "wget".to_string(),
            "nc".to_string(),
            "netcat".to_string(),
            "telnet".to_string(),
            // Process operations
            "kill".to_string(),
            "killall".to_string(),
            "pkill".to_string(),
            // System control
            "shutdown".to_string(),
            "reboot".to_string(),
            "halt".to_string(),
            "poweroff".to_string(),
            // Package management (system-wide)
            "apt".to_string(),
            "apt-get".to_string(),
            "yum".to_string(),
            "dnf".to_string(),
            "pacman".to_string(),
            "brew".to_string(),
        ]
    }

    /// Default protected paths (relative to workspace root)
    fn default_protected_paths() -> Vec<PathBuf> {
        vec![PathBuf::from(".git/config"), PathBuf::from(".git/HEAD")]
    }

    /// Evaluate a permission request
    pub fn evaluate(&self, request: &PermissionRequest) -> PermissionDecision {
        match request.tool_kind {
            ToolKind::Read => self.evaluate_read(request),
            ToolKind::Write => self.evaluate_write(request),
            ToolKind::Execute => self.evaluate_execute(request),
            ToolKind::Delete => self.evaluate_delete(request),
            ToolKind::Search => self.evaluate_search(request),
        }
    }

    /// Evaluate read permission
    fn evaluate_read(&self, request: &PermissionRequest) -> PermissionDecision {
        if !self.allow_read {
            return PermissionDecision::Deny {
                reason: "Read operations are disabled by policy".to_string(),
            };
        }

        // If path is provided, check if it's within workspace
        if let Some(path) = &request.path {
            if !self.is_within_workspace(path) {
                return PermissionDecision::Deny {
                    reason: format!(
                        "Path '{}' is outside workspace '{}'",
                        path.display(),
                        self.workspace_root.display()
                    ),
                };
            }
        }

        PermissionDecision::Allow {
            reason: "Read operation within workspace".to_string(),
        }
    }

    /// Evaluate write permission
    fn evaluate_write(&self, request: &PermissionRequest) -> PermissionDecision {
        if !self.allow_workspace_write {
            return PermissionDecision::Deny {
                reason: "Write operations are disabled by policy".to_string(),
            };
        }

        // Must have path
        let Some(path) = &request.path else {
            return PermissionDecision::Deny {
                reason: "Write operation missing path".to_string(),
            };
        };

        // Check if within workspace
        if !self.is_within_workspace(path) {
            return PermissionDecision::Deny {
                reason: format!(
                    "Path '{}' is outside workspace '{}'",
                    path.display(),
                    self.workspace_root.display()
                ),
            };
        }

        // Check if path is protected
        if self.is_path_protected(path) {
            return PermissionDecision::Deny {
                reason: format!(
                    "Path '{}' is protected and cannot be modified",
                    path.display()
                ),
            };
        }

        PermissionDecision::Allow {
            reason: "Write operation within workspace to non-protected path".to_string(),
        }
    }

    /// Evaluate execute permission
    fn evaluate_execute(&self, request: &PermissionRequest) -> PermissionDecision {
        // Must have command
        let Some(command) = &request.command else {
            return PermissionDecision::Deny {
                reason: "Execute operation missing command".to_string(),
            };
        };

        // Check denylist first (takes precedence)
        if self.is_command_denied(command) {
            return PermissionDecision::Deny {
                reason: format!("Command '{}' is in denylist", command),
            };
        }

        // Check for dangerous command patterns
        if let Some(args) = &request.args {
            if let Some(reason) = self.check_dangerous_patterns(command, args) {
                return PermissionDecision::Deny { reason };
            }
        }

        // If allowlist is not empty, check if command is in it
        if !self.command_allowlist.is_empty() && !self.is_command_allowed(command) {
            return PermissionDecision::Deny {
                reason: format!("Command '{}' is not in allowlist", command),
            };
        }

        PermissionDecision::Allow {
            reason: format!("Command '{}' is allowed", command),
        }
    }

    /// Evaluate delete permission
    fn evaluate_delete(&self, request: &PermissionRequest) -> PermissionDecision {
        if !self.allow_delete {
            return PermissionDecision::Deny {
                reason: "Delete operations are disabled by policy".to_string(),
            };
        }

        // Check if path is within workspace
        if let Some(path) = &request.path {
            if !self.is_within_workspace(path) {
                return PermissionDecision::Deny {
                    reason: format!(
                        "Path '{}' is outside workspace '{}'",
                        path.display(),
                        self.workspace_root.display()
                    ),
                };
            }

            // Check if path is protected
            if self.is_path_protected(path) {
                return PermissionDecision::Deny {
                    reason: format!(
                        "Path '{}' is protected and cannot be deleted",
                        path.display()
                    ),
                };
            }
        }

        PermissionDecision::Allow {
            reason: "Delete operation within workspace to non-protected path".to_string(),
        }
    }

    /// Evaluate search permission
    fn evaluate_search(&self, _request: &PermissionRequest) -> PermissionDecision {
        if !self.allow_search {
            return PermissionDecision::Deny {
                reason: "Search operations are disabled by policy".to_string(),
            };
        }

        PermissionDecision::Allow {
            reason: "Search operation allowed".to_string(),
        }
    }

    /// Check if path is within workspace boundaries
    ///
    /// This function canonicalizes both paths and checks if the target path
    /// starts with the workspace root, preventing path traversal attacks.
    fn is_within_workspace(&self, path: &Path) -> bool {
        // Convert to absolute path if relative
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Try to canonicalize paths (resolve symlinks, .., etc.)
        match (abs_path.canonicalize(), self.workspace_root.canonicalize()) {
            (Ok(canonical_path), Ok(canonical_root)) => canonical_path.starts_with(&canonical_root),
            _ => {
                // Fallback: manually resolve .. components to detect traversal
                // This handles non-existent paths
                let normalized = self.normalize_path(&abs_path);
                let normalized_root = self.normalize_path(&self.workspace_root);
                normalized.starts_with(&normalized_root)
            }
        }
    }

    /// Manually normalize a path by resolving .. components
    /// This is a security-critical function to prevent path traversal
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    // Pop the last component if possible
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                _ => {
                    components.push(component);
                }
            }
        }

        components.iter().collect()
    }

    /// Check if path is protected
    fn is_path_protected(&self, path: &Path) -> bool {
        // Convert to relative path from workspace root
        let rel_path = if path.is_absolute() {
            path.strip_prefix(&self.workspace_root).ok()
        } else {
            Some(path)
        };

        if let Some(rel_path) = rel_path {
            for protected in &self.protected_paths {
                if rel_path.starts_with(protected) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if command is in allowlist
    fn is_command_allowed(&self, command: &str) -> bool {
        // Extract command name (remove path)
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command);

        self.command_allowlist
            .iter()
            .any(|allowed| allowed == cmd_name)
    }

    /// Check if command is in denylist
    fn is_command_denied(&self, command: &str) -> bool {
        // Extract command name (remove path)
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(command);

        self.command_denylist
            .iter()
            .any(|denied| denied == cmd_name)
    }

    /// Check for dangerous command patterns
    fn check_dangerous_patterns(&self, command: &str, args: &[String]) -> Option<String> {
        // Check for rm with dangerous flags
        if command.contains("rm") {
            for arg in args {
                if arg.contains('r') || arg.contains('f') || arg == "-rf" || arg == "-fr" {
                    return Some("Recursive or force delete is not allowed".to_string());
                }
            }
        }

        // Check for chmod with dangerous permissions
        if command.contains("chmod") {
            for arg in args {
                if arg.starts_with("777") || arg.starts_with("+x") {
                    return Some("Dangerous chmod permissions detected".to_string());
                }
            }
        }

        // Check for paths outside workspace
        for arg in args {
            if arg.starts_with('/') || arg.starts_with("..") {
                if let Ok(path) = PathBuf::from(arg).canonicalize() {
                    if !self.is_within_workspace(&path) {
                        return Some(format!(
                            "Argument '{}' references path outside workspace",
                            arg
                        ));
                    }
                }
            }
        }

        None
    }

    /// Set whether to allow read operations
    pub fn set_allow_read(&mut self, allow: bool) {
        self.allow_read = allow;
    }

    /// Set whether to allow workspace write operations
    pub fn set_allow_workspace_write(&mut self, allow: bool) {
        self.allow_workspace_write = allow;
    }

    /// Set whether to allow delete operations
    pub fn set_allow_delete(&mut self, allow: bool) {
        self.allow_delete = allow;
    }

    /// Add command to allowlist
    pub fn add_allowed_command(&mut self, command: String) {
        if !self.command_allowlist.contains(&command) {
            self.command_allowlist.push(command);
        }
    }

    /// Add command to denylist
    pub fn add_denied_command(&mut self, command: String) {
        if !self.command_denylist.contains(&command) {
            self.command_denylist.push(command);
        }
    }

    /// Add protected path
    pub fn add_protected_path(&mut self, path: PathBuf) {
        if !self.protected_paths.contains(&path) {
            self.protected_paths.push(path);
        }
    }

    /// Get workspace root
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().to_path_buf();
        (temp_dir, workspace_root)
    }

    #[test]
    fn test_permission_decision() {
        let allow = PermissionDecision::Allow {
            reason: "test".to_string(),
        };
        let deny = PermissionDecision::Deny {
            reason: "test".to_string(),
        };

        assert!(allow.is_allowed());
        assert!(!deny.is_allowed());
        assert_eq!(allow.reason(), "test");
        assert_eq!(deny.reason(), "test");
    }

    #[test]
    fn test_tool_kind() {
        assert_eq!(ToolKind::Read, ToolKind::Read);
        assert_ne!(ToolKind::Read, ToolKind::Write);
    }

    #[test]
    fn test_permission_policy_default() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);
        assert!(policy.allow_read);
        assert!(policy.allow_search);
        assert!(policy.allow_workspace_write);
        assert!(!policy.allow_delete);
    }

    #[test]
    fn test_evaluate_read_allowed() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: Some(workspace_root.join("test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_evaluate_read_outside_workspace() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: Some(PathBuf::from("/etc/passwd")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("outside workspace"));
    }

    #[test]
    fn test_evaluate_write_in_workspace() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        let request = PermissionRequest {
            tool_kind: ToolKind::Write,
            path: Some(workspace_root.join("test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_evaluate_write_outside_workspace() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Write,
            path: Some(PathBuf::from("/etc/test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("outside workspace"));
    }

    #[test]
    fn test_evaluate_write_protected_path() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        // Create .git directory
        fs::create_dir(workspace_root.join(".git")).unwrap();

        let request = PermissionRequest {
            tool_kind: ToolKind::Write,
            path: Some(workspace_root.join(".git/config")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("protected"));
    }

    #[test]
    fn test_evaluate_execute_allowed_command() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Execute,
            path: None,
            command: Some("git".to_string()),
            args: Some(vec!["status".to_string()]),
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_evaluate_execute_denied_command() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Execute,
            path: None,
            command: Some("rm".to_string()),
            args: Some(vec!["-rf".to_string(), "/".to_string()]),
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("denylist"));
    }

    #[test]
    fn test_evaluate_execute_not_in_allowlist() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Execute,
            path: None,
            command: Some("unknown-command".to_string()),
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("not in allowlist"));
    }

    #[test]
    fn test_evaluate_delete_denied_by_default() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Delete,
            path: Some(PathBuf::from("test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("Delete operations are disabled"));
    }

    #[test]
    fn test_evaluate_search_allowed() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Search,
            path: None,
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_command_allowlist() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        assert!(policy.is_command_allowed("git"));
        assert!(policy.is_command_allowed("cargo"));
        assert!(policy.is_command_allowed("npm"));
        assert!(!policy.is_command_allowed("unknown"));
    }

    #[test]
    fn test_command_denylist() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        assert!(policy.is_command_denied("rm"));
        assert!(policy.is_command_denied("sudo"));
        assert!(policy.is_command_denied("dd"));
        assert!(!policy.is_command_denied("git"));
    }

    #[test]
    fn test_dangerous_patterns_detection() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        // rm -rf is dangerous
        let result =
            policy.check_dangerous_patterns("rm", &vec!["-rf".to_string(), "/".to_string()]);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Recursive or force delete"));

        // git commands are usually safe
        let result = policy.check_dangerous_patterns("git", &vec!["status".to_string()]);
        assert!(result.is_none());
    }

    #[test]
    fn test_add_allowed_command() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root);

        assert!(!policy.is_command_allowed("custom-tool"));

        policy.add_allowed_command("custom-tool".to_string());
        assert!(policy.is_command_allowed("custom-tool"));
    }

    #[test]
    fn test_add_denied_command() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root);

        assert!(!policy.is_command_denied("custom-dangerous"));

        policy.add_denied_command("custom-dangerous".to_string());
        assert!(policy.is_command_denied("custom-dangerous"));
    }

    #[test]
    fn test_set_allow_read() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root);
        assert!(policy.allow_read);

        policy.set_allow_read(false);
        assert!(!policy.allow_read);

        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: None,
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
    }

    #[test]
    fn test_set_allow_workspace_write() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root);
        assert!(policy.allow_workspace_write);

        policy.set_allow_workspace_write(false);
        assert!(!policy.allow_workspace_write);
    }

    #[test]
    fn test_set_allow_delete() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root);
        assert!(!policy.allow_delete);

        policy.set_allow_delete(true);
        assert!(policy.allow_delete);
    }

    #[test]
    fn test_restrictive_policy() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::restrictive(workspace_root.clone());

        let write_request = PermissionRequest {
            tool_kind: ToolKind::Write,
            path: Some(workspace_root.join("test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&write_request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("Write operations are disabled"));
    }

    #[test]
    fn test_permissive_policy() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::permissive(workspace_root.clone());

        let delete_request = PermissionRequest {
            tool_kind: ToolKind::Delete,
            path: Some(workspace_root.join("test.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&delete_request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_permission_log_entry() {
        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: Some(PathBuf::from("/test/file.txt")),
            command: None,
            args: None,
        };
        let decision = PermissionDecision::Allow {
            reason: "Test reason".to_string(),
        };

        let log_entry = PermissionLogEntry::new(request, decision, Some(1), Some(2));

        assert_eq!(log_entry.task_id, Some(1));
        assert_eq!(log_entry.agent_id, Some(2));
        assert!(log_entry.decision.is_allowed());
    }

    #[test]
    fn test_relative_path_within_workspace() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: Some(PathBuf::from("src/main.rs")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_command_with_path_extracted() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root);

        let request = PermissionRequest {
            tool_kind: ToolKind::Execute,
            path: None,
            command: Some("/usr/bin/git".to_string()),
            args: Some(vec!["status".to_string()]),
        };

        let decision = policy.evaluate(&request);
        assert!(decision.is_allowed());
    }

    #[test]
    fn test_path_traversal_attack_prevented() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        // Try to escape workspace using relative path
        let request = PermissionRequest {
            tool_kind: ToolKind::Read,
            path: Some(PathBuf::from("../../../etc/passwd")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        // This should be denied because the normalized path will be outside workspace
        assert!(!decision.is_allowed(), "Path traversal should be denied");
    }

    #[test]
    fn test_add_protected_path() {
        let (_temp, workspace_root) = create_test_workspace();
        let mut policy = PermissionPolicy::new(workspace_root.clone());

        policy.add_protected_path(PathBuf::from("secret.txt"));

        let request = PermissionRequest {
            tool_kind: ToolKind::Write,
            path: Some(workspace_root.join("secret.txt")),
            command: None,
            args: None,
        };

        let decision = policy.evaluate(&request);
        assert!(!decision.is_allowed());
        assert!(decision.reason().contains("protected"));
    }

    #[test]
    fn test_workspace_root_getter() {
        let (_temp, workspace_root) = create_test_workspace();
        let policy = PermissionPolicy::new(workspace_root.clone());

        assert_eq!(policy.workspace_root(), workspace_root.as_path());
    }
}
