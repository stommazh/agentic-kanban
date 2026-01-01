//! Provider detection from git remote URLs

use git2::Repository;
use regex::Regex;
use std::path::Path;

use super::{ProviderError, ProviderType, RepoIdentifier};

/// Detect provider and repo info from repository path
pub fn detect_provider(repo_path: &Path) -> Result<(ProviderType, RepoIdentifier), ProviderError> {
    let url = get_remote_url(repo_path)?;
    detect_provider_from_url(&url)
}

/// Get remote URL from repository path
pub fn get_remote_url(repo_path: &Path) -> Result<String, ProviderError> {
    let repo = Repository::open(repo_path)
        .map_err(|e| ProviderError::Git(format!("Failed to open repo: {e}")))?;

    // Try common remote names
    let remote_names = ["origin", "upstream"];
    for name in remote_names {
        if let Ok(remote) = repo.find_remote(name) {
            if let Some(url) = remote.url() {
                return Ok(url.to_string());
            }
        }
    }

    // Try first remote
    if let Ok(remotes) = repo.remotes() {
        if let Some(Some(name)) = remotes.iter().next() {
            if let Ok(remote) = repo.find_remote(name) {
                if let Some(url) = remote.url() {
                    return Ok(url.to_string());
                }
            }
        }
    }

    Err(ProviderError::Git("No remote URL found".into()))
}

/// Detect provider type and extract repo info from URL
pub fn detect_provider_from_url(url: &str) -> Result<(ProviderType, RepoIdentifier), ProviderError> {
    // Try GitHub first
    if let Some(repo_id) = parse_github_url(url) {
        return Ok((ProviderType::GitHub, repo_id));
    }

    // Try GitLab
    if let Some(repo_id) = parse_gitlab_url(url) {
        return Ok((ProviderType::GitLab, repo_id));
    }

    Err(ProviderError::UnknownProvider(url.to_string()))
}

/// Parse GitHub URLs (SSH and HTTPS)
fn parse_github_url(url: &str) -> Option<RepoIdentifier> {
    // Patterns:
    // - git@github.com:owner/repo.git
    // - https://github.com/owner/repo
    // - https://github.com/owner/repo.git
    // - ssh://git@github.com/owner/repo.git

    let re = Regex::new(r"github\.com[:/](?P<owner>[^/]+)/(?P<repo>[^/]+?)(?:\.git)?(?:/|$)")
        .ok()?;

    let caps = re.captures(url)?;
    let owner = caps.name("owner")?.as_str().to_string();
    let name = caps.name("repo")?.as_str().to_string();

    Some(RepoIdentifier::new_github(owner, name))
}

/// Parse GitLab URLs (SSH and HTTPS, including self-hosted)
fn parse_gitlab_url(url: &str) -> Option<RepoIdentifier> {
    // Patterns:
    // - git@gitlab.com:group/project.git
    // - https://gitlab.com/group/project
    // - https://gitlab.example.com/group/subgroup/project.git
    // - ssh://git@gitlab.example.com/group/project.git

    // Check if URL contains "gitlab" anywhere (gitlab.com or self-hosted)
    if !url.to_lowercase().contains("gitlab") {
        return None;
    }

    // Extract host for self-hosted check
    let host = extract_gitlab_host(url)?;
    let is_cloud = host == "gitlab.com";

    // GitLab supports nested groups: group/subgroup/project
    // For simplicity, treat everything before last segment as "owner"
    // Pattern handles:
    // - git@gitlab.com:group/project (SSH with colon)
    // - https://gitlab.com/group/project (HTTPS with slash after domain)
    // - https://gitlab.internal:8443/team/project (HTTPS with port)
    // - gitlab.example.com variants
    // Use greedy (.+) to capture full path, then trim trailing slash if present
    // Optional port (?::\d+)? handles URLs like gitlab.internal:8443
    let re = Regex::new(r"gitlab[^/:]*(?::\d+)?[:/](?P<path>.+?)(?:\.git)?$").ok()?;
    let caps = re.captures(url)?;
    let path = caps.name("path")?.as_str();

    // Split path into owner and project
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let name = parts.last()?.to_string();
    let owner = parts[..parts.len() - 1].join("/");

    Some(RepoIdentifier::new_gitlab(
        owner,
        name,
        if is_cloud { None } else { Some(host) },
    ))
}

/// Extract GitLab host from URL
fn extract_gitlab_host(url: &str) -> Option<String> {
    // SSH format: git@hostname:path
    if url.starts_with("git@") {
        let parts: Vec<&str> = url.splitn(2, ':').collect();
        if parts.len() == 2 {
            return Some(parts[0].trim_start_matches("git@").to_string());
        }
    }

    // HTTPS/SSH format: protocol://host/path
    let re = Regex::new(r"(?:https?://|ssh://(?:git@)?)?([^/:]+)").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_https() {
        let (ptype, repo) =
            detect_provider_from_url("https://github.com/owner/repo").unwrap();
        assert_eq!(ptype, ProviderType::GitHub);
        assert_eq!(repo.owner, "owner");
        assert_eq!(repo.name, "repo");
        assert!(repo.host.is_none());
    }

    #[test]
    fn test_github_https_with_git() {
        let (ptype, repo) =
            detect_provider_from_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(ptype, ProviderType::GitHub);
        assert_eq!(repo.owner, "owner");
        assert_eq!(repo.name, "repo");
    }

    #[test]
    fn test_github_ssh() {
        let (ptype, repo) =
            detect_provider_from_url("git@github.com:owner/repo.git").unwrap();
        assert_eq!(ptype, ProviderType::GitHub);
        assert_eq!(repo.owner, "owner");
        assert_eq!(repo.name, "repo");
    }

    #[test]
    fn test_gitlab_https() {
        let (ptype, repo) =
            detect_provider_from_url("https://gitlab.com/group/project").unwrap();
        assert_eq!(ptype, ProviderType::GitLab);
        assert_eq!(repo.owner, "group");
        assert_eq!(repo.name, "project");
        assert!(repo.host.is_none());
    }

    #[test]
    fn test_gitlab_ssh() {
        let (ptype, repo) =
            detect_provider_from_url("git@gitlab.com:group/project.git").unwrap();
        assert_eq!(ptype, ProviderType::GitLab);
        assert_eq!(repo.owner, "group");
        assert_eq!(repo.name, "project");
    }

    #[test]
    fn test_gitlab_nested_groups() {
        let (ptype, repo) =
            detect_provider_from_url("https://gitlab.com/group/subgroup/project.git").unwrap();
        assert_eq!(ptype, ProviderType::GitLab);
        assert_eq!(repo.owner, "group/subgroup");
        assert_eq!(repo.name, "project");
    }

    #[test]
    fn test_gitlab_self_hosted() {
        let (ptype, repo) =
            detect_provider_from_url("https://gitlab.example.com/team/project").unwrap();
        assert_eq!(ptype, ProviderType::GitLab);
        assert_eq!(repo.owner, "team");
        assert_eq!(repo.name, "project");
        assert_eq!(repo.host, Some("gitlab.example.com".to_string()));
    }

    #[test]
    fn test_gitlab_self_hosted_ssh() {
        let (ptype, repo) =
            detect_provider_from_url("git@gitlab.company.io:dev/app.git").unwrap();
        assert_eq!(ptype, ProviderType::GitLab);
        assert_eq!(repo.owner, "dev");
        assert_eq!(repo.name, "app");
        assert_eq!(repo.host, Some("gitlab.company.io".to_string()));
    }

    #[test]
    fn test_unknown_provider() {
        let result = detect_provider_from_url("https://bitbucket.org/owner/repo");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ProviderError::UnknownProvider(_)));
    }
}
