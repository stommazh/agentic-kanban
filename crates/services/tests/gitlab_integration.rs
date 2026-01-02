//! GitLab Integration Workflow Tests
//!
//! End-to-end tests for GitLab MR creation and management workflows

use services::services::git_provider::{
    create_provider_by_type, detect_provider_from_url, CreateMrRequest, GitLabProvider,
    GitProvider, ProviderError, ProviderType, RepoIdentifier,
};

/// Test provider creation from type
#[test]
fn test_create_gitlab_provider_by_type() {
    let result = create_provider_by_type(ProviderType::GitLab);
    assert!(result.is_ok());
    let provider = result.unwrap();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

/// Test GitLab provider instantiation
#[test]
fn test_gitlab_provider_new() {
    let provider = GitLabProvider::new();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

/// Test GitLab provider default
#[test]
fn test_gitlab_provider_default() {
    let provider = GitLabProvider::default();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

/// Test detection from GitLab URLs
#[test]
fn test_detect_gitlab_from_urls() {
    let test_cases = vec![
        "https://gitlab.com/group/project",
        "git@gitlab.com:group/project.git",
        "https://gitlab.example.com/team/app.git",
        "git@gitlab.company.io:dev/service.git",
    ];

    for url in test_cases {
        let result = detect_provider_from_url(url);
        assert!(result.is_ok(), "Failed to detect GitLab from URL: {}", url);
        let (provider, _) = result.unwrap();
        assert_eq!(provider, ProviderType::GitLab);
    }
}

/// Test detection distinguishes GitHub from GitLab
#[test]
fn test_detect_provider_distinction() {
    let github_url = "https://github.com/owner/repo";
    let gitlab_url = "https://gitlab.com/group/project";

    let (gh_provider, _) = detect_provider_from_url(github_url).unwrap();
    let (gl_provider, _) = detect_provider_from_url(gitlab_url).unwrap();

    assert_eq!(gh_provider, ProviderType::GitHub);
    assert_eq!(gl_provider, ProviderType::GitLab);
    assert_ne!(gh_provider, gl_provider);
}

/// Test MR request construction
#[test]
fn test_create_mr_request_construction() {
    let req = CreateMrRequest {
        title: "Add new feature".to_string(),
        body: Some("This MR adds a new feature".to_string()),
        head_branch: "feature/new-thing".to_string(),
        base_branch: "main".to_string(),
        draft: Some(false),
    };

    assert_eq!(req.title, "Add new feature");
    assert_eq!(req.head_branch, "feature/new-thing");
    assert_eq!(req.base_branch, "main");
    assert!(!req.draft.unwrap());
}

/// Test draft MR request
#[test]
fn test_draft_mr_request() {
    let req = CreateMrRequest {
        title: "WIP: New feature".to_string(),
        body: Some("Work in progress".to_string()),
        head_branch: "wip-branch".to_string(),
        base_branch: "develop".to_string(),
        draft: Some(true),
    };

    assert!(req.draft.unwrap());
}

/// Test error handling for unsupported provider
#[test]
fn test_unsupported_provider_error() {
    let result = detect_provider_from_url("https://bitbucket.org/owner/repo");
    assert!(result.is_err());
    match result.unwrap_err() {
        ProviderError::UnknownProvider(url) => {
            assert!(url.contains("bitbucket"));
        }
        _ => panic!("Expected UnknownProvider error"),
    }
}

#[tokio::test]
#[ignore] // Requires glab CLI installed and authenticated
async fn test_gitlab_auth_check_with_cli() {
    // This test requires actual glab CLI setup
    let provider = GitLabProvider::new();
    let result = provider.check_auth().await;

    // Should succeed if glab is installed and authenticated
    // or fail with appropriate error message
    match result {
        Ok(()) => println!("GitLab auth check passed"),
        Err(e) => {
            // Expected errors: NotInstalled or NotAuthenticated
            assert!(
                matches!(e, ProviderError::NotInstalled { .. })
                    || matches!(e, ProviderError::NotAuthenticated(_))
            );
        }
    }
}

#[tokio::test]
#[ignore] // Requires real git repository with GitLab remote
async fn test_create_provider_from_gitlab_repo() {
    // This test would need a real git repo with GitLab remote
    // For CI, this should be mocked or use test fixtures

    // Example test structure:
    // let temp_dir = setup_test_gitlab_repo();
    // let provider = create_provider(&temp_dir.path()).unwrap();
    // assert_eq!(provider.provider_type(), ProviderType::GitLab);
}

#[tokio::test]
#[ignore] // Requires glab CLI and real GitLab access
async fn test_gitlab_mr_creation_flow() {
    // Full end-to-end test of MR creation
    // This would require:
    // 1. Real git repository with GitLab remote
    // 2. glab CLI installed and authenticated
    // 3. Permissions to create MRs in test project

    // Test structure:
    // let provider = GitLabProvider::new();
    // let repo = RepoIdentifier::new_gitlab("test-group", "test-project", None);
    // let req = CreateMrRequest { ... };
    // let result = provider.create_merge_request(&repo, &req).await;
    // assert!(result.is_ok());
    // let pr_info = result.unwrap();
    // assert!(pr_info.number > 0);
    // assert!(pr_info.url.contains("merge_requests"));
}

#[tokio::test]
#[ignore] // Requires glab CLI and real GitLab access
async fn test_gitlab_list_mrs_for_branch() {
    // Test listing MRs for a specific branch
    // Would require real GitLab project with existing MRs
}

#[tokio::test]
#[ignore] // Requires glab CLI and real GitLab access
async fn test_gitlab_get_mr_status() {
    // Test fetching MR status
    // Would require real GitLab project with existing MR
}

#[tokio::test]
#[ignore] // Requires GitLab API access
async fn test_gitlab_get_comments() {
    // Test fetching MR comments/notes
    // Would require real GitLab project with MR that has comments
}

/// Test self-hosted GitLab configuration
#[test]
fn test_self_hosted_gitlab_config() {
    // Set environment variables for self-hosted instance
    unsafe {
        std::env::set_var("GITLAB_BASE_URL", "https://gitlab.company.com");
    }

    let provider = GitLabProvider::new();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);

    // Clean up
    unsafe {
        std::env::remove_var("GITLAB_BASE_URL");
    }
}

/// Test nested group project paths
#[test]
fn test_nested_group_full_path() {
    let repo = RepoIdentifier::new_gitlab(
        "org/division/team".to_string(),
        "project".to_string(),
        None,
    );

    assert_eq!(repo.full_path(), "org/division/team/project");
}

/// Test URL encoding for API calls
#[test]
fn test_project_path_encoding() {
    let repo = RepoIdentifier::new_gitlab(
        "my-org/my-team".to_string(),
        "my-project".to_string(),
        None,
    );

    let full_path = repo.full_path();
    assert_eq!(full_path, "my-org/my-team/my-project");

    // In actual API calls, this would be URL encoded to:
    // "my-org%2Fmy-team%2Fmy-project"
    let encoded = full_path.replace('/', "%2F");
    assert_eq!(encoded, "my-org%2Fmy-team%2Fmy-project");
}

/// Test provider error types
#[test]
fn test_provider_error_types() {
    let errors = vec![
        ProviderError::NotInstalled {
            cli_name: "glab".to_string(),
        },
        ProviderError::NotAuthenticated("Not logged in".to_string()),
        ProviderError::ApiError {
            status: 404,
            message: "Not found".to_string(),
        },
        ProviderError::UnknownProvider("unknown://host/repo".to_string()),
    ];

    for error in errors {
        // All errors should be displayable
        let error_string = format!("{}", error);
        assert!(!error_string.is_empty());
    }
}

/// Test backwards compatibility with existing GitHub workflows
#[test]
fn test_github_provider_still_works() {
    let result = create_provider_by_type(ProviderType::GitHub);
    assert!(result.is_ok());
    let provider = result.unwrap();
    assert_eq!(provider.provider_type(), ProviderType::GitHub);
}

/// Test both providers can coexist
#[test]
fn test_multiple_providers_coexist() {
    let github = create_provider_by_type(ProviderType::GitHub).unwrap();
    let gitlab = create_provider_by_type(ProviderType::GitLab).unwrap();

    assert_ne!(github.provider_type(), gitlab.provider_type());
    assert_eq!(github.provider_type(), ProviderType::GitHub);
    assert_eq!(gitlab.provider_type(), ProviderType::GitLab);
}
