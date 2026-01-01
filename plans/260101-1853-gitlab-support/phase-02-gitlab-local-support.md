# Phase 2: GitLab Local Support

**Effort:** 4-5 days
**Priority:** P1 (Core Feature)
**Prerequisites:** Phase 1 complete

## Context

Research:
- [GitLab API Analysis](./research/researcher-01-gitlab-api.md) - Auth methods, endpoints, rate limits
- [GitHub Integration](./scout/scout-01-github-integration.md) - CLI wrapper pattern to replicate

Phase 1 established `GitProvider` trait. Now implement GitLab-specific provider for local MR operations.

## Overview

Create `GitLabProvider` implementing `GitProvider` trait. Support MR creation, comment fetching via `glab` CLI with REST API fallback. Enable self-hosted GitLab instances.

## Key Insights

1. **glab CLI** less mature than `gh` - need REST API fallback for reliability
2. **Personal Access Tokens** simplest auth method (scopes: `api`, `read_repository`, `write_repository`)
3. **Project IDs** vs path encoding - GitLab uses numeric IDs or URL-encoded paths (`namespace%2Fproject`)
4. **Token expiry** - GitLab tokens expire (unlike GitHub), but only critical for Phase 5 (webhooks)
5. **Self-hosted** - Base URL configurable, API structure identical to GitLab.com

## Requirements

### Functional
- [ ] Implement `GitLabProvider` struct
- [ ] Wrap `glab` CLI commands (create MR, view MR, get notes)
- [ ] REST API client for operations unsupported by CLI
- [ ] Support Personal Access Token auth
- [ ] Handle self-hosted GitLab instances (custom base URL)
- [ ] Parse GitLab remote URLs (SSH/HTTPS)
- [ ] CLI setup helper for macOS (brew install glab)

### Non-Functional
- [ ] Graceful degradation if `glab` unavailable (use REST API)
- [ ] Rate limit respect (2000 req/min for authenticated)
- [ ] Error messages matching GitHub equivalents

## Architecture

### Module Structure
```
crates/services/src/services/git_provider/
├── gitlab.rs           # GitLabProvider implementation
└── gitlab/
    ├── cli.rs          # GlabCli wrapper
    ├── api.rs          # REST API client (fallback)
    └── types.rs        # GitLab-specific response types
```

### Core Implementation
```rust
pub struct GitLabProvider {
    cli: GlabCli,
    api_client: GitLabApiClient,
}

impl GitProvider for GitLabProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::GitLab
    }

    async fn check_auth(&self) -> Result<(), ProviderError> {
        // Try glab CLI first
        if self.cli.check_token().await.is_ok() {
            return Ok(());
        }
        // Fallback to API with PAT from env
        self.api_client.check_auth().await
    }

    async fn create_merge_request(&self, req: CreateMrRequest)
        -> Result<PrInfo, ProviderError> {
        // Prefer CLI for interactive operations
        match self.cli.create_mr(req).await {
            Ok(info) => Ok(info),
            Err(_) => self.api_client.create_mr(req).await,
        }
    }

    // ... other methods
}
```

### CLI Wrapper
```rust
pub struct GlabCli {
    base_url: Option<String>, // For self-hosted
}

impl GlabCli {
    pub async fn check_token(&self) -> Result<(), GlabCliError> {
        // Run: glab auth status
        let output = Command::new("glab")
            .args(["auth", "status"])
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(GlabCliError::NotAuthenticated)
        }
    }

    pub async fn create_mr(&self, req: CreateMrRequest)
        -> Result<PrInfo, GlabCliError> {
        // Run: glab mr create --title "..." --description "..." --target-branch main
        let mut cmd = Command::new("glab");
        cmd.args([
            "mr", "create",
            "--title", &req.title,
            "--description", &req.body,
            "--target-branch", &req.target_branch,
        ]);

        if req.draft {
            cmd.arg("--draft");
        }

        let output = cmd.output()?;
        self.parse_mr_output(&output)
    }

    pub async fn get_mr_notes(&self, project: &str, mr_number: u64)
        -> Result<Vec<MrNote>, GlabCliError> {
        // glab mr view doesn't have good JSON output for notes
        // Return NotSupported to fallback to API
        Err(GlabCliError::NotSupported {
            feature: "get_mr_notes".into()
        })
    }
}
```

### REST API Client
```rust
pub struct GitLabApiClient {
    base_url: String, // Default: https://gitlab.com/api/v4
    token: SecretString, // Personal Access Token
    http_client: reqwest::Client,
}

impl GitLabApiClient {
    pub async fn create_mr(&self, req: CreateMrRequest)
        -> Result<PrInfo, ProviderError> {
        // POST /api/v4/projects/:id/merge_requests
        let project_id = self.get_project_id(&req.repo).await?;

        let body = serde_json::json!({
            "source_branch": req.source_branch,
            "target_branch": req.target_branch,
            "title": req.draft.then(|| format!("Draft: {}", req.title))
                .unwrap_or(req.title),
            "description": req.body,
        });

        let response = self.http_client
            .post(&format!("{}/projects/{}/merge_requests",
                self.base_url, project_id))
            .header("PRIVATE-TOKEN", self.token.expose_secret())
            .json(&body)
            .send()
            .await?;

        let mr: GitLabMergeRequest = response.json().await?;
        Ok(PrInfo {
            number: mr.iid as u64,
            url: mr.web_url,
            title: mr.title,
            state: mr.state.into(),
        })
    }

    pub async fn get_mr_notes(&self, project: &str, mr_number: u64)
        -> Result<Vec<UnifiedComment>, ProviderError> {
        // GET /api/v4/projects/:id/merge_requests/:mr_iid/notes
        let project_id = self.get_project_id_from_path(project).await?;

        let response = self.http_client
            .get(&format!("{}/projects/{}/merge_requests/{}/notes",
                self.base_url, project_id, mr_number))
            .header("PRIVATE-TOKEN", self.token.expose_secret())
            .send()
            .await?;

        let notes: Vec<GitLabNote> = response.json().await?;
        Ok(notes.into_iter().map(|n| n.into()).collect())
    }

    async fn get_project_id_from_path(&self, path: &str)
        -> Result<u64, ProviderError> {
        // GET /api/v4/projects/:path (URL-encoded)
        let encoded = urlencoding::encode(path);
        let response = self.http_client
            .get(&format!("{}/projects/{}", self.base_url, encoded))
            .header("PRIVATE-TOKEN", self.token.expose_secret())
            .send()
            .await?;

        let project: GitLabProject = response.json().await?;
        Ok(project.id)
    }
}
```

### Configuration
```rust
// Environment variables
// - GITLAB_TOKEN: Personal Access Token (required)
// - GITLAB_BASE_URL: For self-hosted (default: https://gitlab.com/api/v4)

impl GitLabProvider {
    pub fn new() -> Result<Self, ProviderError> {
        let base_url = std::env::var("GITLAB_BASE_URL")
            .unwrap_or_else(|_| "https://gitlab.com/api/v4".to_string());

        let token = std::env::var("GITLAB_TOKEN")
            .ok()
            .map(SecretString::from);

        Ok(Self {
            cli: GlabCli::new(base_url.clone()),
            api_client: GitLabApiClient::new(base_url, token),
        })
    }
}
```

## Related Code Files

**To Create:**
- `crates/services/src/services/git_provider/gitlab.rs` (400+ lines)
- `crates/services/src/services/git_provider/gitlab/cli.rs` (300+ lines)
- `crates/services/src/services/git_provider/gitlab/api.rs` (500+ lines)
- `crates/services/src/services/git_provider/gitlab/types.rs` (200+ lines)

**To Update:**
- `crates/services/src/services/git_provider/mod.rs` - Export GitLabProvider
- `crates/services/src/services/git_provider/detection.rs` - Return GitLab provider
- `crates/server/src/routes/task_attempts/gh_cli_setup.rs` - Add glab setup route

**Reference:**
- `crates/services/src/services/git_provider/github/cli.rs` - Pattern for CLI wrapper
- `crates/services/src/services/github.rs` - Error handling patterns

## Implementation Steps

### Step 1: GitLab API Client (1.5d)
1. Create `gitlab/api.rs` with REST client
2. Implement token-based authentication
3. Add methods: `create_mr()`, `get_mr_notes()`, `get_project_id()`
4. Handle rate limiting (parse headers)
5. Support self-hosted base URL

### Step 2: GitLab CLI Wrapper (1d)
1. Create `gitlab/cli.rs` with `GlabCli` struct
2. Implement `check_token()`, `create_mr()`
3. Parse JSON output from `glab mr view --json`
4. Handle `glab` not installed error gracefully

### Step 3: GitLabProvider Implementation (1d)
1. Create `gitlab.rs` implementing `GitProvider` trait
2. Delegate to CLI or API based on availability
3. Implement `parse_remote_url()` for GitLab patterns
4. Handle draft MRs (use "Draft:" prefix in title)

### Step 4: Self-Hosted Support (0.5d)
1. Add base URL detection from git remote
2. Support custom domains in provider detection
3. Environment variable override (`GITLAB_BASE_URL`)
4. Validate connectivity on initialization

### Step 5: CLI Setup Helper (0.5d)
1. Add route: `POST /api/task-attempts/{id}/setup-glab-cli`
2. Detect Homebrew, run `brew install glab`
3. Guide user through `glab auth login`
4. Return setup errors (brew missing, not supported, etc.)

### Step 6: Provider Factory Update (0.5d)
1. Update `create_provider()` to return GitLabProvider for GitLab URLs
2. Handle provider initialization errors
3. Log provider detection for debugging

### Step 7: Testing (1d)
1. Unit tests for URL parsing (GitLab.com, self-hosted)
2. Integration tests with mock GitLab API
3. CLI wrapper tests (with glab mocked)
4. Error handling tests (auth failures, rate limits)

## Todo List

- [ ] Create `GitLabApiClient` struct with token auth
- [ ] Implement `create_mr()` via REST API
- [ ] Implement `get_mr_notes()` via REST API
- [ ] Add `get_project_id()` helper (path → numeric ID)
- [ ] Create `GlabCli` wrapper struct
- [ ] Implement `glab mr create` command wrapper
- [ ] Implement `glab auth status` check
- [ ] Create `GitLabProvider` implementing `GitProvider` trait
- [ ] Add GitLab remote URL parsing (SSH/HTTPS)
- [ ] Support self-hosted GitLab base URL detection
- [ ] Add CLI setup route for macOS (brew install glab)
- [ ] Update provider factory to return GitLab provider
- [ ] Write unit tests for GitLab API client
- [ ] Write unit tests for CLI wrapper
- [ ] Write integration tests with mock API
- [ ] Document environment variables (GITLAB_TOKEN, GITLAB_BASE_URL)
- [ ] Add rate limit handling (parse RateLimit-* headers)

## Success Criteria

### Must Have
- [x] Create MR via `glab` CLI successfully
- [x] Create MR via REST API (fallback)
- [x] Fetch MR notes/comments
- [x] Support self-hosted GitLab (custom base URL)
- [x] Graceful error when `glab` not installed
- [x] Personal Access Token authentication working

### Nice to Have
- [ ] Auto-detect base URL from git remote
- [ ] Cache project ID lookups (path → ID)
- [ ] Retry logic for transient API failures
- [ ] Progress feedback during MR creation

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `glab` CLI unreliable/buggy | High | High | REST API fallback for all operations |
| Self-hosted GitLab version incompatibility | Medium | Medium | Document min version (v13+), test against v13/v14/v15 |
| Project ID lookup overhead | Medium | Low | Cache IDs per session |
| Token expiry mid-operation | Low | Medium | Clear error message, prompt re-auth |
| Rate limiting on GitLab.com | Low | Medium | Respect headers, implement backoff |

## Security Considerations

- **Token storage**: Use `SecretString` for PAT, never log token values
- **Token scopes**: Document minimum required scopes (`api`, `read_repository`, `write_repository`)
- **Self-hosted URL validation**: Sanitize base URL input to prevent SSRF
- **Error messages**: Don't expose token or internal paths in errors
- **Token transmission**: Always use HTTPS for API calls

## Next Steps

After Phase 2 completion:
1. Begin Phase 3 (Server Integration)
2. Create unified API endpoints working with both providers
3. Update frontend to detect and display provider-specific terminology

## Unresolved Questions

1. **Token management**: Should we support multiple GitLab tokens (per instance)?
2. **glab reliability**: At what failure rate should we skip CLI entirely?
3. **Project namespace**: How to handle GitLab subgroups (group/subgroup/project)?
4. **Draft MRs**: GitLab supports `Draft:` prefix - should we also support legacy `WIP:` prefix?
5. **API version**: Should we support GitLab API v3 (legacy) or only v4?
