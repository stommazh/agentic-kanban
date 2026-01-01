//! GitLab provider implementation

mod api;
mod cli;
mod types;

use async_trait::async_trait;
use secrecy::SecretString;

pub use api::GitLabApiClient;
pub use cli::{GlabCli, GlabCliError};

use super::{
    CreateMrRequest, GitProvider, PrInfo, ProviderError, ProviderType, RepoIdentifier,
    UnifiedComment,
};

/// GitLab provider implementation
#[derive(Debug, Clone)]
pub struct GitLabProvider {
    cli: GlabCli,
    api_client: GitLabApiClient,
}

impl GitLabProvider {
    /// Create new GitLab provider
    ///
    /// Uses environment variables:
    /// - GITLAB_TOKEN: Personal Access Token (required)
    /// - GITLAB_BASE_URL: For self-hosted (default: https://gitlab.com/api/v4)
    pub fn new() -> Self {
        let base_url = std::env::var("GITLAB_BASE_URL")
            .unwrap_or_else(|_| "https://gitlab.com".to_string());

        let api_base_url = if base_url.contains("/api/v4") {
            base_url.clone()
        } else {
            format!("{}/api/v4", base_url.trim_end_matches('/'))
        };

        let token = std::env::var("GITLAB_TOKEN").ok().map(SecretString::from);

        // Extract host for CLI
        let cli_host = if base_url != "https://gitlab.com" && base_url != "https://gitlab.com/api/v4"
        {
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

        Self {
            cli: GlabCli::new(cli_host),
            api_client: GitLabApiClient::new(api_base_url, token),
        }
    }

    /// Check if CLI is available and authenticated
    fn check_cli_available(&self) -> bool {
        self.cli.check_auth().is_ok()
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
        // Try CLI first
        if let Ok(()) = tokio::task::spawn_blocking({
            let cli = self.cli.clone();
            move || cli.check_auth()
        })
        .await
        .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
        {
            return Ok(());
        }

        // Fallback to API
        self.api_client.check_auth().await
    }

    async fn create_merge_request(
        &self,
        repo: &RepoIdentifier,
        req: &CreateMrRequest,
    ) -> Result<PrInfo, ProviderError> {
        // Try CLI first if available
        if self.check_cli_available() {
            let cli = self.cli.clone();
            let repo_clone = repo.clone();
            let req_clone = req.clone();

            match tokio::task::spawn_blocking(move || cli.create_mr(&repo_clone, &req_clone))
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            {
                Ok(pr_info) => {
                    tracing::debug!("Created MR via glab CLI");
                    return Ok(pr_info);
                }
                Err(e) => {
                    tracing::warn!("glab CLI failed, falling back to API: {}", e);
                }
            }
        }

        // Fallback to API
        tracing::debug!("Creating MR via GitLab API");
        self.api_client.create_mr(repo, req).await
    }

    async fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<PrInfo, ProviderError> {
        // Try CLI first if available
        if self.check_cli_available() {
            let cli = self.cli.clone();
            let repo_clone = repo.clone();

            match tokio::task::spawn_blocking(move || cli.get_mr_status(&repo_clone, number))
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            {
                Ok(pr_info) => {
                    tracing::debug!("Got MR status via glab CLI");
                    return Ok(pr_info);
                }
                Err(e) => {
                    tracing::warn!("glab CLI failed, falling back to API: {}", e);
                }
            }
        }

        // Fallback to API
        tracing::debug!("Getting MR status via GitLab API");
        self.api_client.get_mr_status(repo, number).await
    }

    async fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, ProviderError> {
        // Try CLI first if available
        if self.check_cli_available() {
            let cli = self.cli.clone();
            let repo_clone = repo.clone();
            let branch_clone = branch.to_string();

            match tokio::task::spawn_blocking(move || {
                cli.list_mrs_for_branch(&repo_clone, &branch_clone)
            })
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            {
                Ok(prs) => {
                    tracing::debug!("Listed MRs via glab CLI");
                    return Ok(prs);
                }
                Err(e) => {
                    tracing::warn!("glab CLI failed, falling back to API: {}", e);
                }
            }
        }

        // Fallback to API
        tracing::debug!("Listing MRs via GitLab API");
        self.api_client.list_mrs_for_branch(repo, branch).await
    }

    async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError> {
        // glab CLI doesn't support this well, use API directly
        tracing::debug!("Getting MR comments via GitLab API");
        self.api_client.get_comments(repo, number).await
    }
}
