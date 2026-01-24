use serde_json::json;
use vibe_repo::git_provider::{gitea::GiteaClient, models::*, traits::GitProvider};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_gitea_create_webhook() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/repos/owner/repo/hooks"))
        .and(header("Authorization", "token test-token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": 1,
            "type": "gitea",
            "config": {
                "url": "https://example.com/webhook",
                "content_type": "json",
                "secret": "secret123"
            },
            "events": ["issue_comment", "pull_request_comment"],
            "active": true,
            "created_at": "2026-01-17T10:00:00Z",
            "updated_at": "2026-01-17T10:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = GiteaClient::new(&mock_server.uri(), "test-token").unwrap();

    let req = CreateWebhookRequest {
        url: "https://example.com/webhook".to_string(),
        secret: "secret123".to_string(),
        events: vec![WebhookEvent::IssueComment, WebhookEvent::PullRequestComment],
        active: true,
    };

    let webhook = client.create_webhook("owner", "repo", req).await.unwrap();

    assert_eq!(webhook.id, "1");
    assert_eq!(webhook.url, "https://example.com/webhook");
    assert!(webhook.active);
    assert_eq!(webhook.events.len(), 2);
}

#[tokio::test]
async fn test_gitea_delete_webhook() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/repos/owner/repo/hooks/1"))
        .and(header("Authorization", "token test-token"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = GiteaClient::new(&mock_server.uri(), "test-token").unwrap();

    let result = client.delete_webhook("owner", "repo", "1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_gitea_list_webhooks() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/owner/repo/hooks"))
        .and(header("Authorization", "token test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": 1,
                "type": "gitea",
                "config": {
                    "url": "https://example.com/webhook1",
                    "content_type": "json"
                },
                "events": ["issue_comment"],
                "active": true,
                "created_at": "2026-01-17T10:00:00Z",
                "updated_at": "2026-01-17T10:00:00Z"
            },
            {
                "id": 2,
                "type": "gitea",
                "config": {
                    "url": "https://example.com/webhook2",
                    "content_type": "json"
                },
                "events": ["push"],
                "active": false,
                "created_at": "2026-01-17T11:00:00Z",
                "updated_at": "2026-01-17T11:00:00Z"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = GiteaClient::new(&mock_server.uri(), "test-token").unwrap();

    let webhooks = client.list_webhooks("owner", "repo").await.unwrap();

    assert_eq!(webhooks.len(), 2);
    assert_eq!(webhooks[0].id, "1");
    assert_eq!(webhooks[0].url, "https://example.com/webhook1");
    assert!(webhooks[0].active);
    assert_eq!(webhooks[1].id, "2");
    assert!(!webhooks[1].active);
}
