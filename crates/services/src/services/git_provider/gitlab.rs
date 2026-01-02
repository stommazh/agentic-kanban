//! GitLab provider implementation
//!
//! Uses `glab` CLI for core MR operations (create, list, status).
//! Uses REST API for comments (requires GitLab token in config).
//!
//! If `glab` CLI is authenticated, core MR operations just work.
//! For comments, user must configure GitLab API token in app settings.

mod api;
mod cli;

use async_trait::async_trait;
use secrecy::SecretString;

pub use cli::{GlabCli, GlabCliError};

use self::api::GitLabApiClient;
use super::{
    CreateMrRequest, GitProvider, PrInfo, ProviderError, ProviderType, RepoIdentifier,
    UnifiedComment,
};

/// GitLab provider implementation
///
/// Core MR operations use glab CLI.
/// Comments require API token (configured in app settings).
#[derive(Debug, Clone)]
pub struct GitLabProvider {
    cli: GlabCli,
    api_client: Option<GitLabApiClient>,
    /// Host for self-hosted instances (None for gitlab.com)
    /// Reserved for future use in URL construction
    #[allow(dead_code)]
    host: Option<String>,
}

impl GitLabProvider {
    /// Create new GitLab provider
    ///
    /// - For core MR operations: uses `glab` CLI (requires `glab auth login`)
    /// - For comments: requires `GITLAB_TOKEN` env var or config setting
    ///
    /// For self-hosted instances, set `GITLAB_BASE_URL` environment variable.
    pub fn new() -> Self {
        let base_url = std::env::var("GITLAB_BASE_URL")
            .unwrap_or_else(|_| "https://gitlab.com".to_string());

        let api_base_url = if base_url.contains("/api/v4") {
            base_url.clone()
        } else {
            format!("{}/api/v4", base_url.trim_end_matches('/'))
        };

        // Extract host for CLI (only for self-hosted)
        let host = if base_url != "https://gitlab.com" && base_url != "https://gitlab.com/api/v4" {
            Some(
                base_url
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .trim_end_matches("/api/v4")
                    .to_string(),
            )
        } else {
            None
        };

        // API client is optional - only created if token is available
        let api_client = std::env::var("GITLAB_TOKEN")
            .ok()
            .map(|token| GitLabApiClient::new(api_base_url, SecretString::from(token)));

        Self {
            cli: GlabCli::new(host.clone()),
            api_client,
            host,
        }
    }

    /// Create provider with explicit token (for config-based token)
    pub fn with_token(token: Option<String>) -> Self {
        let base_url = std::env::var("GITLAB_BASE_URL")
            .unwrap_or_else(|_| "https://gitlab.com".to_string());

        let api_base_url = if base_url.contains("/api/v4") {
            base_url.clone()
        } else {
            format!("{}/api/v4", base_url.trim_end_matches('/'))
        };

        let host = if base_url != "https://gitlab.com" && base_url != "https://gitlab.com/api/v4" {
            Some(
                base_url
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .trim_end_matches("/api/v4")
                    .to_string(),
            )
        } else {
            None
        };

        // Use provided token, falling back to env var
        let api_client = token
            .or_else(|| std::env::var("GITLAB_TOKEN").ok())
            .map(|t| GitLabApiClient::new(api_base_url, SecretString::from(t)));

        Self {
            cli: GlabCli::new(host.clone()),
            api_client,
            host,
        }
    }

    /// Check if API client is available (token configured)
    pub fn has_api_token(&self) -> bool {
        self.api_client.is_some()
    }
}

impl Default for GitLabProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl From<GlabCliError> for ProviderError {
    fn from(err: GlabCliError) -> Self {
        match err {
            GlabCliError::NotAvailable => ProviderError::NotInstalled {
                cli_name: "glab".into(),
            },
            GlabCliError::AuthFailed(msg) => ProviderError::NotAuthenticated(msg),
            GlabCliError::CommandFailed(msg) => ProviderError::CommandFailed(msg),
            GlabCliError::UnexpectedOutput(msg) => ProviderError::ParseError(msg),
            GlabCliError::NotSupported(msg) => ProviderError::NotSupported { feature: msg },
        }
    }
}

#[async_trait]
impl GitProvider for GitLabProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitLab
    }

    async fn check_auth(&self) -> Result<(), ProviderError> {
        // Check CLI auth (for core MR operations)
        let cli = self.cli.clone();
        tokio::task::spawn_blocking(move || cli.check_auth())
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            .map_err(ProviderError::from)
    }

    async fn create_merge_request(
        &self,
        repo: &RepoIdentifier,
        req: &CreateMrRequest,
    ) -> Result<PrInfo, ProviderError> {
        let cli = self.cli.clone();
        let repo_clone = repo.clone();
        let req_clone = req.clone();

        tokio::task::spawn_blocking(move || cli.create_mr(&repo_clone, &req_clone))
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            .map_err(ProviderError::from)
    }

    async fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<PrInfo, ProviderError> {
        let cli = self.cli.clone();
        let repo_clone = repo.clone();

        tokio::task::spawn_blocking(move || cli.get_mr_status(&repo_clone, number))
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            .map_err(ProviderError::from)
    }

    async fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, ProviderError> {
        let cli = self.cli.clone();
        let repo_clone = repo.clone();
        let branch_clone = branch.to_string();

        tokio::task::spawn_blocking(move || cli.list_mrs_for_branch(&repo_clone, &branch_clone))
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            .map_err(ProviderError::from)
    }

    async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError> {
        // Use API client if token is configured
        if let Some(ref api_client) = self.api_client {
            tracing::debug!("Fetching MR comments via GitLab API");
            return api_client.get_comments(repo, number).await;
        }

        // No token configured - return empty list with info message
        tracing::info!(
            "GitLab API token not configured - MR comments unavailable. \
             Configure token in Settings > Integrations > GitLab"
        );
        Ok(vec![])
    }
}
