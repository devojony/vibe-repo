//! Git provider integration and property tests

#[path = "git_provider/enum_dispatch_git_client_property_tests.rs"]
mod enum_dispatch_git_client_property_tests;

#[path = "git_provider/git_provider_error_property_tests.rs"]
mod git_provider_error_property_tests;

#[path = "git_provider/git_provider_factory_property_tests.rs"]
mod git_provider_factory_property_tests;

#[path = "git_provider/git_provider_webhook_models_tests.rs"]
mod git_provider_webhook_models_tests;

#[path = "git_provider/git_provider_webhook_trait_tests.rs"]
mod git_provider_webhook_trait_tests;

#[path = "git_provider/gitea_model_conversion_property_tests.rs"]
mod gitea_model_conversion_property_tests;

#[path = "git_provider/gitea_null_response_tests.rs"]
mod gitea_null_response_tests;
