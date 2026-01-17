use vibe_repo::git_provider::models::*;

#[test]
fn test_webhook_event_serialization() {
    let event = WebhookEvent::IssueComment;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, r#""issue_comment""#);

    let event = WebhookEvent::PullRequestComment;
    let json = serde_json::to_string(&event).unwrap();
    assert_eq!(json, r#""pull_request_comment""#);
}

#[test]
fn test_webhook_event_deserialization() {
    let json = r#""issue_comment""#;
    let event: WebhookEvent = serde_json::from_str(json).unwrap();
    assert_eq!(event, WebhookEvent::IssueComment);
}

#[test]
fn test_create_webhook_request_construction() {
    let req = CreateWebhookRequest {
        url: "https://example.com/webhook".to_string(),
        secret: "secret123".to_string(),
        events: vec![WebhookEvent::IssueComment, WebhookEvent::PullRequestComment],
        active: true,
    };

    assert_eq!(req.url, "https://example.com/webhook");
    assert_eq!(req.events.len(), 2);
    assert!(req.active);
}

#[test]
fn test_git_webhook_response_construction() {
    use chrono::Utc;

    let webhook = GitWebhook {
        id: "123".to_string(),
        url: "https://example.com/webhook".to_string(),
        active: true,
        events: vec![WebhookEvent::IssueComment],
        created_at: Utc::now(),
    };

    assert_eq!(webhook.id, "123");
    assert!(webhook.active);
    assert_eq!(webhook.events.len(), 1);
}

#[test]
fn test_webhook_event_hash_and_eq() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(WebhookEvent::IssueComment);
    set.insert(WebhookEvent::IssueComment); // Duplicate

    assert_eq!(
        set.len(),
        1,
        "WebhookEvent should be hashable and comparable"
    );
}
