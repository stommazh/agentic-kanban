---
title: "GitLab Support Integration"
description: "Add GitLab support alongside GitHub with auto-detection, MR operations, and optional remote webhooks"
status: pending
priority: P1
effort: 12-15d
branch: main
tags: [gitlab, git-provider, abstraction, multi-provider, integration]
created: 2026-01-01
---

# GitLab Support Implementation Plan

## Context

Add GitLab support (including self-managed instances) to Vibe-Kanban alongside existing GitHub integration. Users should experience seamless provider auto-detection from git remote URL without manual configuration.

## Research Foundation

- **GitLab API Analysis**: [researcher-01-gitlab-api.md](./research/researcher-01-gitlab-api.md)
- **Abstraction Patterns**: [researcher-02-abstraction-patterns.md](./research/researcher-02-abstraction-patterns.md)
- **Current GitHub Code**: [scout-01-github-integration.md](./scout/scout-01-github-integration.md)

## Architecture Approach

**Provider Trait Abstraction** - unified interface for both GitHub and GitLab:

```rust
pub trait GitProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;
    async fn create_merge_request(&self, req: CreateMrRequest) -> Result<PrInfo>;
    async fn get_comments(&self, repo_id: &str, mr_number: u64) -> Result<Vec<Comment>>;
    async fn check_auth(&self) -> Result<()>;
    fn parse_remote_url(&self, url: &str) -> Option<RepoIdentifier>;
}
```

## Implementation Phases

### Phase 1: Provider Abstraction Layer (3-4d)
Create trait-based git provider abstraction, refactor GitHub to implement it, add URL-based provider detection.

**Deliverables**: `GitProvider` trait, `GitHubService` refactored, provider detection from remote URL

### Phase 2: GitLab Local Support (4-5d)
Implement GitLab MR operations via `glab` CLI with REST API fallback, support self-hosted instances.

**Deliverables**: `GitLabService` implementation, MR creation/comment fetching, CLI setup helpers

### Phase 3: Server Integration (2-3d)
Unified server endpoints working with both providers, frontend updates for PR/MR terminology.

**Deliverables**: Provider-agnostic API routes, frontend dialogs supporting both providers

### Phase 4: Testing & Documentation (2-3d)
Comprehensive tests for provider detection and GitLab operations, documentation for self-hosted setup.

**Deliverables**: Unit/integration tests, user documentation, configuration guides

### Phase 5: Remote Integration (OPTIONAL - 5-7d)
GitLab OAuth provider, webhook handlers, database schema for group tokens.

**Deliverables**: OAuth flow, webhook processing, automated MR reviews for GitLab

## Success Criteria

1. ✅ Auto-detect GitHub vs GitLab from git remote URL
2. ✅ Create MR via `glab` CLI or REST API
3. ✅ Fetch MR comments/notes
4. ✅ Support self-hosted GitLab instances
5. ✅ Frontend works seamlessly with both providers
6. ✅ Zero breaking changes to existing GitHub functionality

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `glab` CLI less mature than `gh` | High | REST API fallback for all operations |
| Self-hosted GitLab version compatibility | Medium | Document minimum supported version (v13+) |
| Token expiry handling (GitLab tokens expire) | Medium | Implement refresh logic in Phase 5 |
| Breaking existing GitHub workflows | High | Comprehensive regression tests |

## Out of Scope

- Bitbucket/Gitea support (future consideration)
- Migration of existing GitHub data
- GraphQL API support (REST sufficient per YAGNI)
