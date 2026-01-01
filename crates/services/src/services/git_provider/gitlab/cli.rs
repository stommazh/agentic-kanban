//! GitLab CLI (glab) wrapper

use std::{
    ffi::{OsStr, OsString},
    process::Command,
};

use chrono::{DateTime, Utc};
use serde_json::Value;
use thiserror::Error;
use utils::shell::resolve_executable_path_blocking;

use crate::services::git_provider::{CreateMrRequest, PrInfo, PrState, RepoIdentifier};

/// Errors from glab CLI
#[derive(Debug, Error)]
pub enum GlabCliError {
    #[error("GitLab CLI (`glab`) executable not found or not runnable")]
    NotAvailable,
    #[error("GitLab CLI command failed: {0}")]
    CommandFailed(String),
    #[error("GitLab CLI authentication failed: {0}")]
    AuthFailed(String),
    #[error("GitLab CLI returned unexpected output: {0}")]
    UnexpectedOutput(String),
    #[error("Feature not supported by glab CLI: {0}")]
    NotSupported(String),
}

/// GitLab CLI wrapper
#[derive(Debug, Clone, Default)]
pub struct GlabCli {
    /// Base URL for self-hosted instances
    base_url: Option<String>,
}

impl GlabCli {
    pub fn new(base_url: Option<String>) -> Self {
        Self { base_url }
    }

    /// Ensure glab CLI is available
    fn ensure_available(&self) -> Result<(), GlabCliError> {
        resolve_executable_path_blocking("glab").ok_or(GlabCliError::NotAvailable)?;
        Ok(())
    }

    /// Execute glab command and return stdout
    fn run<I, S>(&self, args: I) -> Result<String, GlabCliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.ensure_available()?;
        let glab = resolve_executable_path_blocking("glab").ok_or(GlabCliError::NotAvailable)?;
        let mut cmd = Command::new(&glab);

        // Add base URL if self-hosted
        if let Some(ref url) = self.base_url {
            cmd.env("GITLAB_HOST", url);
        }

        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd
            .output()
            .map_err(|err| GlabCliError::CommandFailed(err.to_string()))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        // Check for auth errors
        let lower = stderr.to_ascii_lowercase();
        if lower.contains("authentication failed")
            || lower.contains("unauthorized")
            || lower.contains("bad credentials")
            || lower.contains("glab auth login")
            || lower.contains("not logged in")
        {
            return Err(GlabCliError::AuthFailed(stderr));
        }

        Err(GlabCliError::CommandFailed(stderr))
    }

    /// Check authentication status
    pub fn check_auth(&self) -> Result<(), GlabCliError> {
        match self.run(["auth", "status"]) {
            Ok(_) => Ok(()),
            Err(GlabCliError::CommandFailed(msg)) => Err(GlabCliError::AuthFailed(msg)),
            Err(err) => Err(err),
        }
    }

    /// Create merge request
    pub fn create_mr(
        &self,
        repo: &RepoIdentifier,
        req: &CreateMrRequest,
    ) -> Result<PrInfo, GlabCliError> {
        let mut args: Vec<OsString> = Vec::with_capacity(16);
        args.push(OsString::from("mr"));
        args.push(OsString::from("create"));

        // Specify repo
        args.push(OsString::from("--repo"));
        args.push(OsString::from(repo.full_path()));

        // Branches
        args.push(OsString::from("--source-branch"));
        args.push(OsString::from(&req.head_branch));
        args.push(OsString::from("--target-branch"));
        args.push(OsString::from(&req.base_branch));

        // Title
        args.push(OsString::from("--title"));
        args.push(OsString::from(&req.title));

        // Description
        if let Some(ref body) = req.body {
            args.push(OsString::from("--description"));
            args.push(OsString::from(body));
        }

        // Draft flag
        if req.draft.unwrap_or(false) {
            args.push(OsString::from("--draft"));
        }

        let raw = self.run(args)?;
        Self::parse_mr_create_output(&raw)
    }

    /// Get MR status
    pub fn get_mr_status(
        &self,
        repo: &RepoIdentifier,
        mr_number: u64,
    ) -> Result<PrInfo, GlabCliError> {
        let raw = self.run([
            "mr",
            "view",
            &mr_number.to_string(),
            "--repo",
            &repo.full_path(),
            "--json",
        ])?;

        Self::parse_mr_json(&raw)
    }

    /// List MRs for branch
    pub fn list_mrs_for_branch(
        &self,
        repo: &RepoIdentifier,
        branch: &str,
    ) -> Result<Vec<PrInfo>, GlabCliError> {
        let raw = self.run([
            "mr",
            "list",
            "--repo",
            &repo.full_path(),
            "--source-branch",
            branch,
            "--json",
        ])?;

        Self::parse_mr_list_json(&raw)
    }

    /// Get comments for MR (not well supported by glab, use API instead)
    #[allow(dead_code)]
    pub fn get_comments(
        &self,
        _repo: &RepoIdentifier,
        _mr_number: u64,
    ) -> Result<Vec<()>, GlabCliError> {
        // glab doesn't have good JSON output for notes/comments
        // Return not supported to trigger API fallback
        Err(GlabCliError::NotSupported(
            "Getting MR comments via CLI is not well supported, use API instead".to_string(),
        ))
    }

    /// Parse MR creation output
    fn parse_mr_create_output(raw: &str) -> Result<PrInfo, GlabCliError> {
        // glab mr create returns URL in output like:
        // !123 Create new feature (https://gitlab.com/owner/repo/-/merge_requests/123)
        // or just the URL

        let mr_url = raw
            .lines()
            .flat_map(|line| line.split_whitespace())
            .find(|token| token.starts_with("http") && token.contains("/merge_requests/"))
            .ok_or_else(|| {
                GlabCliError::UnexpectedOutput(format!(
                    "glab mr create did not return a merge request URL; raw output: {raw}"
                ))
            })?
            .trim_end_matches([')', '.', ',', ';'])
            .to_string();

        // Extract MR number from URL
        let number = mr_url
            .rsplit('/')
            .next()
            .ok_or_else(|| {
                GlabCliError::UnexpectedOutput(format!(
                    "Failed to extract MR number from URL '{mr_url}'"
                ))
            })?
            .parse::<u64>()
            .map_err(|err| {
                GlabCliError::UnexpectedOutput(format!(
                    "Failed to parse MR number from URL '{mr_url}': {err}"
                ))
            })?;

        Ok(PrInfo {
            number,
            url: mr_url,
            state: PrState::Open,
            merged_at: None,
            merge_commit_sha: None,
        })
    }

    /// Parse MR JSON from view command
    fn parse_mr_json(raw: &str) -> Result<PrInfo, GlabCliError> {
        let value: Value = serde_json::from_str(raw.trim()).map_err(|err| {
            GlabCliError::UnexpectedOutput(format!(
                "Failed to parse glab mr view response: {err}; raw: {raw}"
            ))
        })?;

        Self::extract_mr_info(&value).ok_or_else(|| {
            GlabCliError::UnexpectedOutput(format!(
                "glab mr view response missing required fields: {value:#?}"
            ))
        })
    }

    /// Parse MR list JSON
    fn parse_mr_list_json(raw: &str) -> Result<Vec<PrInfo>, GlabCliError> {
        let value: Value = serde_json::from_str(raw.trim()).map_err(|err| {
            GlabCliError::UnexpectedOutput(format!(
                "Failed to parse glab mr list response: {err}; raw: {raw}"
            ))
        })?;

        let arr = value.as_array().ok_or_else(|| {
            GlabCliError::UnexpectedOutput(format!(
                "glab mr list response is not an array: {value:#?}"
            ))
        })?;

        arr.iter()
            .map(|item| {
                Self::extract_mr_info(item).ok_or_else(|| {
                    GlabCliError::UnexpectedOutput(format!(
                        "glab mr list item missing required fields: {item:#?}"
                    ))
                })
            })
            .collect()
    }

    /// Extract MR info from JSON value
    fn extract_mr_info(value: &Value) -> Option<PrInfo> {
        let number = value.get("iid")?.as_u64()?;
        let url = value.get("web_url")?.as_str()?.to_string();
        let state_str = value.get("state")?.as_str().unwrap_or("opened");

        let state = match state_str.to_lowercase().as_str() {
            "opened" => PrState::Open,
            "merged" => PrState::Merged,
            "closed" | "locked" => PrState::Closed,
            _ => PrState::Unknown,
        };

        let merged_at = value
            .get("merged_at")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let merge_commit_sha = value
            .get("merge_commit_sha")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        Some(PrInfo {
            number,
            url,
            state,
            merged_at,
            merge_commit_sha,
        })
    }
}
