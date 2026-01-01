//! GitLab API response types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitLab project response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub path_with_namespace: String,
    pub web_url: String,
}

/// GitLab merge request state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitLabMrState {
    Opened,
    Closed,
    Locked,
    Merged,
}

/// GitLab merge request response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabMergeRequest {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: GitLabMrState,
    pub merged_at: Option<DateTime<Utc>>,
    pub merge_commit_sha: Option<String>,
    pub web_url: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
}

/// GitLab note/comment on MR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabNote {
    pub id: u64,
    pub body: String,
    pub author: GitLabNoteAuthor,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub system: bool,
    pub noteable_id: u64,
    pub noteable_type: String,
}

/// GitLab note author
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabNoteAuthor {
    pub id: u64,
    pub username: String,
    pub name: String,
}

/// GitLab diff note (inline comment on code)
/// Reserved for future use (Phase 3)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabDiffNote {
    pub id: u64,
    pub body: String,
    pub author: GitLabNoteAuthor,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub position: GitLabDiffPosition,
}

/// Position information for diff notes
/// Reserved for future use (Phase 3)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabDiffPosition {
    pub base_sha: String,
    pub head_sha: String,
    pub start_sha: String,
    pub new_path: Option<String>,
    pub old_path: Option<String>,
    pub new_line: Option<u32>,
    pub old_line: Option<u32>,
    pub position_type: String,
}

/// GitLab error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabError {
    pub message: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl GitLabError {
    pub fn message(&self) -> String {
        self.message
            .clone()
            .or_else(|| self.error.clone())
            .or_else(|| self.error_description.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}
