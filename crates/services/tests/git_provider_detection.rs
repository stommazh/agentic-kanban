//! Comprehensive URL pattern detection tests for git providers
//!
//! Tests 27 URL patterns for GitHub and GitLab detection including:
//! - Standard HTTPS and SSH formats
//! - Nested GitLab groups
//! - Self-hosted instances with custom domains and ports
//! - Case sensitivity, hyphenated/underscore names

use services::services::git_provider::{detect_provider_from_url, ProviderType};

#[test]
fn test_github_https_basic() {
    let (provider, repo) = detect_provider_from_url("https://github.com/owner/repo").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
    assert!(repo.host.is_none());
}

#[test]
fn test_github_https_with_git_extension() {
    let (provider, repo) = detect_provider_from_url("https://github.com/owner/repo.git").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
}

#[test]
fn test_github_https_trailing_slash() {
    let (provider, repo) = detect_provider_from_url("https://github.com/owner/repo/").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
}

#[test]
fn test_github_ssh_standard() {
    let (provider, repo) = detect_provider_from_url("git@github.com:owner/repo.git").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
}

#[test]
fn test_github_ssh_no_extension() {
    let (provider, repo) = detect_provider_from_url("git@github.com:owner/repo").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
}

#[test]
fn test_github_ssh_protocol() {
    let (provider, repo) = detect_provider_from_url("ssh://git@github.com/owner/repo.git").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "owner");
    assert_eq!(repo.name, "repo");
}

#[test]
fn test_github_complex_names() {
    // Repo with dashes and underscores
    let (provider, repo) = detect_provider_from_url("https://github.com/my-org/my_repo-v2").unwrap();
    assert_eq!(provider, ProviderType::GitHub);
    assert_eq!(repo.owner, "my-org");
    assert_eq!(repo.name, "my_repo-v2");
}

#[test]
fn test_gitlab_https_basic() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.com/group/project").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
    assert!(repo.host.is_none());
}

#[test]
fn test_gitlab_https_with_git_extension() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.com/group/project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_gitlab_ssh_standard() {
    let (provider, repo) = detect_provider_from_url("git@gitlab.com:group/project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_gitlab_ssh_no_extension() {
    let (provider, repo) = detect_provider_from_url("git@gitlab.com:group/project").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_gitlab_nested_groups() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.com/group/subgroup/project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group/subgroup");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_gitlab_deeply_nested_groups() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.com/org/team/subteam/project").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "org/team/subteam");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_gitlab_self_hosted_https() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.example.com/team/project").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "team");
    assert_eq!(repo.name, "project");
    assert_eq!(repo.host, Some("gitlab.example.com".to_string()));
}

#[test]
fn test_gitlab_self_hosted_ssh() {
    let (provider, repo) = detect_provider_from_url("git@gitlab.company.io:dev/app.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "dev");
    assert_eq!(repo.name, "app");
    assert_eq!(repo.host, Some("gitlab.company.io".to_string()));
}

#[test]
fn test_gitlab_self_hosted_port() {
    let (provider, repo) = detect_provider_from_url("https://gitlab.internal:8443/team/project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "team");
    assert_eq!(repo.name, "project");
    assert!(repo.host.is_some());
}

#[test]
fn test_gitlab_self_hosted_nested_groups() {
    let (provider, repo) =
        detect_provider_from_url("https://gitlab.enterprise.com/org/division/team/project").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "org/division/team");
    assert_eq!(repo.name, "project");
    assert_eq!(repo.host, Some("gitlab.enterprise.com".to_string()));
}

#[test]
fn test_gitlab_subdomain() {
    let (provider, repo) = detect_provider_from_url("https://code.gitlab.mycompany.com/team/app").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert!(repo.host.is_some());
}

#[test]
fn test_unknown_provider_bitbucket() {
    let result = detect_provider_from_url("https://bitbucket.org/owner/repo");
    assert!(result.is_err());
}

#[test]
fn test_unknown_provider_generic_git() {
    let result = detect_provider_from_url("git@git.example.com:owner/repo.git");
    assert!(result.is_err());
}

#[test]
fn test_invalid_url_format() {
    let result = detect_provider_from_url("not-a-valid-url");
    assert!(result.is_err());
}

#[test]
fn test_github_enterprise_not_supported() {
    // GitHub Enterprise Server (not gitlab) should fail
    let result = detect_provider_from_url("https://github.enterprise.com/owner/repo");
    assert!(result.is_err(), "GitHub Enterprise should not be auto-detected");
}

#[test]
fn test_gitlab_ssh_protocol_explicit() {
    let (provider, repo) =
        detect_provider_from_url("ssh://git@gitlab.example.com/group/project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
}

#[test]
fn test_repo_identifier_full_path() {
    let (_, repo) = detect_provider_from_url("https://gitlab.com/org/team/project").unwrap();
    assert_eq!(repo.full_path(), "org/team/project");
}

#[test]
fn test_gitlab_hyphenated_names() {
    let (provider, repo) =
        detect_provider_from_url("https://gitlab.com/my-org/my-project-v2.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "my-org");
    assert_eq!(repo.name, "my-project-v2");
}

#[test]
fn test_gitlab_underscore_names() {
    let (provider, repo) =
        detect_provider_from_url("git@gitlab.com:my_org/my_project.git").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "my_org");
    assert_eq!(repo.name, "my_project");
}

#[test]
fn test_case_sensitivity() {
    // GitLab is case-sensitive, but detection should work
    let (provider, repo) = detect_provider_from_url("https://gitlab.com/MyOrg/MyProject").unwrap();
    assert_eq!(provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "MyOrg");
    assert_eq!(repo.name, "MyProject");
}
