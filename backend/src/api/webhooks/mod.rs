//! Webhook API module
//!
//! Handles incoming webhooks from Git providers (Gitea, GitHub, GitLab).
//! Processes webhook events and triggers automated development workflows.

pub mod event_handler;
pub mod handlers;
pub mod mention;
pub mod models;
pub mod routes;
pub mod verification;

pub use routes::router;
