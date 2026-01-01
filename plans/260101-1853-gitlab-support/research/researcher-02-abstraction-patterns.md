# Multi-Provider Git Hosting Abstraction - Best Practices

## 1. GitHub vs GitLab API Differences

### Terminology
- **PR vs MR**: GitHub "Pull Request" vs GitLab "Merge Request" - purely semantic difference, functionally identical
- Both support squash, rebase, merge strategies with similar commands

### Structural Differences
**API Organization**:
- GitHub: Dual API (REST + GraphQL), GraphQL preferred for complex queries
- GitLab: Primarily REST-focused, GraphQL available but less mature
- Both use similar REST endpoint patterns: `/repos/:owner/:repo` vs `/projects/:id`

**Authentication**:
- GitHub: Personal Access Tokens (PATs), OAuth Apps, GitHub Apps (fine-grained permissions)
- GitLab: Personal Access Tokens, OAuth2, Deploy Tokens, Project/Group Access Tokens
- Token scope naming differs: `repo` (GitHub) vs `api`, `read_api` (GitLab)

**Repository Access**:
- GitHub: `owner/repo` format, numeric IDs optional
- GitLab: Numeric project IDs primary, URL-encoded path (`namespace%2Fproject`) secondary
- GitLab groups add complexity: nested namespaces (`group/subgroup/project`)

**Webhook Events**:
- Event name mapping needed:
  - `pull_request` (GH) → `merge_request` (GL)
  - `push` (both similar)
  - `issues` (GH) → `issue` (GL)
- Payload structures differ significantly - GL includes more nested objects

## 2. Abstraction Patterns from Existing Tools

### Pattern: Provider-Specific Adapters
**Gitea/Forgejo approach**:
- Gitea API nearly identical to GitHub v3 REST
- Forgejo hard fork maintains compatibility
- Allows reusing GitHub client logic with minimal changes

**Key insight**: Build provider-specific modules implementing common trait

### Pattern: URL-Based Detection
**git-url-parse libraries** (npm, Python):
- Parse SSH/HTTPS URLs to extract: provider, owner, repo, protocol
- Regex patterns for common providers:
  ```
  github.com:owner/repo.git
  git@gitlab.com:owner/repo.git
  https://gitlab.example.com/group/subgroup/project
  ```
- Fallback to generic Git protocol for self-hosted

### Pattern: Unified Interface with Provider Context
**Common trait design**:
```rust
trait GitProvider {
    // Core operations
    async fn get_repository(&self, identifier: &RepoIdentifier) -> Result<Repository>;
    async fn list_pull_requests(&self, repo: &RepoIdentifier) -> Result<Vec<PullRequest>>;
    async fn create_webhook(&self, repo: &RepoIdentifier, config: WebhookConfig) -> Result<Webhook>;

    // Provider metadata
    fn provider_type(&self) -> ProviderType;
    fn supports_graphql(&self) -> bool;
}

// Provider-specific implementations
struct GitHubProvider { client: octocrab::Octocrab }
struct GitLabProvider { client: gitlab::AsyncGitlab }
```

### Pattern: Abstraction Layer Translation
**Normalization approach**:
- Internal unified models (e.g., `PullRequest` struct)
- Provider modules translate to/from native formats
- Event webhook payloads normalized at ingress

## 3. Provider Detection Implementation

### URL Pattern Matching
```rust
enum ProviderType {
    GitHub,
    GitLab,
    Gitea,
    Forgejo,
    Bitbucket,
    Generic,
}

fn detect_provider(remote_url: &str) -> ProviderType {
    // Regex patterns:
    // github.com -> GitHub
    // gitlab.com or contains 'gitlab' in hostname -> GitLab
    // gitea/forgejo in hostname -> Gitea/Forgejo
    // bitbucket.org -> Bitbucket
    // Fallback: Generic with basic git operations only
}
```

### Detection Strategies
1. **URL hostname matching**: Most reliable for known providers
2. **API probing**: Query `/api/v4/version` (GitLab), `/api/version` (Gitea)
3. **Git remote metadata**: Limited, requires API call

### Self-Hosted Support
- Accept provider type as explicit config override
- Environment variable: `GIT_PROVIDER=gitlab`
- Parse from `.git/config` remote URL custom attributes

## 4. Unified Interface Design

### Core Trait Structure
```rust
// Common types
struct RepoIdentifier {
    provider: ProviderType,
    owner: String,
    name: String,
    host: Option<String>, // for self-hosted
}

struct PullRequest {
    id: u64,
    number: u64,
    title: String,
    state: PrState,
    author: User,
    source_branch: String,
    target_branch: String,
}

// Provider factory
fn create_provider(config: ProviderConfig) -> Box<dyn GitProvider> {
    match config.provider_type {
        ProviderType::GitHub => Box::new(GitHubProvider::new(config)),
        ProviderType::GitLab => Box::new(GitLabProvider::new(config)),
        // ...
    }
}
```

### API Method Mapping Strategy
**Tiered approach**:
1. **Tier 1** (universal): Operations all providers support identically
   - `get_repository`, `list_branches`, `get_commit`
2. **Tier 2** (common with translation): Requires mapping
   - `list_pull_requests`, `create_pull_request`, `merge_pull_request`
3. **Tier 3** (provider-specific): Optional, return `NotSupported` error
   - GitHub Checks API, GitLab CI/CD variables

### Error Handling
```rust
enum ProviderError {
    NotSupported(String), // Feature unavailable for this provider
    Authentication,
    RateLimited { retry_after: Duration },
    ApiError { status: u16, message: String },
}
```

## Implementation Recommendations

### Architecture
1. **Modular provider crates**: `git-provider-github`, `git-provider-gitlab`
2. **Common trait crate**: `git-provider-trait` with shared types
3. **Registry pattern**: Dynamic provider registration

### Key Design Principles
- **YAGNI**: Only implement methods currently needed (Tier 1 first)
- **KISS**: Start with REST APIs, add GraphQL only if necessary
- **DRY**: Share HTTP client config, auth handling, retry logic

### Testing Strategy
- Mock provider trait for unit tests
- Use real API test instances (GitHub/GitLab test repos)
- Separate integration tests per provider

## Unresolved Questions

1. **GraphQL for GitHub**: Worth complexity for batch operations, or REST sufficient?
2. **Webhook signature validation**: Each provider uses different HMAC algorithms - abstract or provider-specific?
3. **Rate limiting strategy**: Unified backoff, or respect per-provider limits?
4. **Repository identifier**: Use strings (`"owner/repo"`) or structured types throughout?
5. **Self-hosted GitLab**: API compatibility across versions - which minimum version to support?
