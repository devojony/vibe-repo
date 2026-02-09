//! Event processing and storage for ACP agent sessions
//!
//! This module handles parsing, extraction, and storage of events from ACP sessionUpdate
//! notifications. It provides event compaction, progress calculation, and query methods.

use agent_client_protocol as acp;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Maximum number of events to keep in storage (for compaction)
const MAX_EVENTS: usize = 100;

/// Agent event types that can be extracted from ACP sessionUpdate notifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Agent execution plan
    Plan(PlanEvent),
    /// Tool call execution
    ToolCall(ToolCallEvent),
    /// Agent message
    Message(MessageEvent),
    /// Task completion
    Completed(CompletedEvent),
}

impl AgentEvent {
    /// Get the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            AgentEvent::Plan(e) => e.timestamp,
            AgentEvent::ToolCall(e) => e.timestamp,
            AgentEvent::Message(e) => e.timestamp,
            AgentEvent::Completed(e) => e.timestamp,
        }
    }

    /// Get the event type as a string
    pub fn event_type(&self) -> &str {
        match self {
            AgentEvent::Plan(_) => "plan",
            AgentEvent::ToolCall(_) => "tool_call",
            AgentEvent::Message(_) => "message",
            AgentEvent::Completed(_) => "completed",
        }
    }
}

/// Plan event representing agent's execution plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanEvent {
    /// Plan steps/entries
    pub steps: Vec<PlanStep>,
    /// Current step index (0-based)
    pub current_step: Option<usize>,
    /// Overall plan status
    pub status: PlanStatus,
    /// Timestamp when plan was created/updated
    pub timestamp: DateTime<Utc>,
}

/// Individual step in a plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanStep {
    /// Step description
    pub description: String,
    /// Step status
    pub status: StepStatus,
    /// Step index
    pub index: usize,
}

/// Status of a plan
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    /// Plan is being created
    Creating,
    /// Plan is active and being executed
    Active,
    /// Plan is completed
    Completed,
    /// Plan was modified
    Modified,
}

/// Status of an individual step
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step is pending
    Pending,
    /// Step is in progress
    InProgress,
    /// Step is completed
    Completed,
    /// Step was skipped
    Skipped,
}

/// Tool call event representing a tool execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallEvent {
    /// Tool name (e.g., "read_file", "write_file", "bash")
    pub tool_name: String,
    /// Tool call title/description
    pub title: String,
    /// Tool arguments (serialized as JSON)
    pub args: serde_json::Value,
    /// Tool execution result (if available)
    pub result: Option<String>,
    /// Tool call status
    pub status: ToolCallStatus,
    /// Timestamp when tool was called
    pub timestamp: DateTime<Utc>,
}

/// Status of a tool call
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// Tool call is starting
    Started,
    /// Tool call is in progress
    InProgress,
    /// Tool call completed successfully
    Completed,
    /// Tool call failed
    Failed,
}

/// Message event representing agent communication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageEvent {
    /// Message content
    pub content: String,
    /// Message role (agent, user, system)
    pub role: String,
    /// Timestamp when message was sent
    pub timestamp: DateTime<Utc>,
}

/// Completed event representing task completion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletedEvent {
    /// Whether the task completed successfully
    pub success: bool,
    /// Completion reason/message
    pub reason: Option<String>,
    /// Timestamp when task completed
    pub timestamp: DateTime<Utc>,
}

/// Parse a sessionUpdate notification into agent events
pub fn parse_session_update(update: &acp::SessionUpdate) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    match update {
        acp::SessionUpdate::Plan(plan) => {
            if let Some(event) = extract_plan(plan) {
                events.push(AgentEvent::Plan(event));
            }
        }
        acp::SessionUpdate::ToolCall(tool_call) => {
            if let Some(event) = extract_tool_call(tool_call) {
                events.push(AgentEvent::ToolCall(event));
            }
        }
        acp::SessionUpdate::ToolCallUpdate(update) => {
            if let Some(event) = extract_tool_call_update(update) {
                events.push(AgentEvent::ToolCall(event));
            }
        }
        acp::SessionUpdate::UserMessageChunk(chunk) => {
            // Extract user message from the chunk
            let content = extract_content_block(&chunk.content);
            if !content.is_empty() {
                events.push(AgentEvent::Message(MessageEvent {
                    content,
                    role: "user".to_string(),
                    timestamp: Utc::now(),
                }));
            }
        }
        acp::SessionUpdate::AgentMessageChunk(chunk) => {
            // Extract agent message from the chunk
            let content = extract_content_block(&chunk.content);
            if !content.is_empty() {
                events.push(AgentEvent::Message(MessageEvent {
                    content,
                    role: "agent".to_string(),
                    timestamp: Utc::now(),
                }));
            }
        }
        acp::SessionUpdate::AgentThoughtChunk(chunk) => {
            // Extract agent thought from the chunk
            let content = extract_content_block(&chunk.content);
            if !content.is_empty() {
                events.push(AgentEvent::Message(MessageEvent {
                    content,
                    role: "thought".to_string(),
                    timestamp: Utc::now(),
                }));
            }
        }
        acp::SessionUpdate::AvailableCommandsUpdate(_) => {
            // Available commands updates don't need to be stored as events
            // They are logged in session_notification for debugging
        }
        acp::SessionUpdate::CurrentModeUpdate(_) => {
            // Mode updates don't need to be stored as events
            // They are logged in session_notification for debugging
        }
        acp::SessionUpdate::ConfigOptionUpdate(_) => {
            // Config option updates don't need to be stored as events
            // They are logged in session_notification for debugging
        }
        _ => {
            // Other update types (e.g., unstable features) are not currently processed
        }
    }

    events
}

/// Extract plan event from ACP plan update
fn extract_plan(plan: &acp::Plan) -> Option<PlanEvent> {
    let steps: Vec<PlanStep> = plan
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            // In SDK v0.9, PlanEntry.content is a String (not Vec<ContentBlock>)
            // This is the "Human-readable description" field from the protocol
            let description = entry.content.clone();

            PlanStep {
                description,
                status: match entry.status {
                    acp::PlanEntryStatus::Pending => StepStatus::Pending,
                    acp::PlanEntryStatus::InProgress => StepStatus::InProgress,
                    acp::PlanEntryStatus::Completed => StepStatus::Completed,
                    _ => StepStatus::Pending, // Default to pending for unknown statuses
                },
                index,
            }
        })
        .collect();

    // Find current step (first in-progress or pending step)
    let current_step = steps
        .iter()
        .position(|s| matches!(s.status, StepStatus::InProgress | StepStatus::Pending));

    // Determine overall status
    let status = if steps.iter().all(|s| s.status == StepStatus::Completed) {
        PlanStatus::Completed
    } else if steps.iter().any(|s| s.status == StepStatus::InProgress) {
        PlanStatus::Active
    } else {
        PlanStatus::Creating
    };

    Some(PlanEvent {
        steps,
        current_step,
        status,
        timestamp: Utc::now(),
    })
}

/// Extract tool call event from ACP tool call
fn extract_tool_call(tool_call: &acp::ToolCall) -> Option<ToolCallEvent> {
    // Extract tool name from kind
    let tool_name = format!("{:?}", tool_call.kind);

    // Extract content as result from ToolCallContent
    let result = if !tool_call.content.is_empty() {
        Some(extract_tool_call_content(&tool_call.content))
    } else {
        None
    };

    // Serialize raw input as args
    let args = tool_call
        .raw_input
        .clone()
        .unwrap_or(serde_json::Value::Null);

    Some(ToolCallEvent {
        tool_name,
        title: tool_call.title.clone(),
        args,
        result,
        status: match tool_call.status {
            acp::ToolCallStatus::Pending => ToolCallStatus::Started,
            acp::ToolCallStatus::InProgress => ToolCallStatus::InProgress,
            acp::ToolCallStatus::Completed => ToolCallStatus::Completed,
            acp::ToolCallStatus::Failed => ToolCallStatus::Failed,
            _ => ToolCallStatus::Started,
        },
        timestamp: Utc::now(),
    })
}

/// Extract tool call event from ACP tool call update
fn extract_tool_call_update(update: &acp::ToolCallUpdate) -> Option<ToolCallEvent> {
    // Extract tool name from kind if available
    let tool_name = if let Some(kind) = &update.fields.kind {
        format!("{:?}", kind)
    } else {
        "unknown".to_string()
    };

    // Extract content as result - fields.content is Option<Vec<ToolCallContent>>
    let result = update.fields.content.as_ref().map(|content_vec| extract_tool_call_content(content_vec));

    // Serialize raw input as args
    let args = update
        .fields
        .raw_input
        .clone()
        .unwrap_or(serde_json::Value::Null);

    Some(ToolCallEvent {
        tool_name,
        title: update.fields.title.clone().unwrap_or_default(),
        args,
        result,
        status: match &update.fields.status {
            Some(acp::ToolCallStatus::Pending) => ToolCallStatus::Started,
            Some(acp::ToolCallStatus::InProgress) => ToolCallStatus::InProgress,
            Some(acp::ToolCallStatus::Completed) => ToolCallStatus::Completed,
            Some(acp::ToolCallStatus::Failed) => ToolCallStatus::Failed,
            Some(_) => ToolCallStatus::InProgress, // Default for unknown statuses
            None => ToolCallStatus::InProgress,
        },
        timestamp: Utc::now(),
    })
}

/// Extract text from a content block
fn extract_content_block(block: &acp::ContentBlock) -> String {
    match block {
        acp::ContentBlock::Text(text_content) => text_content.text.clone(),
        acp::ContentBlock::Image(_) => "<image>".to_string(),
        acp::ContentBlock::Audio(_) => "<audio>".to_string(),
        acp::ContentBlock::ResourceLink(resource_link) => {
            format!("<resource: {}>", resource_link.uri)
        }
        acp::ContentBlock::Resource(_) => "<resource>".to_string(),
        _ => String::new(),
    }
}

/// Extract text from tool call content
fn extract_tool_call_content(content: &[acp::ToolCallContent]) -> String {
    content
        .iter()
        .map(extract_tool_call_content_single)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract text from a single tool call content item
fn extract_tool_call_content_single(content: &acp::ToolCallContent) -> String {
    match content {
        acp::ToolCallContent::Content(c) => extract_content_block(&c.content),
        acp::ToolCallContent::Diff(d) => {
            format!("Diff: {} ({} bytes)", d.path.display(), d.new_text.len())
        }
        acp::ToolCallContent::Terminal(t) => {
            format!("Terminal: {}", t.terminal_id)
        }
        _ => String::new(),
    }
}

/// Compact events by keeping only the most recent MAX_EVENTS
pub fn compact_events(events: Vec<AgentEvent>) -> Vec<AgentEvent> {
    if events.len() <= MAX_EVENTS {
        return events;
    }

    // Keep the most recent MAX_EVENTS
    events.into_iter().rev().take(MAX_EVENTS).rev().collect()
}

/// Calculate progress percentage from plans (0.0 to 1.0)
pub fn calculate_progress(plans: &[PlanEvent]) -> f32 {
    if plans.is_empty() {
        return 0.0;
    }

    // Use the most recent plan
    let latest_plan = &plans[plans.len() - 1];

    if latest_plan.steps.is_empty() {
        return 0.0;
    }

    let completed_steps = latest_plan
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Completed)
        .count();

    let total_steps = latest_plan.steps.len();

    completed_steps as f32 / total_steps as f32
}

/// Filter events by type
pub fn filter_events_by_type(events: &[AgentEvent], event_type: &str) -> Vec<AgentEvent> {
    events
        .iter()
        .filter(|e| e.event_type() == event_type)
        .cloned()
        .collect()
}

/// Get the latest message from events
pub fn get_latest_message(events: &[AgentEvent]) -> Option<MessageEvent> {
    events.iter().rev().find_map(|e| {
        if let AgentEvent::Message(msg) = e {
            Some(msg.clone())
        } else {
            None
        }
    })
}

/// Extract all plans from events
pub fn extract_plans(events: &[AgentEvent]) -> Vec<PlanEvent> {
    events
        .iter()
        .filter_map(|e| {
            if let AgentEvent::Plan(plan) = e {
                Some(plan.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Event storage helper for managing events in a task
#[derive(Debug, Clone)]
pub struct EventStore {
    events: VecDeque<AgentEvent>,
    max_events: usize,
}

impl EventStore {
    /// Create a new event store
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            max_events: MAX_EVENTS,
        }
    }

    /// Create an event store with custom max events
    pub fn with_capacity(max_events: usize) -> Self {
        Self {
            events: VecDeque::new(),
            max_events,
        }
    }

    /// Add an event to the store
    pub fn add_event(&mut self, event: AgentEvent) {
        self.events.push_back(event);

        // Compact if needed
        while self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    /// Add multiple events to the store
    pub fn add_events(&mut self, events: Vec<AgentEvent>) {
        for event in events {
            self.add_event(event);
        }
    }

    /// Get all events
    pub fn get_events(&self) -> Vec<AgentEvent> {
        self.events.iter().cloned().collect()
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<AgentEvent> {
        filter_events_by_type(&self.get_events(), event_type)
    }

    /// Get all plans
    pub fn get_plans(&self) -> Vec<PlanEvent> {
        extract_plans(&self.get_events())
    }

    /// Get the latest message
    pub fn get_latest_message(&self) -> Option<MessageEvent> {
        get_latest_message(&self.get_events())
    }

    /// Calculate progress from stored plans
    pub fn calculate_progress(&self) -> f32 {
        calculate_progress(&self.get_plans())
    }

    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_event_timestamp() {
        let now = Utc::now();
        let plan_event = AgentEvent::Plan(PlanEvent {
            steps: vec![],
            current_step: None,
            status: PlanStatus::Creating,
            timestamp: now,
        });

        assert_eq!(plan_event.timestamp(), now);
    }

    #[test]
    fn test_agent_event_type() {
        let plan_event = AgentEvent::Plan(PlanEvent {
            steps: vec![],
            current_step: None,
            status: PlanStatus::Creating,
            timestamp: Utc::now(),
        });

        assert_eq!(plan_event.event_type(), "plan");
    }

    // Note: We cannot test extract_plan, extract_tool_call, etc. directly
    // because the ACP SDK structs are non-exhaustive and cannot be constructed
    // in tests. These functions will be tested through integration tests.

    #[test]
    fn test_compact_events() {
        let mut events = Vec::new();
        for i in 0..150 {
            events.push(AgentEvent::Message(MessageEvent {
                content: format!("Message {}", i),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }));
        }

        let compacted = compact_events(events);

        assert_eq!(compacted.len(), MAX_EVENTS);
        // Should keep the most recent events (50-149)
        if let AgentEvent::Message(msg) = &compacted[0] {
            assert_eq!(msg.content, "Message 50");
        }
    }

    #[test]
    fn test_compact_events_no_change() {
        let events = vec![
            AgentEvent::Message(MessageEvent {
                content: "Message 1".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
            AgentEvent::Message(MessageEvent {
                content: "Message 2".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
        ];

        let compacted = compact_events(events.clone());

        assert_eq!(compacted.len(), 2);
        assert_eq!(compacted, events);
    }

    #[test]
    fn test_calculate_progress_empty() {
        let plans = vec![];
        assert_eq!(calculate_progress(&plans), 0.0);
    }

    #[test]
    fn test_calculate_progress_no_steps() {
        let plans = vec![PlanEvent {
            steps: vec![],
            current_step: None,
            status: PlanStatus::Creating,
            timestamp: Utc::now(),
        }];

        assert_eq!(calculate_progress(&plans), 0.0);
    }

    #[test]
    fn test_calculate_progress_partial() {
        let plans = vec![PlanEvent {
            steps: vec![
                PlanStep {
                    description: "Step 1".to_string(),
                    status: StepStatus::Completed,
                    index: 0,
                },
                PlanStep {
                    description: "Step 2".to_string(),
                    status: StepStatus::InProgress,
                    index: 1,
                },
                PlanStep {
                    description: "Step 3".to_string(),
                    status: StepStatus::Pending,
                    index: 2,
                },
                PlanStep {
                    description: "Step 4".to_string(),
                    status: StepStatus::Pending,
                    index: 3,
                },
            ],
            current_step: Some(1),
            status: PlanStatus::Active,
            timestamp: Utc::now(),
        }];

        assert_eq!(calculate_progress(&plans), 0.25); // 1 out of 4 completed
    }

    #[test]
    fn test_calculate_progress_complete() {
        let plans = vec![PlanEvent {
            steps: vec![
                PlanStep {
                    description: "Step 1".to_string(),
                    status: StepStatus::Completed,
                    index: 0,
                },
                PlanStep {
                    description: "Step 2".to_string(),
                    status: StepStatus::Completed,
                    index: 1,
                },
            ],
            current_step: None,
            status: PlanStatus::Completed,
            timestamp: Utc::now(),
        }];

        assert_eq!(calculate_progress(&plans), 1.0);
    }

    #[test]
    fn test_filter_events_by_type() {
        let events = vec![
            AgentEvent::Plan(PlanEvent {
                steps: vec![],
                current_step: None,
                status: PlanStatus::Creating,
                timestamp: Utc::now(),
            }),
            AgentEvent::Message(MessageEvent {
                content: "Hello".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
            AgentEvent::Message(MessageEvent {
                content: "World".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
        ];

        let messages = filter_events_by_type(&events, "message");
        assert_eq!(messages.len(), 2);

        let plans = filter_events_by_type(&events, "plan");
        assert_eq!(plans.len(), 1);
    }

    #[test]
    fn test_get_latest_message() {
        let events = vec![
            AgentEvent::Message(MessageEvent {
                content: "First".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
            AgentEvent::Plan(PlanEvent {
                steps: vec![],
                current_step: None,
                status: PlanStatus::Creating,
                timestamp: Utc::now(),
            }),
            AgentEvent::Message(MessageEvent {
                content: "Last".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
        ];

        let latest = get_latest_message(&events).unwrap();
        assert_eq!(latest.content, "Last");
    }

    #[test]
    fn test_extract_plans() {
        let events = vec![
            AgentEvent::Plan(PlanEvent {
                steps: vec![],
                current_step: None,
                status: PlanStatus::Creating,
                timestamp: Utc::now(),
            }),
            AgentEvent::Message(MessageEvent {
                content: "Hello".to_string(),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }),
            AgentEvent::Plan(PlanEvent {
                steps: vec![],
                current_step: None,
                status: PlanStatus::Active,
                timestamp: Utc::now(),
            }),
        ];

        let plans = extract_plans(&events);
        assert_eq!(plans.len(), 2);
    }

    #[test]
    fn test_event_store_new() {
        let store = EventStore::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn test_event_store_add_event() {
        let mut store = EventStore::new();
        let event = AgentEvent::Message(MessageEvent {
            content: "Test".to_string(),
            role: "agent".to_string(),
            timestamp: Utc::now(),
        });

        store.add_event(event);
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
    }

    #[test]
    fn test_event_store_compaction() {
        let mut store = EventStore::with_capacity(10);

        for i in 0..20 {
            store.add_event(AgentEvent::Message(MessageEvent {
                content: format!("Message {}", i),
                role: "agent".to_string(),
                timestamp: Utc::now(),
            }));
        }

        assert_eq!(store.len(), 10);
        // Should keep the most recent 10 events (10-19)
        let events = store.get_events();
        if let AgentEvent::Message(msg) = &events[0] {
            assert_eq!(msg.content, "Message 10");
        }
    }

    #[test]
    fn test_event_store_get_events_by_type() {
        let mut store = EventStore::new();

        store.add_event(AgentEvent::Plan(PlanEvent {
            steps: vec![],
            current_step: None,
            status: PlanStatus::Creating,
            timestamp: Utc::now(),
        }));

        store.add_event(AgentEvent::Message(MessageEvent {
            content: "Hello".to_string(),
            role: "agent".to_string(),
            timestamp: Utc::now(),
        }));

        let messages = store.get_events_by_type("message");
        assert_eq!(messages.len(), 1);

        let plans = store.get_events_by_type("plan");
        assert_eq!(plans.len(), 1);
    }

    #[test]
    fn test_event_store_calculate_progress() {
        let mut store = EventStore::new();

        store.add_event(AgentEvent::Plan(PlanEvent {
            steps: vec![
                PlanStep {
                    description: "Step 1".to_string(),
                    status: StepStatus::Completed,
                    index: 0,
                },
                PlanStep {
                    description: "Step 2".to_string(),
                    status: StepStatus::Pending,
                    index: 1,
                },
            ],
            current_step: Some(1),
            status: PlanStatus::Active,
            timestamp: Utc::now(),
        }));

        assert_eq!(store.calculate_progress(), 0.5);
    }

    #[test]
    fn test_event_store_clear() {
        let mut store = EventStore::new();

        store.add_event(AgentEvent::Message(MessageEvent {
            content: "Test".to_string(),
            role: "agent".to_string(),
            timestamp: Utc::now(),
        }));

        assert_eq!(store.len(), 1);

        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }
}
