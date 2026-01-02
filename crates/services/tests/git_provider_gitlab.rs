//! GitLab CLI Integration Tests
//!
//! Tests GitLab provider operations via glab CLI
//!
//! Note: Most tests require glab CLI to be installed and authenticated.
//! Run with `cargo test --package services -- git_provider_gitlab --ignored` for full tests.

#![allow(dead_code)]

use services::services::git_provider::{
    CreateMrRequest, GitLabProvider, GitProvider, ProviderError, ProviderType, RepoIdentifier,
};

/// Helper to create test repo identifier
fn test_gitlab_repo() -> RepoIdentifier {
    RepoIdentifier::new_gitlab("test-group".to_string(), "test-project".to_string(), None)
}

/// Helper to create test MR request
fn test_mr_request() -> CreateMrRequest {
    CreateMrRequest {
        title: "Test MR".to_string(),
        body: Some("Test description".to_string()),
        head_branch: "feature-branch".to_string(),
        base_branch: "main".to_string(),
        draft: Some(false),
    }
}

#[test]
fn test_gitlab_provider_creation() {
    let provider = GitLabProvider::new();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

#[test]
fn test_gitlab_provider_default() {
    let provider = GitLabProvider::default();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

#[tokio::test]
#[ignore = "Requires glab CLI"]
async fn test_check_auth_without_credentials() {
    // Temporarily clear environment for self-hosted config
    let old_url = std::env::var("GITLAB_BASE_URL").ok();
    unsafe {
        std::env::remove_var("GITLAB_BASE_URL");
    }

    let provider = GitLabProvider::new();
    let result = provider.check_auth().await;

    // Restore environment
    if let Some(url) = old_url {
        unsafe {
            std::env::set_var("GITLAB_BASE_URL", url);
        }
    }

    // Will fail if glab not installed or not authenticated
    assert!(result.is_err());
}

#[test]
fn test_repo_identifier_gitlab_cloud() {
    let repo = RepoIdentifier::new_gitlab("group".to_string(), "project".to_string(), None);

    assert_eq!(repo.provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "group");
    assert_eq!(repo.name, "project");
    assert!(repo.host.is_none());
    assert_eq!(repo.full_path(), "group/project");
}

#[test]
fn test_repo_identifier_gitlab_self_hosted() {
    let repo = RepoIdentifier::new_gitlab(
        "team".to_string(),
        "app".to_string(),
        Some("gitlab.company.com".to_string()),
    );

    assert_eq!(repo.provider, ProviderType::GitLab);
    assert_eq!(repo.owner, "team");
    assert_eq!(repo.name, "app");
    assert_eq!(repo.host, Some("gitlab.company.com".to_string()));
    assert_eq!(repo.full_path(), "team/app");
}

#[test]
fn test_repo_identifier_nested_groups() {
    let repo = RepoIdentifier::new_gitlab(
        "org/division/team".to_string(),
        "project".to_string(),
        None,
    );

    assert_eq!(repo.owner, "org/division/team");
    assert_eq!(repo.name, "project");
    assert_eq!(repo.full_path(), "org/division/team/project");
}

#[test]
fn test_create_mr_request_draft() {
    let req = CreateMrRequest {
        title: "WIP Feature".to_string(),
        body: Some("Work in progress".to_string()),
        head_branch: "wip-branch".to_string(),
        base_branch: "develop".to_string(),
        draft: Some(true),
    };

    assert!(req.draft.unwrap());
    assert_eq!(req.title, "WIP Feature");
}

#[test]
fn test_create_mr_request_no_body() {
    let req = CreateMrRequest {
        title: "Simple MR".to_string(),
        body: None,
        head_branch: "feature".to_string(),
        base_branch: "main".to_string(),
        draft: Some(false),
    };

    assert!(req.body.is_none());
}

// Integration test placeholders (require actual API or mock server)
// These would be expanded with wiremock or similar in production

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_create_mr_success() {
    // Mock server would be set up here with expected response
    // For now, this is a placeholder for the test structure
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_get_mr_status_success() {
    // Mock server would return MR status
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_list_mrs_for_branch_success() {
    // Mock server would return list of MRs
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_get_comments_success() {
    // Mock server would return MR notes/comments
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_api_rate_limit_handling() {
    // Mock server would return 429 rate limit error
    // Test should retry with backoff
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_api_unauthorized_handling() {
    // Mock server would return 401/403 error
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_api_not_found_handling() {
    // Mock server would return 404 error for non-existent project
}

#[tokio::test]
#[ignore] // Requires mock server setup
async fn test_get_project_id_encoding() {
    // Test URL encoding of project path with slashes
    // "org/team/project" should become "org%2Fteam%2Fproject"
}

/// Test error conversion and display
#[test]
fn test_provider_error_display() {
    let error = ProviderError::NotAuthenticated("Token missing".to_string());
    assert!(format!("{}", error).contains("Token missing"));

    let error = ProviderError::ApiError {
        status: 404,
        message: "Project not found".to_string(),
    };
    assert!(format!("{}", error).contains("404"));
}

/// Test provider type equality
#[test]
fn test_provider_type_equality() {
    assert_eq!(ProviderType::GitLab, ProviderType::GitLab);
    assert_ne!(ProviderType::GitLab, ProviderType::GitHub);
}

/// Test provider type display
#[test]
fn test_provider_type_display() {
    assert_eq!(format!("{}", ProviderType::GitLab), "GitLab");
    assert_eq!(format!("{}", ProviderType::GitHub), "GitHub");
}

/// Test RepoIdentifier clone and equality
#[test]
fn test_repo_identifier_clone() {
    let repo1 = test_gitlab_repo();
    let repo2 = repo1.clone();

    assert_eq!(repo1, repo2);
    assert_eq!(repo1.provider, repo2.provider);
    assert_eq!(repo1.owner, repo2.owner);
    assert_eq!(repo1.name, repo2.name);
}
