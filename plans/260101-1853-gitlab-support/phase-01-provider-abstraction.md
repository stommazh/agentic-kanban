# Phase 1: Provider Abstraction Layer

**Effort:** 3-4 days
**Priority:** P1 (Foundational)

## Context

Research:
- [Abstraction Patterns](./research/researcher-02-abstraction-patterns.md) - Trait-based design
- [GitHub Integration](./scout/scout-01-github-integration.md) - Current implementation

Current state: GitHub logic tightly coupled to `GitHubService` without abstraction. Need trait interface allowing multiple provider implementations.

## Overview

Create provider abstraction layer enabling GitHub/GitLab implementations to coexist. Refactor existing `GitHubService` to implement new `GitProvider` trait without breaking functionality.

## Key Insights

1. **URL-based detection** most reliable - parse git remote to determine provider
2. **Unified types** reduce complexity - single `PullRequest` struct works for both PR/MR
3. **Graceful degradation** - return `NotSupported` error for provider-specific features
4. **Existing patterns** - Current `GhCli` wrapper provides template for abstraction

## Requirements

### Functional
- [ ] Define `GitProvider` trait with core operations
- [ ] Create provider detection from remote URL (SSH/HTTPS)
- [ ] Refactor `GitHubService` to implement trait
- [ ] Support self-hosted instances (custom domains)
- [ ] Maintain backward compatibility

### Non-Functional
- [ ] Zero performance regression
- [ ] Type-safe provider selection
- [ ] Extensible for future providers (Gitea, Bitbucket)

## Architecture

### Module Structure
```
crates/services/src/services/git_provider/
├── mod.rs              # Public interface, trait definition
├── types.rs            # Shared types (PrInfo, Comment, etc.)
├── detection.rs        # URL parsing, provider detection
├── github.rs           # GitHubProvider (refactored GitHubService)
└── error.rs            # ProviderError enum
```

### Core Types
```rust
// Trait definition
pub trait GitProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    async fn check_auth(&self) -> Result<(), ProviderError>;
    async fn create_merge_request(&self, req: CreateMrRequest)
        -> Result<PrInfo, ProviderError>;
    async fn get_comments(&self, repo: &RepoIdentifier, mr_number: u64)
        -> Result<Vec<UnifiedComment>, ProviderError>;
    async fn update_pr_status(&self, repo: &RepoIdentifier, number: u64)
        -> Result<PrStatus, ProviderError>;
    fn parse_remote_url(&self, url: &str) -> Option<RepoIdentifier>;
}

// Unified types
#[derive(Debug, Clone, TS)]
#[ts(export)]
pub struct RepoIdentifier {
    pub provider: ProviderType,
    pub owner: String,
    pub name: String,
    pub host: Option<String>, // For self-hosted
}

#[derive(Debug, Clone, TS)]
#[ts(export)]
pub enum ProviderType {
    GitHub,
    GitLab,
}

#[derive(Debug, Clone, TS)]
#[ts(export)]
pub struct PrInfo {
    pub number: u64,
    pub url: String,
    pub title: String,
    pub state: PrState,
}

// Error handling
pub enum ProviderError {
    NotAuthenticated,
    NotInstalled { cli_name: String },
    NotSupported { feature: String },
    ApiError { status: u16, message: String },
    ParseError(String),
}
```

### Provider Detection
```rust
pub fn detect_provider_from_remote(remote_url: &str) -> Option<ProviderType> {
    // Patterns:
    // - git@github.com:owner/repo.git -> GitHub
    // - https://github.com/owner/repo -> GitHub
    // - git@gitlab.com:group/project.git -> GitLab
    // - https://gitlab.example.com/group/project -> GitLab (self-hosted)

    if remote_url.contains("github.com") {
        Some(ProviderType::GitHub)
    } else if remote_url.contains("gitlab") {
        Some(ProviderType::GitLab)
    } else {
        None // Unknown provider
    }
}
```

## Related Code Files

**To Refactor:**
- `crates/services/src/services/github.rs` → `git_provider/github.rs`
- `crates/services/src/services/github/cli.rs` → Keep, used by GitHubProvider

**To Create:**
- `crates/services/src/services/git_provider/mod.rs`
- `crates/services/src/services/git_provider/types.rs`
- `crates/services/src/services/git_provider/detection.rs`
- `crates/services/src/services/git_provider/error.rs`

**To Update:**
- `crates/server/src/routes/task_attempts/pr.rs` - Use `Box<dyn GitProvider>` instead of `GitHubService`
- `shared/types.ts` - Add `ProviderType` export

## Implementation Steps

### Step 1: Create Module Structure (0.5d)
1. Create `git_provider/` directory
2. Define `GitProvider` trait in `mod.rs`
3. Define shared types in `types.rs` (`RepoIdentifier`, `PrInfo`, `UnifiedComment`)
4. Implement provider detection in `detection.rs`

### Step 2: Refactor GitHub Service (1.5d)
1. Move `github.rs` to `git_provider/github.rs`
2. Implement `GitProvider` trait for `GitHubService`
3. Update imports across codebase
4. Ensure all existing tests pass

### Step 3: Provider Factory (0.5d)
1. Create factory function:
   ```rust
   pub fn create_provider(
       repo_path: &Path
   ) -> Result<Box<dyn GitProvider>, ProviderError> {
       let remote_url = get_git_remote_url(repo_path)?;
       match detect_provider_from_remote(&remote_url) {
           Some(ProviderType::GitHub) => Ok(Box::new(GitHubProvider::new())),
           Some(ProviderType::GitLab) => Err(ProviderError::NotSupported {
               feature: "GitLab".into()
           }),
           None => Err(ProviderError::ParseError("Unknown provider".into())),
       }
   }
   ```

### Step 4: Update Server Routes (0.5d)
1. Update `pr.rs` to use trait object:
   ```rust
   let provider = git_provider::create_provider(&workspace_path)?;
   let pr_info = provider.create_merge_request(req).await?;
   ```
2. Handle `ProviderError::NotSupported` gracefully

### Step 5: Update TypeScript Types (0.25d)
1. Run `pnpm run generate-types`
2. Verify `ProviderType` exported to `shared/types.ts`
3. Update frontend API client if needed

### Step 6: Testing (0.5d)
1. Unit tests for provider detection (various URL formats)
2. Integration tests ensuring GitHub functionality unchanged
3. Test error handling for unknown providers

## Todo List

- [ ] Create `git_provider/` module structure
- [ ] Define `GitProvider` trait with all required methods
- [ ] Implement provider detection from git remote URL
- [ ] Create shared types (`RepoIdentifier`, `PrInfo`, `UnifiedComment`)
- [ ] Refactor `GitHubService` to implement `GitProvider`
- [ ] Update all imports in server routes
- [ ] Create provider factory function
- [ ] Update TypeScript type generation
- [ ] Write unit tests for URL detection (10+ URL patterns)
- [ ] Write integration tests for GitHub provider
- [ ] Run full test suite and fix regressions
- [ ] Update documentation comments

## Success Criteria

### Must Have
- [x] `GitProvider` trait defined with 5+ core methods
- [x] GitHub implementation passes all existing tests
- [x] Provider detection works for GitHub.com, gitlab.com, self-hosted GitLab
- [x] Zero breaking changes to existing GitHub workflows
- [x] Type-safe provider handling throughout codebase

### Nice to Have
- [ ] Metrics/logging for provider detection
- [ ] Configuration override for provider (manual selection)
- [ ] Graceful error messages for unsupported providers

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Refactor breaks existing GitHub functionality | High | Critical | Comprehensive test coverage before refactor |
| Provider detection false positives | Medium | High | Extensive URL pattern testing |
| Performance regression from trait dispatch | Low | Medium | Benchmark critical paths |
| Type export issues (Rust → TS) | Medium | Medium | Automated type generation tests |

## Security Considerations

- **URL parsing**: Validate remote URLs to prevent injection attacks
- **Provider spoofing**: Ensure detection cannot be manipulated by malicious git remotes
- **Error messages**: Don't leak sensitive paths in error responses

## Next Steps

After Phase 1 completion:
1. Begin Phase 2 (GitLab Local Support)
2. Implement `GitLabProvider` using same trait
3. Add `glab` CLI wrapper parallel to `gh` CLI

## Unresolved Questions

1. Should we cache provider detection per workspace to avoid repeated git calls?
2. How to handle repos with multiple remotes (origin vs upstream)?
3. Should provider type be stored in database or always detected dynamically?
4. What to do with ambiguous URLs (custom domains that could be Gitea/GitLab)?
