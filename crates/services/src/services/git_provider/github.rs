//! GitHub provider implementation

use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use tokio::task;

use super::{
    CreateMrRequest, GitProvider, PrInfo, ProviderError, ProviderType, RepoIdentifier,
    UnifiedComment,
};
use crate::services::github::cli::{GhCli, GhCliError};

/// GitHub provider implementation using gh CLI
#[derive(Debug, Clone)]
pub struct GitHubProvider {
    cli: GhCli,
}

impl GitHubProvider {
    pub fn new() -> Self {
        Self { cli: GhCli::new() }
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl From<GhCliError> for ProviderError {
    fn from(err: GhCliError) -> Self {
        match err {
            GhCliError::NotAvailable => ProviderError::NotInstalled {
                cli_name: "gh".into(),
            },
            GhCliError::AuthFailed(msg) => ProviderError::NotAuthenticated(msg),
            GhCliError::CommandFailed(msg) => ProviderError::CommandFailed(msg),
            GhCliError::UnexpectedOutput(msg) => ProviderError::ParseError(msg),
        }
    }
}

#[async_trait]
impl GitProvider for GitHubProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitHub
    }

    async fn check_auth(&self) -> Result<(), ProviderError> {
        let cli = self.cli.clone();
        task::spawn_blocking(move || cli.check_auth())
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
        let owner = repo.owner.clone();
        let name = repo.name.clone();
        let title = req.title.clone();
        let body = req.body.clone();
        let head = req.head_branch.clone();
        let base = req.base_branch.clone();
        let draft = req.draft;

        let result = (|| async {
            let cli = cli.clone();
            let owner = owner.clone();
            let name = name.clone();
            let title = title.clone();
            let body = body.clone();
            let head = head.clone();
            let base = base.clone();

            let pr_info = task::spawn_blocking(move || {
                use crate::services::github::{CreatePrRequest as GhCreatePrRequest, GitHubRepoInfo};

                let repo_info = GitHubRepoInfo {
                    owner,
                    repo_name: name,
                };
                let request = GhCreatePrRequest {
                    title,
                    body,
                    head_branch: head,
                    base_branch: base,
                    draft,
                };
                cli.create_pr(&request, &repo_info)
            })
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
            .map_err(ProviderError::from)?;

            Ok(convert_pr_info(pr_info))
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitHub API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await?;

        Ok(result)
    }

    async fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<PrInfo, ProviderError> {
        let cli = self.cli.clone();
        let owner = repo.owner.clone();
        let name = repo.name.clone();

        (|| async {
            let cli = cli.clone();
            let owner = owner.clone();
            let name = name.clone();

            let pr_info = task::spawn_blocking(move || cli.view_pr(&owner, &name, number as i64))
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
                .map_err(ProviderError::from)?;

            Ok(convert_pr_info(pr_info))
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitHub API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await
    }

    async fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, ProviderError> {
        let cli = self.cli.clone();
        let owner = repo.owner.clone();
        let name = repo.name.clone();
        let branch = branch.to_string();

        (|| async {
            let cli = cli.clone();
            let owner = owner.clone();
            let name = name.clone();
            let branch = branch.clone();

            let prs = task::spawn_blocking(move || cli.list_prs_for_branch(&owner, &name, &branch))
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
                .map_err(ProviderError::from)?;

            Ok(prs.into_iter().map(convert_pr_info).collect())
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitHub API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await
    }

    async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError> {
        let cli = self.cli.clone();
        let owner = repo.owner.clone();
        let name = repo.name.clone();

        // Fetch both types in parallel
        let general_future = {
            let cli = cli.clone();
            let owner = owner.clone();
            let name = name.clone();
            async move {
                (|| async {
                    let cli = cli.clone();
                    let owner = owner.clone();
                    let name = name.clone();
                    task::spawn_blocking(move || cli.get_pr_comments(&owner, &name, number as i64))
                        .await
                        .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
                        .map_err(ProviderError::from)
                })
                .retry(retry_config())
                .when(|e: &ProviderError| e.should_retry())
                .await
            }
        };

        let review_future = {
            let cli = cli.clone();
            let owner = owner.clone();
            let name = name.clone();
            async move {
                (|| async {
                    let cli = cli.clone();
                    let owner = owner.clone();
                    let name = name.clone();
                    task::spawn_blocking(move || {
                        cli.get_pr_review_comments(&owner, &name, number as i64)
                    })
                    .await
                    .map_err(|e| ProviderError::CommandFailed(format!("Task join error: {e}")))?
                    .map_err(ProviderError::from)
                })
                .retry(retry_config())
                .when(|e: &ProviderError| e.should_retry())
                .await
            }
        };

        let (general_result, review_result) = tokio::join!(general_future, review_future);
        let general = general_result?;
        let review = review_result?;

        // Convert to unified format
        let mut unified: Vec<UnifiedComment> = Vec::new();

        for c in general {
            unified.push(UnifiedComment::General {
                id: c.id,
                author: c.author.login,
                author_association: c.author_association,
                body: c.body,
                created_at: c.created_at,
                url: c.url,
            });
        }

        for c in review {
            unified.push(UnifiedComment::Review {
                id: c.id,
                author: c.user.login,
                author_association: c.author_association,
                body: c.body,
                created_at: c.created_at,
                url: c.html_url,
                path: c.path,
                line: c.line,
                diff_hunk: c.diff_hunk,
            });
        }

        // Sort by creation time
        unified.sort_by_key(|c| c.created_at());

        Ok(unified)
    }
}

/// Convert db::models::merge::PullRequestInfo to PrInfo
fn convert_pr_info(pr: db::models::merge::PullRequestInfo) -> PrInfo {
    PrInfo {
        number: pr.number as u64,
        url: pr.url,
        state: pr.status.into(),
        merged_at: pr.merged_at,
        merge_commit_sha: pr.merge_commit_sha,
    }
}

fn retry_config() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(30))
        .with_max_times(3)
        .with_jitter()
}
