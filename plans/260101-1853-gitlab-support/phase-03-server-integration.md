# Phase 3: Server Integration

**Effort:** 2-3 days
**Priority:** P1 (Integration)
**Prerequisites:** Phase 1 & 2 complete

## Context

Research:
- [GitHub Integration](./scout/scout-01-github-integration.md) - Current server routes and frontend components

Phase 1 created provider abstraction, Phase 2 implemented GitLab provider. Now integrate both into server routes and frontend.

## Overview

Update server API endpoints to work with both GitHub and GitLab providers seamlessly. Implement provider-agnostic routes that auto-detect provider from workspace. Update frontend to display correct terminology (PR vs MR) and handle both providers.

## Key Insights

1. **No new endpoints** - Reuse existing `/github-pr` routes, rename to `/merge-request` (backwards compatible)
2. **Auto-detection** - Provider determined from git remote, no user selection needed
3. **Terminology mapping** - Frontend shows "Pull Request" (GitHub) or "Merge Request" (GitLab) dynamically
4. **Type unification** - Single `PrInfo` struct works for both, frontend doesn't need provider-specific types
5. **Error handling** - Map provider-specific errors to unified `CreatePrError` enum

## Requirements

### Functional
- [ ] Unified `/merge-request` endpoint (replaces `/github-pr`)
- [ ] Provider detection in PR creation flow
- [ ] Frontend detects provider and shows correct terminology
- [ ] Comment fetching works for both providers
- [ ] CLI setup routes for both `gh` and `glab`
- [ ] Backwards compatibility with existing GitHub workflows

### Non-Functional
- [ ] Zero breaking changes to existing API contracts
- [ ] Provider detection < 100ms overhead
- [ ] Clear error messages for unsupported providers
- [ ] Frontend bundle size increase < 50KB

## Architecture

### Server Route Refactoring
```rust
// Before (GitHub-specific)
POST /api/task-attempts/{id}/github-pr

// After (provider-agnostic, with backwards compat)
POST /api/task-attempts/{id}/github-pr  // Alias for backwards compat
POST /api/task-attempts/{id}/merge-request  // New unified endpoint

// Implementation
pub async fn create_merge_request(
    Path(attempt_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<CreatePrRequest>,
) -> Result<Json<PrInfo>, AppError> {
    let workspace = state.db.get_workspace_for_attempt(&attempt_id).await?;
    let workspace_path = &workspace.path;

    // Auto-detect provider from git remote
    let provider = git_provider::create_provider(workspace_path)?;

    // Call provider-agnostic method
    let pr_info = provider.create_merge_request(CreateMrRequest {
        title: req.title,
        body: req.body,
        target_branch: req.target_branch,
        source_branch: req.source_branch,
        draft: req.draft,
        repo_id: req.repo_id,
    }).await?;

    // Store in database (same schema works for both)
    state.db.store_pr_info(&attempt_id, &pr_info).await?;

    Ok(Json(pr_info))
}
```

### Frontend Provider Detection
```typescript
// Detect provider from stored PR info or workspace metadata
export function useGitProvider(workspaceId: string): {
  provider: ProviderType;
  terminology: {
    pr: string;      // "Pull Request" or "Merge Request"
    prShort: string; // "PR" or "MR"
  };
} {
  const { data: workspace } = useWorkspace(workspaceId);

  // Provider stored in workspace metadata (detected on backend)
  const provider = workspace?.git_provider ?? 'github';

  const terminology = {
    pr: provider === 'gitlab' ? 'Merge Request' : 'Pull Request',
    prShort: provider === 'gitlab' ? 'MR' : 'PR',
  };

  return { provider, terminology };
}
```

### Component Updates
```tsx
// Before
<DialogTitle>Create Pull Request</DialogTitle>

// After
export function CreatePRDialog({ attemptId }: Props) {
  const { provider, terminology } = useGitProvider(attemptId);

  return (
    <Dialog>
      <DialogTitle>Create {terminology.pr}</DialogTitle>
      <DialogDescription>
        Push your changes and create a {terminology.prShort}
      </DialogDescription>
      {/* ... */}
    </Dialog>
  );
}
```

## Related Code Files

**To Update (Server):**
- `crates/server/src/routes/task_attempts/pr.rs` - Rename to `mr.rs`, add provider detection
- `crates/server/src/routes/task_attempts/gh_cli_setup.rs` - Add `setup_gitlab_cli()` route
- `crates/server/src/routes/task_attempts.rs` - Register new routes
- `crates/db/src/models/workspace.rs` - Add `git_provider` field (optional cache)

**To Update (Frontend):**
- `frontend/src/components/dialogs/tasks/CreatePRDialog.tsx` - Use dynamic terminology
- `frontend/src/components/dialogs/tasks/GitHubCommentsDialog.tsx` - Rename to `MergeRequestCommentsDialog.tsx`
- `frontend/src/components/dialogs/auth/GhCliSetupDialog.tsx` - Create parallel `GlabCliSetupDialog.tsx`
- `frontend/src/components/ui/github-comment-card.tsx` - Rename to `mr-comment-card.tsx`
- `frontend/src/hooks/usePrComments.ts` - Rename to `useMrComments.ts`
- `frontend/src/lib/api.ts` - Update endpoint paths

**To Create:**
- `frontend/src/hooks/useGitProvider.ts` - Provider detection hook
- `frontend/src/components/dialogs/auth/GlabCliSetupDialog.tsx` - GitLab CLI setup
- `crates/server/src/routes/task_attempts/mr.rs` - Unified MR routes

## Implementation Steps

### Step 1: Database Schema (0.5d)
1. Add `git_provider` field to `workspaces` table (optional cache):
   ```sql
   ALTER TABLE workspaces ADD COLUMN git_provider TEXT;
   -- Values: 'github', 'gitlab', NULL (auto-detect)
   ```
2. Populate existing workspaces with 'github' (backwards compat)
3. Update `workspace_repos` to store provider per repo

### Step 2: Server Route Refactoring (1d)
1. Rename `pr.rs` to `mr.rs` (but keep backwards-compatible routes)
2. Update `create_pr()` to use `git_provider::create_provider()`
3. Add provider detection at route entry point
4. Update error handling to map provider errors
5. Add route: `GET /api/workspaces/{id}/git-provider` (detection endpoint)
6. Keep `/github-pr` as alias to new `/merge-request` endpoint

### Step 3: CLI Setup Routes (0.5d)
1. Refactor `gh_cli_setup.rs` to support both providers
2. Add `setup_gitlab_cli()` route
3. Detect Homebrew, run `brew install glab`
4. Guide through `glab auth login`

### Step 4: Frontend Provider Detection (0.5d)
1. Create `useGitProvider()` hook
2. Fetch provider from backend or detect from PR URL
3. Store provider in React context for easy access

### Step 5: Frontend Component Updates (1d)
1. Update `CreatePRDialog.tsx` to use dynamic terminology
2. Create `GlabCliSetupDialog.tsx` (parallel to `GhCliSetupDialog.tsx`)
3. Rename `GitHubCommentsDialog.tsx` to `MergeRequestCommentsDialog.tsx`
4. Update all "Pull Request" strings to use `terminology.pr`
5. Update button labels: "Create PR" → "Create {terminology.prShort}"

### Step 6: API Client Updates (0.5d)
1. Update `frontend/src/lib/api.ts` to use new endpoints
2. Keep backwards-compatible calls to `/github-pr` (fallback)
3. Add `getGitProvider(workspaceId)` API method

### Step 7: TypeScript Type Generation (0.25d)
1. Run `pnpm run generate-types`
2. Verify `ProviderType` exported correctly
3. Update frontend imports

### Step 8: Testing (0.75d)
1. Integration tests: Create MR with GitLab provider
2. Regression tests: Existing GitHub workflows still work
3. Frontend tests: Provider detection and terminology display
4. Test CLI setup for both providers

## Todo List

- [ ] Add `git_provider` field to `workspaces` table
- [ ] Rename `pr.rs` to `mr.rs` with backwards-compatible routes
- [ ] Implement provider auto-detection in route handlers
- [ ] Create unified `/merge-request` endpoint
- [ ] Add `GET /api/workspaces/{id}/git-provider` route
- [ ] Create `setup_gitlab_cli()` route
- [ ] Create `useGitProvider()` hook in frontend
- [ ] Update `CreatePRDialog.tsx` with dynamic terminology
- [ ] Create `GlabCliSetupDialog.tsx` component
- [ ] Rename `GitHubCommentsDialog.tsx` to `MergeRequestCommentsDialog.tsx`
- [ ] Update all frontend strings to use provider-aware terminology
- [ ] Update API client to use new endpoints
- [ ] Run TypeScript type generation
- [ ] Write integration tests for GitLab MR creation
- [ ] Write regression tests for GitHub PR creation
- [ ] Test provider detection with various git remotes
- [ ] Update API documentation with new endpoints

## Success Criteria

### Must Have
- [x] Unified API endpoint works for both GitHub and GitLab
- [x] Provider auto-detected from git remote (< 100ms overhead)
- [x] Frontend displays correct terminology (PR vs MR)
- [x] Existing GitHub workflows unchanged (backwards compatibility)
- [x] CLI setup works for both `gh` and `glab`
- [x] Error messages clear for unsupported providers

### Nice to Have
- [ ] Provider override in workspace settings
- [ ] Provider badge in task list (GitHub logo vs GitLab logo)
- [ ] Analytics tracking provider usage
- [ ] Admin panel showing provider distribution

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing GitHub integrations | High | Critical | Extensive regression testing, backwards-compatible routes |
| Frontend bundle size increase | Medium | Low | Code-split provider-specific components |
| Provider detection false positives | Medium | High | Comprehensive URL pattern tests |
| Database migration failures | Low | High | Test migration on dev DB first, add rollback script |
| TypeScript type export issues | Medium | Medium | Automated type generation in CI |

## Security Considerations

- **Provider detection**: Validate git remote URL format to prevent injection
- **Error exposure**: Don't leak workspace paths in provider detection errors
- **CLI execution**: Sanitize inputs to `brew install` commands
- **Token handling**: Both providers use secure token storage from Phase 2

## Next Steps

After Phase 3 completion:
1. Begin Phase 4 (Testing & Documentation)
2. Comprehensive test coverage for both providers
3. User documentation for GitLab setup and usage

## Unresolved Questions

1. **Route naming**: Keep `/github-pr` as primary or deprecate in favor of `/merge-request`?
2. **Provider caching**: Store in database vs detect on every request?
3. **Multi-remote repos**: How to handle workspace with both GitHub and GitLab remotes?
4. **Provider icons**: Should we show GitHub/GitLab logo in UI?
5. **Error fallback**: If provider detection fails, default to GitHub or return error?
