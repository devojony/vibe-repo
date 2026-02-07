use crate::entities::repository;

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
    /// Returns `ClientCreationError` if the HTTP client cannot be created
    pub fn create(
        provider_type: &str,
        base_url: &str,
        access_token: &str,
    ) -> Result<GitClient, GitProviderError> {
        match provider_type {
            "gitea" => Ok(GitClient::Gitea(
                GiteaClient::new(base_url, access_token)
                    .map_err(GitProviderError::ClientCreationError)?,
            )),
            "github" => Ok(GitClient::GitHub(GitHubClient::new(base_url, access_token))),
            "gitlab" => Ok(GitClient::GitLab(GitLabClient::new(base_url, access_token))),
            _ => Err(GitProviderError::UnsupportedProvider(
                provider_type.to_string(),
            )),
        }
    }

    /// Create a GitClient instance from a Repository entity
    ///
    /// # Arguments
    /// * `repo` - Repository entity model
    ///
    /// # Returns
    /// A GitClient enum variant for the repository's provider type
    ///
    /// # Errors
    /// Returns `UnsupportedProvider` if the provider type is not recognized
    /// Returns `ClientCreationError` if the HTTP client cannot be created
    pub fn from_repository(repo: &repository::Model) -> Result<GitClient, GitProviderError> {
        Self::create(
            &repo.provider_type,
            &repo.provider_base_url,
            &repo.access_token,
        )
    }
}
