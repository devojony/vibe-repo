use crate::entities::repo_provider;

use super::{error::GitProviderError, gitea::GiteaClient, GitClient, GitHubClient, GitLabClient};

/// Factory for creating GitClient instances based on provider type
///
/// This factory uses static dispatch by returning the `GitClient` enum
/// instead of `Box<dyn GitProvider>`, eliminating virtual dispatch overhead.
pub struct GitClientFactory;

impl GitClientFactory {
    /// Create a GitClient instance based on provider type
    ///
    /// # Arguments
    /// * `provider_type` - Type of provider ("gitea", "github", "gitlab")
    /// * `base_url` - Base URL of the provider instance
    /// * `access_token` - Authentication token
    ///
    /// # Returns
    /// A GitClient enum variant for the specified provider type
    ///
    /// # Errors
    /// Returns `UnsupportedProvider` if the provider type is not recognized
    pub fn create(
        provider_type: &str,
        base_url: &str,
        access_token: &str,
    ) -> Result<GitClient, GitProviderError> {
        match provider_type {
            "gitea" => Ok(GitClient::Gitea(GiteaClient::new(base_url, access_token))),
            "github" => Ok(GitClient::GitHub(GitHubClient::new(base_url, access_token))),
            "gitlab" => Ok(GitClient::GitLab(GitLabClient::new(base_url, access_token))),
            _ => Err(GitProviderError::UnsupportedProvider(
                provider_type.to_string(),
            )),
        }
    }

    /// Create a GitClient instance from a RepoProvider entity
    ///
    /// # Arguments
    /// * `provider` - RepoProvider entity model
    ///
    /// # Returns
    /// A GitClient enum variant for the provider's type
    ///
    /// # Errors
    /// Returns `UnsupportedProvider` if the provider type is not recognized
    pub fn from_provider(provider: &repo_provider::Model) -> Result<GitClient, GitProviderError> {
        let provider_type = match provider.provider_type {
            repo_provider::ProviderType::Gitea => "gitea",
            // Future implementations:
            // repo_provider::ProviderType::GitHub => "github",
            // repo_provider::ProviderType::GitLab => "gitlab",
        };
        Self::create(provider_type, &provider.base_url, &provider.access_token)
    }
}
