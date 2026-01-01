//! Shared types for git provider abstraction

use chrono::{DateTime, Utc};
use db::models::merge::MergeStatus;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Git hosting provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    GitHub,
    GitLab,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::GitHub => write!(f, "GitHub"),
            ProviderType::GitLab => write!(f, "GitLab"),
        }
    }
}

/// Repository identifier (works for both GitHub and GitLab)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RepoIdentifier {
    /// Provider type (GitHub/GitLab)
    pub provider: ProviderType,
    /// Owner (GitHub) or Group/Namespace (GitLab)
    pub owner: String,
    /// Repository name (GitHub) or Project name (GitLab)
    pub name: String,
    /// Custom host for self-hosted instances (None for cloud)
    pub host: Option<String>,
}

impl RepoIdentifier {
    pub fn new_github(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            provider: ProviderType::GitHub,
            owner: owner.into(),
            name: name.into(),
            host: None,
        }
    }

    pub fn new_gitlab(
        owner: impl Into<String>,
        name: impl Into<String>,
        host: Option<String>,
    ) -> Self {
        Self {
            provider: ProviderType::GitLab,
            owner: owner.into(),
            name: name.into(),
            host,
        }
    }

    /// Full path (owner/name)
    pub fn full_path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// PR/MR state (unified)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Merged,
    Closed,
    Unknown,
}

impl From<MergeStatus> for PrState {
    fn from(status: MergeStatus) -> Self {
        match status {
            MergeStatus::Open => PrState::Open,
            MergeStatus::Merged => PrState::Merged,
            MergeStatus::Closed => PrState::Closed,
            MergeStatus::Unknown => PrState::Unknown,
        }
    }
}

impl From<PrState> for MergeStatus {
    fn from(state: PrState) -> Self {
        match state {
            PrState::Open => MergeStatus::Open,
            PrState::Merged => MergeStatus::Merged,
            PrState::Closed => MergeStatus::Closed,
            PrState::Unknown => MergeStatus::Unknown,
        }
    }
}

/// Pull Request / Merge Request info (unified)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PrInfo {
    pub number: u64,
    pub url: String,
    pub state: PrState,
    pub merged_at: Option<DateTime<Utc>>,
    pub merge_commit_sha: Option<String>,
}

/// Request to create MR/PR
#[derive(Debug, Clone)]
pub struct CreateMrRequest {
    pub title: String,
    pub body: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    pub draft: Option<bool>,
}

/// Unified comment type (works for both GitHub PR and GitLab MR)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "comment_type", rename_all = "snake_case")]
#[ts(tag = "comment_type", rename_all = "snake_case", export_to = "../../shared/types.ts")]
pub enum UnifiedComment {
    /// General comment (conversation/note)
    General {
        id: String,
        author: String,
        author_association: String,
        body: String,
        created_at: DateTime<Utc>,
        url: String,
    },
    /// Inline review comment (on code)
    Review {
        id: i64,
        author: String,
        author_association: String,
        body: String,
        created_at: DateTime<Utc>,
        url: String,
        path: String,
        line: Option<i64>,
        diff_hunk: String,
    },
}

impl UnifiedComment {
    pub fn created_at(&self) -> DateTime<Utc> {
        match self {
            UnifiedComment::General { created_at, .. } => *created_at,
            UnifiedComment::Review { created_at, .. } => *created_at,
        }
    }
}
