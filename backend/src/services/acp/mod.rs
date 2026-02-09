//! ACP (Agent Client Protocol) integration module
//!
//! This module provides a wrapper around the official `agent-client-protocol` Rust SDK
//! to simplify integration with VibeRepo's task execution system.

mod client;
mod error;
mod events;
mod permissions;

pub use client::{AcpClient, AgentConfig as AcpAgentConfig};
pub use error::{AcpError, AcpResult};
pub use events::{
    calculate_progress, compact_events, extract_plans, filter_events_by_type, get_latest_message,
    parse_session_update, AgentEvent, CompletedEvent, EventStore, MessageEvent, PlanEvent,
    PlanStatus, PlanStep, StepStatus, ToolCallEvent, ToolCallStatus,
};
pub use permissions::{
    PermissionDecision, PermissionLogEntry, PermissionPolicy, PermissionRequest, ToolKind,
};

// Re-export MCP types from config module for convenience
pub use crate::config::mcp::{McpConfigLoader, McpEnvVar, McpServerConfig, McpServersConfig};
