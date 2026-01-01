//! Git Provider Abstraction Layer
//!
//! Provides unified interface for GitHub and GitLab operations.
//! Auto-detects provider from git remote URL.

mod detection;
mod error;
mod github;
mod gitlab;
mod types;

pub use detection::{detect_provider, detect_provider_from_url, get_remote_url};
pub use error::ProviderError;
pub use github::GitHubProvider;
pub use gitlab::GitLabProvider;
pub use types::{
    CreateMrRequest, PrInfo, PrState, ProviderType, RepoIdentifier, UnifiedComment,
};

use async_trait::async_trait;
use std::path::Path;

/// Core trait for git provider operations (GitHub, GitLab, etc.)
#[async_trait]
pub trait GitProvider: Send + Sync {
    /// Returns provider type (GitHub/GitLab)
    fn provider_type(&self) -> ProviderType;

    /// Check if provider CLI is authenticated
    async fn check_auth(&self) -> Result<(), ProviderError>;

    /// Create a merge/pull request
    async fn create_merge_request(
        &self,
        repo: &RepoIdentifier,
        req: &CreateMrRequest,
    ) -> Result<PrInfo, ProviderError>;

    /// Get MR/PR status
    async fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<PrInfo, ProviderError>;

    /// List all MRs/PRs for a branch
    async fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, ProviderError>;

    /// Fetch comments/notes for MR/PR
    async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError>;
}

/// Create provider from repo path (auto-detects from remote URL)
pub fn create_provider(repo_path: &Path) -> Result<Box<dyn GitProvider>, ProviderError> {
    let (provider_type, _repo_id) = detect_provider(repo_path)?;
    match provider_type {
        ProviderType::GitHub => Ok(Box::new(GitHubProvider::new())),
        ProviderType::GitLab => Ok(Box::new(GitLabProvider::new())),
    }
}

/// Create provider from known type
pub fn create_provider_by_type(provider: ProviderType) -> Result<Box<dyn GitProvider>, ProviderError> {
    match provider {
        ProviderType::GitHub => Ok(Box::new(GitHubProvider::new())),
        ProviderType::GitLab => Ok(Box::new(GitLabProvider::new())),
    }
}
