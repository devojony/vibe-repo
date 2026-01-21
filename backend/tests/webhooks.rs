//! Webhook integration and property tests

#[path = "webhooks/gitea_webhook_tests.rs"]
mod gitea_webhook_tests;

#[path = "webhooks/webhook_api_tests.rs"]
mod webhook_api_tests;

#[path = "webhooks/webhook_cleanup_tests.rs"]
mod webhook_cleanup_tests;

#[path = "webhooks/webhook_entity_tests.rs"]
mod webhook_entity_tests;

#[path = "webhooks/webhook_event_routing_tests.rs"]
mod webhook_event_routing_tests;

#[path = "webhooks/webhook_logging_tests.rs"]
mod webhook_logging_tests;

#[path = "webhooks/webhook_migration_tests.rs"]
mod webhook_migration_tests;

#[path = "webhooks/webhook_payload_tests.rs"]
mod webhook_payload_tests;

#[path = "webhooks/webhook_repository_integration_tests.rs"]
mod webhook_repository_integration_tests;

#[path = "webhooks/webhook_retry_service_tests.rs"]
mod webhook_retry_service_tests;

#[path = "webhooks/webhook_retry_tests.rs"]
mod webhook_retry_tests;

#[path = "webhooks/webhook_verification_tests.rs"]
mod webhook_verification_tests;

#[path = "webhooks/webhook_pr_merge_tests.rs"]
mod webhook_pr_merge_tests;
