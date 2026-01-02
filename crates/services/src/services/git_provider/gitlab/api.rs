//! GitLab REST API client - used only for features not available via CLI
//!
//! Currently supports:
//! - Fetching MR comments/notes (requires API token)

use std::time::Duration;

use backon::{ExponentialBuilder, Retryable};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::services::git_provider::{ProviderError, RepoIdentifier, UnifiedComment};

/// GitLab note/comment on MR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabNote {
    pub id: u64,
    pub body: String,
    pub author: GitLabNoteAuthor,
    pub created_at: DateTime<Utc>,
    pub system: bool,
}

/// GitLab note author
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabNoteAuthor {
    pub id: u64,
    pub username: String,
    pub name: String,
}

/// GitLab project response (for getting project ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabProject {
    pub id: u64,
}

/// GitLab error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabError {
    pub message: Option<String>,
    pub error: Option<String>,
}

impl GitLabError {
    pub fn message(&self) -> String {
        self.message
            .clone()
            .or_else(|| self.error.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}

/// Minimal GitLab REST API client for comments only
#[derive(Debug, Clone)]
pub struct GitLabApiClient {
    base_url: String,
    token: SecretString,
    http_client: reqwest::Client,
}

impl GitLabApiClient {
    /// Create new GitLab API client
    pub fn new(base_url: String, token: SecretString) -> Self {
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

    /// Get comments/notes for merge request
    pub async fn get_comments(
        &self,
        repo: &RepoIdentifier,
        mr_number: u64,
    ) -> Result<Vec<UnifiedComment>, ProviderError> {
        let project_id = self.get_project_id(repo).await?;

        // Fetch general notes (comments)
        let notes_result = (|| async {
            let response = self
                .http_client
                .get(format!(
                    "{}/projects/{}/merge_requests/{}/notes",
                    self.base_url, project_id, mr_number
                ))
                .header("PRIVATE-TOKEN", self.token.expose_secret())
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
                author_association: "MEMBER".to_string(),
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
        let path = repo.full_path();
        // URL encode the path (e.g., "owner/repo" -> "owner%2Frepo")
        let encoded_path = path.replace('/', "%2F");

        let result = (|| async {
            let response = self
                .http_client
                .get(format!("{}/projects/{}", self.base_url, encoded_path))
                .header("PRIVATE-TOKEN", self.token.expose_secret())
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

fn retry_config() -> ExponentialBuilder {
    ExponentialBuilder::default()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(30))
        .with_max_times(3)
        .with_jitter()
}
