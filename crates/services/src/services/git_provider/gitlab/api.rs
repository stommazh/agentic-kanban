//! GitLab REST API client

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use reqwest::StatusCode;
use secrecy::{ExposeSecret, SecretString};

use super::types::{
    GitLabError, GitLabMergeRequest, GitLabMrState, GitLabNote, GitLabProject,
};
use crate::services::git_provider::{
    CreateMrRequest, PrInfo, PrState, ProviderError, RepoIdentifier, UnifiedComment,
};

/// GitLab REST API client
#[derive(Debug, Clone)]
pub struct GitLabApiClient {
    base_url: String,
    token: Option<SecretString>,
    http_client: reqwest::Client,
}

impl GitLabApiClient {
    /// Create new GitLab API client
    pub fn new(base_url: String, token: Option<SecretString>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            base_url,
            token,
            http_client,
        }
    }

    /// Check authentication by fetching user info
    pub async fn check_auth(&self) -> Result<(), ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        let response = self
            .http_client
            .get(format!("{}/user", self.base_url))
            .header("PRIVATE-TOKEN", token.expose_secret())
            .send()
            .await
            .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ProviderError::NotAuthenticated(format!(
                "GitLab auth failed ({}): {}",
                status, error_text
            )))
        }
    }

    /// Create merge request
    pub async fn create_mr(
        &self,
        repo: &RepoIdentifier,
        req: &CreateMrRequest,
    ) -> Result<PrInfo, ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        // Get project ID first
        let project_id = self.get_project_id(repo).await?;

        // Prepare title (add Draft: prefix if draft)
        let title = if req.draft.unwrap_or(false) {
            format!("Draft: {}", req.title)
        } else {
            req.title.clone()
        };

        let body = serde_json::json!({
            "source_branch": req.head_branch,
            "target_branch": req.base_branch,
            "title": title,
            "description": req.body.as_deref().unwrap_or(""),
        });

        let result = (|| async {
            let response = self
                .http_client
                .post(format!(
                    "{}/projects/{}/merge_requests",
                    self.base_url, project_id
                ))
                .header("PRIVATE-TOKEN", token.expose_secret())
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

            self.handle_response(response).await
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitLab API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await?;

        Ok(result)
    }

    /// Get merge request status
    pub async fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        mr_number: u64,
    ) -> Result<PrInfo, ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        let project_id = self.get_project_id(repo).await?;

        let result = (|| async {
            let response = self
                .http_client
                .get(format!(
                    "{}/projects/{}/merge_requests/{}",
                    self.base_url, project_id, mr_number
                ))
                .header("PRIVATE-TOKEN", token.expose_secret())
                .send()
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

            self.handle_response(response).await
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitLab API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await?;

        Ok(result)
    }

    /// List merge requests for branch
    pub async fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        let project_id = self.get_project_id(repo).await?;

        let result = (|| async {
            let response = self
                .http_client
                .get(format!(
                    "{}/projects/{}/merge_requests",
                    self.base_url, project_id
                ))
                .header("PRIVATE-TOKEN", token.expose_secret())
                .query(&[("source_branch", branch), ("state", "all")])
                .send()
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(self.parse_error(status, &error_text));
            }

            let mrs: Vec<GitLabMergeRequest> = response
                .json()
                .await
                .map_err(|e| ProviderError::ParseError(format!("Failed to parse response: {e}")))?;

            Ok(mrs.into_iter().map(convert_mr_to_pr_info).collect())
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .notify(|err, dur: Duration| {
            tracing::warn!("GitLab API retry after {:.2}s: {}", dur.as_secs_f64(), err);
        })
        .await?;

        Ok(result)
    }

    /// Get comments/notes for merge request
    pub async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        mr_number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        let project_id = self.get_project_id(repo).await?;

        // Fetch general notes (comments)
        let notes_result = (|| async {
            let response = self
                .http_client
                .get(format!(
                    "{}/projects/{}/merge_requests/{}/notes",
                    self.base_url, project_id, mr_number
                ))
                .header("PRIVATE-TOKEN", token.expose_secret())
                .query(&[("sort", "asc"), ("order_by", "created_at")])
                .send()
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(self.parse_error(status, &error_text));
            }

            let notes: Vec<GitLabNote> = response
                .json()
                .await
                .map_err(|e| ProviderError::ParseError(format!("Failed to parse notes: {e}")))?;

            Ok(notes)
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .await?;

        // Convert to unified format, filtering out system notes
        let mut unified: Vec<UnifiedComment> = notes_result
            .into_iter()
            .filter(|note| !note.system)
            .map(|note| UnifiedComment::General {
                id: note.id.to_string(),
                author: note.author.username.clone(),
                author_association: "MEMBER".to_string(), // GitLab doesn't have this concept
                body: note.body,
                created_at: note.created_at,
                url: format!(
                    "{}/projects/{}/merge_requests/{}#note_{}",
                    self.base_url, project_id, mr_number, note.id
                ),
            })
            .collect();

        // Sort by creation time
        unified.sort_by_key(|c| c.created_at());

        Ok(unified)
    }

    /// Get project ID from path
    async fn get_project_id(&self, repo: &RepoIdentifier) -> Result<u64, ProviderError> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| ProviderError::NotAuthenticated("GITLAB_TOKEN not set".into()))?;

        let path = repo.full_path();
        // URL encode the path (e.g., "owner/repo" -> "owner%2Frepo")
        let encoded_path = path.replace('/', "%2F");

        let result = (|| async {
            let response = self
                .http_client
                .get(format!("{}/projects/{}", self.base_url, encoded_path))
                .header("PRIVATE-TOKEN", token.expose_secret())
                .send()
                .await
                .map_err(|e| ProviderError::CommandFailed(format!("API request failed: {e}")))?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(self.parse_error(status, &error_text));
            }

            let project: GitLabProject = response.json().await.map_err(|e| {
                ProviderError::ParseError(format!("Failed to parse project: {e}"))
            })?;

            Ok(project.id)
        })
        .retry(retry_config())
        .when(|e: &ProviderError| e.should_retry())
        .await?;

        Ok(result)
    }

    /// Handle API response and convert to PrInfo
    async fn handle_response(&self, response: reqwest::Response) -> Result<PrInfo, ProviderError> {
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(self.parse_error(status, &error_text));
        }

        let mr: GitLabMergeRequest = response
            .json()
            .await
            .map_err(|e| ProviderError::ParseError(format!("Failed to parse response: {e}")))?;

        Ok(convert_mr_to_pr_info(mr))
    }

    /// Parse error response
    fn parse_error(&self, status: StatusCode, body: &str) -> ProviderError {
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return ProviderError::NotAuthenticated(format!(
                "GitLab authentication failed: {}",
                body
            ));
        }

        // Try to parse as GitLab error
        if let Ok(error) = serde_json::from_str::<GitLabError>(body) {
            return ProviderError::ApiError {
                status: status.as_u16(),
                message: error.message(),
            };
        }

        ProviderError::ApiError {
            status: status.as_u16(),
            message: body.to_string(),
        }
    }
}

/// Convert GitLab MR to unified PrInfo
fn convert_mr_to_pr_info(mr: GitLabMergeRequest) -> PrInfo {
    let state = match mr.state {
        GitLabMrState::Opened => PrState::Open,
        GitLabMrState::Merged => PrState::Merged,
        GitLabMrState::Closed | GitLabMrState::Locked => PrState::Closed,
    };

    PrInfo {
        number: mr.iid,
        url: mr.web_url,
        state,
        merged_at: mr.merged_at,
        merge_commit_sha: mr.merge_commit_sha,
    }
}

fn retry_config() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(30))
        .with_max_times(3)
        .with_jitter()
}
