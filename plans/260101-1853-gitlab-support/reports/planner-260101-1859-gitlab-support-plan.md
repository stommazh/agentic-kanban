# GitLab Support Implementation Plan - Summary

**Date:** 2026-01-01 18:59
**Plan Location:** `/Users/khangle/Documents/repo/agentic-kanban/plans/260101-1853-gitlab-support/`
**Status:** Ready for implementation

## Executive Summary

Created comprehensive 5-phase implementation plan for adding GitLab support to Vibe-Kanban alongside existing GitHub integration. Architecture uses provider trait abstraction enabling seamless auto-detection from git remote URL without user configuration.

## Plan Structure

### Main Plan
**File:** `plan.md`
**Effort:** 12-15 days (Phases 1-4), +5-7d optional (Phase 5)
**Priority:** P1 (foundational feature)

### Phase Files Created

1. **phase-01-provider-abstraction.md** (3-4d)
   - Create `GitProvider` trait with unified interface
   - Refactor `GitHubService` to implement trait
   - URL-based provider detection (github.com vs gitlab.com)
   - Support self-hosted instances

2. **phase-02-gitlab-local-support.md** (4-5d)
   - Implement `GitLabProvider` with `glab` CLI wrapper
   - REST API fallback for operations
   - Personal Access Token authentication
   - Self-hosted GitLab base URL configuration

3. **phase-03-server-integration.md** (2-3d)
   - Unified `/merge-request` API endpoint (backwards compatible)
   - Frontend provider detection and terminology (PR vs MR)
   - CLI setup routes for both `gh` and `glab`
   - Zero breaking changes to GitHub workflows

4. **phase-04-testing-documentation.md** (2-3d)
   - Unit tests for provider detection (20+ URL patterns)
   - Integration tests for GitLab MR operations
   - Regression tests for GitHub workflows
   - User guides: GitLab setup, self-hosted config, troubleshooting

5. **phase-05-remote-integration.md** (5-7d, OPTIONAL)
   - GitLab OAuth provider with token refresh
   - Webhook handlers for automated MR reviews
   - Database schema for group integrations
   - Multi-instance support (gitlab.com + self-hosted)

## Key Architecture Decisions

### 1. Provider Trait Abstraction
```rust
pub trait GitProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    async fn create_merge_request(&self, req: CreateMrRequest) -> Result<PrInfo>;
    async fn get_comments(&self, repo: &RepoIdentifier, mr_number: u64) -> Result<Vec<Comment>>;
    async fn check_auth(&self) -> Result<()>;
    fn parse_remote_url(&self, url: &str) -> Option<RepoIdentifier>;
}
```

**Rationale:** Unified interface enables adding providers (Gitea, Bitbucket) without touching business logic.

### 2. URL-Based Auto-Detection
```rust
fn detect_provider_from_remote(url: &str) -> Option<ProviderType> {
    if url.contains("github.com") => GitHub
    else if url.contains("gitlab") => GitLab
    else => None
}
```

**Rationale:** Zero config for users, works instantly with existing repos.

### 3. CLI with REST API Fallback
- Primary: `glab` CLI (when available, authenticated)
- Fallback: GitLab REST API v4 (using `GITLAB_TOKEN` env var)

**Rationale:** `glab` less mature than `gh`, REST ensures reliability.

### 4. Unified Types
Single `PrInfo` struct works for both GitHub PRs and GitLab MRs.

**Rationale:** Frontend doesn't need provider-specific types, terminology only difference.

## Implementation Priorities

### Phase 1-4: Core GitLab Support (MUST HAVE)
- **Scope:** Local MR operations, provider abstraction, testing
- **Effort:** 11-15 days
- **Deliverables:** GitLab MR creation/comments working locally

### Phase 5: Remote Integration (OPTIONAL)
- **Scope:** OAuth, webhooks, automated reviews
- **Effort:** +5-7 days
- **Trigger:** After Phase 4 success + user demand

## Research Foundation

Plan built on 3 research reports:

1. **researcher-01-gitlab-api.md**
   - GitLab API v4 capabilities, rate limits (2000 req/min)
   - Auth: Personal Access Tokens (primary), OAuth2 (Phase 5)
   - Self-hosted: Identical API structure, only base URL differs

2. **researcher-02-abstraction-patterns.md**
   - Provider trait pattern from existing tools (Gitea, Forgejo)
   - URL-based detection best practices
   - Error handling strategies (NotSupported, NotAuthenticated)

3. **scout-01-github-integration.md**
   - Current GitHub code spans 50+ files (services, routes, frontend)
   - `GhCli` wrapper pattern to replicate for `GlabCli`
   - 3 integration layers: local CLI, remote webhooks, OAuth

## Success Criteria

### Technical
- ✅ Auto-detect provider from git remote (< 100ms)
- ✅ Create MR via CLI or API successfully
- ✅ Fetch MR comments/notes
- ✅ Support self-hosted GitLab (custom base URL)
- ✅ Zero breaking changes to GitHub functionality
- ✅ Test coverage > 80% for new code

### User Experience
- ✅ No provider selection needed (transparent auto-detection)
- ✅ Frontend shows "Merge Request" for GitLab, "Pull Request" for GitHub
- ✅ CLI setup guides for both `gh` and `glab`
- ✅ Clear error messages for auth/config issues

## Risk Mitigation

| Risk | Mitigation Strategy |
|------|---------------------|
| `glab` CLI unreliable | REST API fallback for all operations |
| Breaking GitHub workflows | Comprehensive regression tests, backwards-compatible routes |
| Self-hosted version incompatibility | Document min version (v13+), test against v13/v14/v15 |
| Provider detection false positives | 20+ URL pattern tests in Phase 4 |

## Dependencies & Prerequisites

### Technical
- Rust 1.70+ (trait-based design)
- `glab` CLI (optional, REST API fallback)
- GitLab API v4 (self-hosted v13+)

### Environment Variables (New)
```bash
# GitLab Personal Access Token (required for API operations)
GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"

# Self-hosted GitLab base URL (optional, default: https://gitlab.com/api/v4)
GITLAB_BASE_URL="https://gitlab.example.com/api/v4"
```

### Phase 5 Only (Optional)
```bash
# GitLab OAuth Application (for remote integration)
GITLAB_APPLICATION_ID="..."
GITLAB_APPLICATION_SECRET="..."
GITLAB_WEBHOOK_SECRET="..."
```

## Out of Scope

- Bitbucket/Gitea support (future consideration)
- GraphQL API (REST sufficient per YAGNI)
- Migration of existing GitHub data
- GitLab CI/CD integration (separate feature)

## Next Actions

1. **Review & Approval:** Share plan with team, gather feedback
2. **Phase 1 Kickoff:** Assign developer to provider abstraction
3. **Test Setup:** Create GitLab test accounts for CI
4. **Documentation:** Prepare release notes for users

## Unresolved Questions

1. **Token management:** Support multiple GitLab tokens (per self-hosted instance)?
2. **Route naming:** Keep `/github-pr` or fully migrate to `/merge-request`?
3. **Provider caching:** Store detected provider in DB or always detect from git remote?
4. **Multi-remote repos:** How to handle workspace with both GitHub and GitLab remotes?
5. **CI infrastructure:** Test against real GitLab.com or only mocks?

## Files Delivered

```
plans/260101-1853-gitlab-support/
├── plan.md                            # Main plan with frontmatter
├── phase-01-provider-abstraction.md   # Detailed Phase 1 spec
├── phase-02-gitlab-local-support.md   # Detailed Phase 2 spec
├── phase-03-server-integration.md     # Detailed Phase 3 spec
├── phase-04-testing-documentation.md  # Detailed Phase 4 spec
├── phase-05-remote-integration.md     # Optional Phase 5 spec
└── reports/
    └── planner-260101-1859-gitlab-support-plan.md  # This summary
```

---

**Plan Status:** ✅ Ready for implementation
**Recommended Start:** Phase 1 (Provider Abstraction)
**Estimated Completion (Phases 1-4):** 2-3 weeks (with 1 developer)
