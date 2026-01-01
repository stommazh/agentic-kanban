# Phase 4: Testing & Documentation

**Effort:** 2-3 days
**Priority:** P1 (Quality Assurance)
**Prerequisites:** Phase 1, 2, 3 complete

## Context

Phases 1-3 implemented core GitLab support. Now ensure reliability through comprehensive testing and enable users via clear documentation.

## Overview

Develop extensive test coverage for provider abstraction, GitLab operations, and backwards compatibility. Create user-facing documentation for self-hosted GitLab setup, CLI configuration, and troubleshooting.

## Key Insights

1. **Multi-provider testing** requires test doubles for both GitHub and GitLab APIs
2. **URL pattern coverage** critical - test 20+ git remote formats
3. **Self-hosted setup** biggest pain point for users - needs detailed guide
4. **CLI fallback logic** complex - needs state machine testing
5. **Backwards compatibility** must be proven, not assumed

## Requirements

### Functional Testing
- [ ] Unit tests for provider detection (20+ URL patterns)
- [ ] Unit tests for GitLab API client (all endpoints)
- [ ] Unit tests for CLI wrappers (gh, glab)
- [ ] Integration tests for MR creation (both providers)
- [ ] Integration tests for comment fetching
- [ ] Regression tests for existing GitHub workflows
- [ ] End-to-end tests for full PR/MR flow

### Documentation
- [ ] User guide for GitLab setup (PAT creation, CLI auth)
- [ ] Self-hosted GitLab configuration guide
- [ ] Troubleshooting common issues
- [ ] API documentation for new endpoints
- [ ] Migration guide (if needed)

## Architecture

### Test Structure
```
crates/services/tests/
├── git_provider/
│   ├── detection_test.rs      # URL pattern tests
│   ├── github_test.rs          # GitHub provider tests
│   ├── gitlab_test.rs          # GitLab provider tests
│   └── mod.rs
└── integration/
    ├── github_workflow_test.rs # Existing GitHub tests
    └── gitlab_workflow_test.rs # New GitLab tests

frontend/src/
├── components/dialogs/tasks/__tests__/
│   ├── CreatePRDialog.test.tsx
│   └── MergeRequestCommentsDialog.test.tsx
└── hooks/__tests__/
    └── useGitProvider.test.ts
```

### Mock Infrastructure
```rust
// Mock GitLab API server for integration tests
pub struct MockGitLabServer {
    server: wiremock::MockServer,
}

impl MockGitLabServer {
    pub async fn new() -> Self {
        let server = wiremock::MockServer::start().await;
        Self { server }
    }

    pub fn mock_create_mr(&self) -> MockBuilder {
        wiremock::Mock::given(method("POST"))
            .and(path_regex("/api/v4/projects/.+/merge_requests"))
            .respond_with(ResponseTemplate::new(201).set_body_json(
                serde_json::json!({
                    "iid": 42,
                    "web_url": "https://gitlab.com/owner/repo/-/merge_requests/42",
                    "title": "Test MR",
                    "state": "opened"
                })
            ))
    }
}
```

## Related Code Files

**To Create (Tests):**
- `crates/services/tests/git_provider/detection_test.rs`
- `crates/services/tests/git_provider/gitlab_test.rs`
- `crates/services/tests/integration/gitlab_workflow_test.rs`
- `frontend/src/components/dialogs/tasks/__tests__/CreatePRDialog.test.tsx`
- `frontend/src/hooks/__tests__/useGitProvider.test.ts`

**To Create (Docs):**
- `docs/gitlab-setup.md` - User guide for GitLab integration
- `docs/self-hosted-gitlab.md` - Self-hosted instance setup
- `docs/troubleshooting-git-providers.md` - Common issues
- `docs/api/merge-requests.md` - API endpoint documentation

**To Update:**
- `README.md` - Add GitLab to supported providers
- `docs/quickstart.md` - Include GitLab setup steps
- `crates/services/tests/git_workflow.rs` - Ensure still passes

## Implementation Steps

### Step 1: Unit Tests - Provider Detection (0.5d)
Test URL patterns:
```rust
#[test]
fn test_detect_github() {
    assert_eq!(
        detect_provider_from_remote("git@github.com:owner/repo.git"),
        Some(ProviderType::GitHub)
    );
    assert_eq!(
        detect_provider_from_remote("https://github.com/owner/repo"),
        Some(ProviderType::GitHub)
    );
}

#[test]
fn test_detect_gitlab() {
    // GitLab.com
    assert_eq!(
        detect_provider_from_remote("git@gitlab.com:group/project.git"),
        Some(ProviderType::GitLab)
    );

    // Self-hosted
    assert_eq!(
        detect_provider_from_remote("https://gitlab.example.com/group/project"),
        Some(ProviderType::GitLab)
    );

    // Subgroups
    assert_eq!(
        detect_provider_from_remote("git@gitlab.com:group/subgroup/project.git"),
        Some(ProviderType::GitLab)
    );
}

#[test]
fn test_detect_unknown() {
    assert_eq!(
        detect_provider_from_remote("git@bitbucket.org:owner/repo.git"),
        None
    );
}
```

### Step 2: Unit Tests - GitLab API Client (1d)
Mock HTTP responses for all endpoints:
```rust
#[tokio::test]
async fn test_create_mr() {
    let mock_server = MockGitLabServer::new().await;
    mock_server.mock_create_mr().mount(&mock_server.server).await;

    let client = GitLabApiClient::new(
        mock_server.server.uri(),
        SecretString::from("test-token"),
    );

    let result = client.create_mr(CreateMrRequest {
        title: "Test MR".into(),
        body: "Description".into(),
        target_branch: "main".into(),
        source_branch: "feature".into(),
        draft: false,
        repo: RepoIdentifier { /* ... */ },
    }).await;

    assert!(result.is_ok());
    let pr_info = result.unwrap();
    assert_eq!(pr_info.number, 42);
    assert_eq!(pr_info.title, "Test MR");
}

#[tokio::test]
async fn test_get_mr_notes() {
    let mock_server = MockGitLabServer::new().await;
    mock_server.mock_get_notes().mount(&mock_server.server).await;

    let client = GitLabApiClient::new(
        mock_server.server.uri(),
        SecretString::from("test-token"),
    );

    let notes = client.get_mr_notes("group/project", 42).await.unwrap();
    assert_eq!(notes.len(), 3);
}

#[tokio::test]
async fn test_rate_limit_handling() {
    let mock_server = MockGitLabServer::new().await;
    mock_server.mock_rate_limit_error().mount(&mock_server.server).await;

    let client = GitLabApiClient::new(
        mock_server.server.uri(),
        SecretString::from("test-token"),
    );

    let result = client.create_mr(/* ... */).await;
    assert!(matches!(result, Err(ProviderError::RateLimited { .. })));
}
```

### Step 3: Integration Tests - GitLab Workflow (1d)
End-to-end test with real git repository:
```rust
#[tokio::test]
#[ignore] // Requires glab CLI installed
async fn test_gitlab_mr_creation_flow() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_path = temp_dir.path();

    // Setup git repo with GitLab remote
    setup_test_repo(repo_path, "git@gitlab.com:test/repo.git");

    // Create provider
    let provider = git_provider::create_provider(repo_path).unwrap();
    assert_eq!(provider.provider_type(), ProviderType::GitLab);

    // Check auth
    provider.check_auth().await.expect("glab not authenticated");

    // Create branch and commit
    create_test_branch(repo_path, "test-branch");

    // Create MR
    let pr_info = provider.create_merge_request(CreateMrRequest {
        title: "Test MR".into(),
        body: "Test description".into(),
        target_branch: "main".into(),
        source_branch: "test-branch".into(),
        draft: false,
        repo: RepoIdentifier { /* ... */ },
    }).await.unwrap();

    assert!(pr_info.number > 0);
    assert!(pr_info.url.contains("gitlab.com"));
}
```

### Step 4: Regression Tests - GitHub (0.5d)
Ensure existing GitHub tests still pass:
```rust
#[tokio::test]
async fn test_github_pr_creation_unchanged() {
    // Run existing test from git_workflow.rs
    // Verify no behavioral changes
}

#[tokio::test]
async fn test_github_comment_fetching_unchanged() {
    // Verify GitHub comment fetching still works
}
```

### Step 5: Frontend Tests (0.5d)
Component and hook tests:
```typescript
// useGitProvider.test.ts
describe('useGitProvider', () => {
  it('returns GitHub for github.com remote', () => {
    const { result } = renderHook(() => useGitProvider('workspace-id'), {
      wrapper: createWrapper({ provider: 'github' }),
    });

    expect(result.current.provider).toBe('github');
    expect(result.current.terminology.pr).toBe('Pull Request');
  });

  it('returns GitLab for gitlab.com remote', () => {
    const { result } = renderHook(() => useGitProvider('workspace-id'), {
      wrapper: createWrapper({ provider: 'gitlab' }),
    });

    expect(result.current.provider).toBe('gitlab');
    expect(result.current.terminology.pr).toBe('Merge Request');
  });
});

// CreatePRDialog.test.tsx
describe('CreatePRDialog', () => {
  it('shows "Create Pull Request" for GitHub', () => {
    render(<CreatePRDialog />, { provider: 'github' });
    expect(screen.getByText('Create Pull Request')).toBeInTheDocument();
  });

  it('shows "Create Merge Request" for GitLab', () => {
    render(<CreatePRDialog />, { provider: 'gitlab' });
    expect(screen.getByText('Create Merge Request')).toBeInTheDocument();
  });
});
```

### Step 6: Documentation - User Guide (0.5d)
Create `docs/gitlab-setup.md`:
```markdown
# GitLab Setup Guide

## Prerequisites
- GitLab account (GitLab.com or self-hosted instance)
- Git repository with GitLab remote

## Option 1: Using glab CLI (Recommended)

### macOS Installation
```bash
brew install glab
```

### Authentication
```bash
glab auth login
```
Follow prompts to authenticate via browser OAuth flow.

## Option 2: Using Personal Access Token

1. Navigate to GitLab → Settings → Access Tokens
2. Create token with scopes: `api`, `read_repository`, `write_repository`
3. Set environment variable:
   ```bash
   export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
   ```

## Self-Hosted GitLab

Set base URL before authentication:
```bash
export GITLAB_BASE_URL="https://gitlab.example.com/api/v4"
glab auth login --hostname gitlab.example.com
```

## Verification

Test authentication:
```bash
glab auth status
```

Expected output:
```
✓ Logged in to gitlab.com as username
✓ Token: glpat-***
```

## Troubleshooting

See [Troubleshooting Guide](./troubleshooting-git-providers.md)
```

### Step 7: Documentation - Self-Hosted Guide (0.5d)
Create `docs/self-hosted-gitlab.md`:
```markdown
# Self-Hosted GitLab Configuration

## Minimum Requirements
- GitLab CE/EE version 13.0 or higher
- HTTPS enabled (required for OAuth)
- API v4 enabled (default)

## Configuration Steps

### 1. Create Personal Access Token
Navigate to: `https://gitlab.example.com/-/profile/personal_access_tokens`

Scopes required:
- ✅ api
- ✅ read_repository
- ✅ write_repository

### 2. Configure Vibe-Kanban
Set environment variables:
```bash
export GITLAB_BASE_URL="https://gitlab.example.com/api/v4"
export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
```

### 3. Configure glab CLI
```bash
glab auth login --hostname gitlab.example.com
```

## Network Requirements
- Port 443 (HTTPS) accessible from Vibe-Kanban server
- API rate limits: Default 2000 req/min (configurable by admin)

## Verification
Test connectivity:
```bash
curl -H "PRIVATE-TOKEN: $GITLAB_TOKEN" "$GITLAB_BASE_URL/version"
```

Expected response:
```json
{
  "version": "15.11.0",
  "revision": "abc123"
}
```

## Troubleshooting

### SSL Certificate Errors
If using self-signed certificate:
```bash
export GITLAB_INSECURE=true  # Not recommended for production
```

### Rate Limiting
Check response headers:
```
RateLimit-Limit: 2000
RateLimit-Remaining: 1998
RateLimit-Reset: 1609459200
```

Contact GitLab admin if limits too restrictive.
```

### Step 8: Update Main Documentation (0.25d)
Update `README.md`:
```markdown
## Supported Git Providers

Vibe-Kanban works seamlessly with:
- ✅ GitHub (github.com)
- ✅ GitLab (gitlab.com and self-hosted instances)

Provider auto-detected from your git remote URL.
```

## Todo List

- [ ] Write unit tests for provider detection (20+ URL patterns)
- [ ] Write unit tests for GitLab API client (create_mr, get_notes)
- [ ] Write unit tests for glab CLI wrapper
- [ ] Create mock GitLab API server for tests
- [ ] Write integration test for GitLab MR creation flow
- [ ] Write integration test for comment fetching
- [ ] Run existing GitHub regression tests
- [ ] Write frontend tests for useGitProvider hook
- [ ] Write frontend tests for CreatePRDialog (both providers)
- [ ] Create user guide: docs/gitlab-setup.md
- [ ] Create self-hosted guide: docs/self-hosted-gitlab.md
- [ ] Create troubleshooting guide: docs/troubleshooting-git-providers.md
- [ ] Update README.md with GitLab support
- [ ] Update API documentation with new endpoints
- [ ] Run full test suite (Rust + Frontend)
- [ ] Generate test coverage report (aim for >80%)
- [ ] Document environment variables in .env.example

## Success Criteria

### Must Have
- [x] Test coverage > 80% for new GitLab code
- [x] All existing GitHub tests pass (regression proof)
- [x] Provider detection tests cover 20+ URL formats
- [x] Integration test for full MR creation flow
- [x] User guide published for GitLab setup
- [x] Self-hosted configuration guide complete
- [x] Troubleshooting guide covers common errors

### Nice to Have
- [ ] E2E tests with real GitLab.com API (nightly CI)
- [ ] Performance benchmarks (MR creation time)
- [ ] Video walkthrough of GitLab setup
- [ ] Interactive troubleshooting wizard

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test coverage insufficient | Medium | High | Enforce coverage threshold in CI (>80%) |
| Integration tests flaky | High | Medium | Use mocked APIs, mark real API tests as `#[ignore]` |
| Documentation outdated quickly | Medium | Medium | Automate doc generation where possible |
| Self-hosted setup too complex | High | High | Provide pre-configured Docker image |

## Security Considerations

- **Test tokens**: Use separate test accounts, rotate tokens regularly
- **Mock servers**: Ensure test mocks don't expose real credentials
- **Documentation**: Never include real tokens in examples
- **Self-hosted guide**: Warn against `INSECURE=true` in production

## Next Steps

After Phase 4 completion:
1. Optional: Begin Phase 5 (Remote Integration - Webhooks & OAuth)
2. Monitor user feedback and bug reports
3. Iterate on documentation based on support requests

## Unresolved Questions

1. **CI infrastructure**: Should we test against real GitLab.com or only mocks?
2. **Test accounts**: Who maintains test GitLab accounts for CI?
3. **Performance benchmarks**: What's acceptable MR creation time (< 3s?)?
4. **Documentation hosting**: Integrate with existing docs site or separate?
5. **Coverage threshold**: 80% enough or aim for 90%?
