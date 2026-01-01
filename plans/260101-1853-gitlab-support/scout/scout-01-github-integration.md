# GitHub Integration Scout Report

**Date:** 2026-01-01  
**Purpose:** Comprehensive analysis of GitHub integration code for GitLab support implementation

## Executive Summary

Vibe-Kanban has deep GitHub integration across multiple layers:
1. **Local GitHub CLI (gh)** - Used for PR operations in local workspace
2. **GitHub App (remote)** - OAuth + webhook-based PR review automation
3. **OAuth Provider (remote)** - User authentication via GitHub
4. **Frontend Components** - GitHub-specific UI dialogs and comment rendering

**Key Finding:** GitHub integration spans ~50 files with 3 distinct integration patterns that will need parallel GitLab equivalents.

---

## 1. Backend Architecture

### 1.1 Local GitHub Service (Rust)

**Primary Module:** `crates/services/src/services/github.rs` (454 lines)

**Purpose:** Local GitHub CLI operations for workspaces

**Key Components:**
- `GitHubService` - Main service struct using GitHub CLI
- `GitHubRepoInfo` - Parses owner/repo from remote URLs
- `CreatePrRequest` - PR creation parameters
- `UnifiedPrComment` - Enum for general vs review comments

**Core Operations:**
```rust
- check_token() - Verify gh CLI auth
- create_pr() - Create PR via gh CLI
- update_pr_status() - Get PR status/merge info
- list_all_prs_for_branch() - Find PRs for branch
- get_pr_comments() - Fetch comments (general + review)
```

**CLI Wrapper:** `crates/services/src/services/github/cli.rs` (392 lines)
- `GhCli` struct wraps `gh` executable
- Executes commands via `std::process::Command`
- Parses JSON output from `gh pr view --json`, `gh api`
- Error handling: `GhCliError` enum (NotAvailable, AuthFailed, CommandFailed, UnexpectedOutput)

**GitLab Port Requirements:**
- Need `gitlab` CLI or REST API client
- Same operations: create_pr → create_merge_request
- Comment fetching needs API (no CLI equivalent as rich as gh)

---

### 1.2 GitHub App Service (Remote Rust)

**Location:** `crates/remote/src/github_app/`

**Purpose:** Server-side GitHub App integration for automated PR reviews

**Modules:**
- `service.rs` (454 lines) - Core GitHub App API client
- `jwt.rs` - JWT generation for GitHub App auth
- `webhook.rs` - HMAC signature verification
- `pr_review.rs` - Orchestrate PR review workflow
- `mod.rs` - Public interface

**GitHubAppService Key Methods:**
```rust
- get_installation_token(installation_id) - Get access token for installation
- get_installation(installation_id) - Fetch installation details
- list_installation_repos(installation_id) - List accessible repos
- post_pr_comment(owner, repo, pr_number, body) - Post comment
- clone_repo(installation_id, owner, repo, head_sha) - Clone with token auth
- get_merge_base(repo_dir, base_ref) - Calculate diff base
- get_pr_details(installation_id, owner, repo, pr_number) - Fetch PR metadata
```

**Database Layer:** `crates/remote/src/db/github_app.rs` (572 lines)

**Tables:**
- `github_app_installations` - Track GitHub App installs per organization
- `github_app_repositories` - Repos accessible via installation (with review_enabled flag)
- `github_app_pending_installations` - Temporary state during OAuth flow

**Key Queries:**
- CRUD for installations (by github_id, org_id, account_login)
- Sync repositories (add/remove based on GitHub webhook events)
- Toggle review_enabled per repo or bulk

**GitLab Port Requirements:**
- GitLab has different app model - may need OAuth Apps or Project Access Tokens
- Webhook signature uses different algorithm (GitLab uses X-Gitlab-Token header)
- No JWT - GitLab uses personal/project access tokens

---

### 1.3 GitHub App Routes (Remote)

**Location:** `crates/remote/src/routes/github_app.rs` (1158 lines)

**Public Routes:**
- `POST /v1/github/webhook` - Receive GitHub webhooks (installation, PR, comments)
- `GET /v1/github/app/callback` - OAuth callback after app installation

**Protected Routes (require auth):**
- `GET /organizations/{org_id}/github-app/install-url` - Generate installation URL
- `GET /organizations/{org_id}/github-app/status` - Get installation status
- `DELETE /organizations/{org_id}/github-app` - Remove installation locally
- `GET /organizations/{org_id}/github-app/repositories` - Fetch repos from API
- `PATCH /organizations/{org_id}/github-app/repositories/{repo_id}/review-enabled` - Toggle review
- `PATCH /organizations/{org_id}/github-app/repositories/review-enabled` - Bulk toggle
- `POST /v1/debug/pr-review/trigger` - Manual PR review trigger (debug)

**Webhook Event Handlers:**
- `installation` - created, deleted, suspend, unsuspend
- `installation_repositories` - added, removed
- `pull_request` - opened (triggers review if enabled)
- `issue_comment` - created (checks for `!reviewfast` trigger)

**GitLab Port Requirements:**
- GitLab webhooks: Push Hook, Merge Request Hook, Note Hook (comments)
- URL patterns: `/gitlab/webhook`, `/gitlab/oauth/callback`
- Group-level (not org-level) access tokens

---

### 1.4 Review Service

**Location:** `crates/review/src/github.rs` (230 lines)

**Purpose:** CLI tool for PR review operations

**Functions:**
- `parse_pr_url(url)` - Extract owner/repo/number from GitHub URL
- `get_pr_info(owner, repo, pr_number)` - Fetch PR details via gh CLI
- `clone_repo(owner, repo, target_dir)` - Clone using gh CLI
- `checkout_commit(commit_sha, repo_dir)` - Checkout specific commit

**GitLab Equivalent:**
- Parse: `gitlab.com/group/project/-/merge_requests/123`
- CLI: `glab mr view` (glab CLI exists but less mature than gh)
- REST API more reliable for GitLab

---

### 1.5 Server Routes (Local PR Operations)

**Location:** `crates/server/src/routes/task_attempts/pr.rs` (522 lines)

**Endpoints:**
- Create PR (`CreateGitHubPrRequest`)
- Attach existing PR to workspace
- Get PR comments (`GetPrCommentsQuery`)

**Key Logic:**
- Validates target branch exists on remote
- Pushes branch to GitHub
- Creates PR via `GitHubService`
- Stores PR info in `merges` table
- Auto-generates PR description via coding agent (optional)
- Opens PR in browser

**Auto-Description Feature:**
- Uses `DEFAULT_PR_DESCRIPTION_PROMPT` template
- Triggers coding agent follow-up to update PR with better title/body
- Prompt includes: `{pr_number}`, `{pr_url}` placeholders

**GitLab Port Requirements:**
- Same flow but call GitLab API/CLI
- Merge request instead of PR
- Note: GitLab doesn't have "draft" concept (uses WIP: prefix in title)

---

## 2. OAuth Provider

**Location:** `crates/remote/src/auth/provider.rs` (693 lines)

**GitHubOAuthProvider:**
- Client ID + Secret
- Scopes: `read:user`, `user:email`
- Token exchange: `https://github.com/login/oauth/access_token`
- User fetch: `https://api.github.com/user`, `https://api.github.com/user/emails`
- Token validation: `https://api.github.com/rate_limit` (GitHub tokens don't expire)

**GitLab Equivalent:**
- Scopes: `read_user`, `email`, `api`
- Token endpoint: `https://gitlab.com/oauth/token`
- User endpoint: `https://gitlab.com/api/v4/user`
- Token refresh: GitLab tokens DO expire (need refresh_token handling)

---

## 3. Database Schema

### 3.1 GitHub App Tables

**Migration:** `crates/remote/migrations/20251215000000_github_app_installations.sql`

**Tables:**
```sql
CREATE TABLE github_app_installations (
    id UUID PRIMARY KEY,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    github_installation_id BIGINT UNIQUE NOT NULL,
    github_account_login TEXT NOT NULL,
    github_account_type TEXT NOT NULL, -- "Organization" or "User"
    repository_selection TEXT NOT NULL, -- "all" or "selected"
    installed_by_user_id UUID REFERENCES users(id),
    suspended_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE github_app_repositories (
    id UUID PRIMARY KEY,
    installation_id UUID REFERENCES github_app_installations(id) ON DELETE CASCADE,
    github_repo_id BIGINT NOT NULL,
    repo_full_name TEXT NOT NULL,
    review_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(installation_id, github_repo_id)
);

CREATE TABLE github_app_pending_installations (
    id UUID PRIMARY KEY,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    state_token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**GitLab Port:**
- Rename tables: `gitlab_group_tokens`, `gitlab_projects`, `gitlab_pending_oauth`
- Add token expiry tracking (GitLab tokens expire)
- `gitlab_group_id` instead of `github_installation_id`

---

## 4. Frontend Components

### 4.1 GitHub CLI Setup Dialog

**Location:** `frontend/src/components/dialogs/auth/GhCliSetupDialog.tsx` (267 lines)

**Purpose:** Guided setup for GitHub CLI on macOS

**Features:**
- Detects Homebrew availability
- Runs `brew install gh` if needed
- Guides user through `gh auth login`
- Error handling: `BREW_MISSING`, `SETUP_HELPER_NOT_SUPPORTED`, `OTHER`

**API Call:** `attemptsApi.setupGhCli(attemptId)`

**GitLab Port:**
- `GlabCliSetupDialog.tsx`
- `brew install glab` or manual instructions
- `glab auth login` (uses OAuth by default)

---

### 4.2 Create PR Dialog

**Location:** `frontend/src/components/dialogs/tasks/CreatePRDialog.tsx` (386 lines)

**Features:**
- PR title/body input
- Base branch selector
- Draft PR checkbox
- Auto-generate description checkbox
- Detects GitHub CLI auth failures → shows setup dialog
- Validates target branch exists

**API Call:** `attemptsApi.createPR(attemptId, { title, body, target_branch, draft, auto_generate_description, repo_id })`

**GitLab Port:**
- Same UI flow
- Remove "draft" checkbox (use WIP: prefix instead)
- Call `attemptsApi.createMergeRequest()`

---

### 4.3 GitHub Comments Dialog

**Location:** `frontend/src/components/dialogs/tasks/GitHubCommentsDialog.tsx` (236 lines)

**Features:**
- Fetch PR comments via `usePrComments` hook
- Display unified timeline (general + review comments)
- Select comments to import into context
- Shows error if no PR attached or CLI not setup

**API Call:** `GET /api/task-attempts/{attemptId}/pr-comments?repo_id={repoId}`

**GitLab Port:**
- `GitLabCommentsDialog.tsx`
- Fetch merge request notes (comments)
- GitLab has simpler comment model (no "review comment" vs "general comment" distinction)

---

### 4.4 GitHub Comment Card

**Location:** `frontend/src/components/ui/github-comment-card.tsx`

**Features:**
- Display comment author, body, timestamp
- Link to GitHub URL
- Show file path/line for review comments
- Render diff hunk

**GitLab Port:**
- `gitlab-comment-card.tsx`
- Same structure but adjust styling to match GitLab's design
- GitLab note URLs: `gitlab.com/group/project/-/merge_requests/123#note_456`

---

### 4.5 WYSIWYG Editor Integration

**Location:** `frontend/src/components/ui/wysiwyg/nodes/github-comment-node.tsx`

**Purpose:** Inline rendering of imported GitHub comments in task description editor

**GitLab Port:**
- `gitlab-comment-node.tsx`
- Same Tiptap extension pattern

---

## 5. Shared Types (TypeScript)

**Location:** `shared/types.ts` (auto-generated from Rust)

**GitHub-specific types:**
```typescript
export type UnifiedPrComment = 
  | { comment_type: "general"; id: string; author: string; ... }
  | { comment_type: "review"; id: number; author: string; path: string; line?: number; diff_hunk: string; ... };

export type PrCommentAuthor = { login: string };
export type ReviewCommentUser = { login: string };

export interface GhCliSetupError = 
  | "BREW_MISSING"
  | "SETUP_HELPER_NOT_SUPPORTED"
  | { OTHER: { message: string } };

export interface CreatePrError {
  type: "github_cli_not_installed" | "github_cli_not_logged_in" | "git_cli_not_installed" | "git_cli_not_logged_in" | "target_branch_not_found";
  branch?: string; // for target_branch_not_found
}

export interface GetPrCommentsError {
  type: "no_pr_attached" | "github_cli_not_installed" | "github_cli_not_logged_in";
}
```

**GitLab Equivalents:**
```typescript
export type GitLabMrComment = { ... }; // Simpler - no general vs review distinction
export type GlabCliSetupError = ...; // Similar error types
export interface CreateMrError { ... }; // Merge request errors
```

---

## 6. API Endpoints Summary

### 6.1 Local Server Endpoints (crates/server)

**GitHub PR Operations:**
- `POST /api/task-attempts/{id}/github-pr` - Create PR
- `POST /api/task-attempts/{id}/attach-pr` - Attach existing PR
- `GET /api/task-attempts/{id}/pr-comments?repo_id={id}` - Fetch comments
- `POST /api/task-attempts/{id}/setup-gh-cli` - Setup GitHub CLI (macOS only)

**GitLab Equivalents:**
- `POST /api/task-attempts/{id}/gitlab-mr` - Create MR
- `POST /api/task-attempts/{id}/attach-mr` - Attach existing MR
- `GET /api/task-attempts/{id}/mr-notes?repo_id={id}` - Fetch notes
- `POST /api/task-attempts/{id}/setup-glab-cli` - Setup GitLab CLI

---

### 6.2 Remote Server Endpoints (crates/remote)

**GitHub App:**
- `POST /v1/github/webhook` - Webhook receiver
- `GET /v1/github/app/callback` - OAuth callback
- `GET /v1/organizations/{org_id}/github-app/install-url` - Get install URL
- `GET /v1/organizations/{org_id}/github-app/status` - Get installation status
- `DELETE /v1/organizations/{org_id}/github-app` - Remove installation
- `GET /v1/organizations/{org_id}/github-app/repositories` - List repos
- `PATCH /v1/organizations/{org_id}/github-app/repositories/{repo_id}/review-enabled` - Toggle review
- `POST /v1/debug/pr-review/trigger` - Manual review trigger

**GitLab Equivalents:**
- `POST /v1/gitlab/webhook` - Webhook receiver
- `GET /v1/gitlab/oauth/callback` - OAuth callback
- `GET /v1/groups/{group_id}/gitlab/access-token` - Setup group access token
- `GET /v1/groups/{group_id}/gitlab/status` - Get integration status
- `DELETE /v1/groups/{group_id}/gitlab` - Remove integration
- `GET /v1/groups/{group_id}/gitlab/projects` - List projects
- `PATCH /v1/groups/{group_id}/gitlab/projects/{project_id}/review-enabled` - Toggle review
- `POST /v1/debug/mr-review/trigger` - Manual review trigger

---

## 7. Configuration

### 7.1 GitHub App Config

**Location:** `crates/remote/src/config.rs`

```rust
pub struct GitHubAppConfig {
    pub app_id: i64,
    pub app_slug: String,
    pub private_key: SecretString,
    pub webhook_secret: SecretString,
}
```

**Environment Variables:**
- `GITHUB_APP_ID`
- `GITHUB_APP_SLUG`
- `GITHUB_APP_PRIVATE_KEY_PATH` or `GITHUB_APP_PRIVATE_KEY`
- `GITHUB_APP_WEBHOOK_SECRET`

**GitLab Config:**
```rust
pub struct GitLabConfig {
    pub application_id: String, // OAuth application ID
    pub application_secret: SecretString,
    pub webhook_secret: SecretString,
}
```

**Environment Variables:**
- `GITLAB_APPLICATION_ID`
- `GITLAB_APPLICATION_SECRET`
- `GITLAB_WEBHOOK_SECRET`

---

### 7.2 OAuth Provider Config

**GitHub:**
```rust
GitHubOAuthProvider::new(client_id, client_secret)
```

**GitLab:**
```rust
GitLabOAuthProvider::new(client_id, client_secret)
```

---

## 8. Testing Files

**Rust Tests:**
- `crates/services/tests/git_workflow.rs` - Integration tests for GitHub PR flow
- Unit tests in `github/cli.rs` - Test PR URL parsing, JSON parsing

**Frontend:**
- No dedicated test files found (manual testing likely)

**GitLab Port:**
- Add parallel tests: `git_workflow_gitlab.rs`
- Test GitLab MR URL parsing

---

## 9. Key Dependencies

**Rust:**
- `reqwest` - HTTP client for API calls
- `tokio` - Async runtime
- `serde_json` - JSON parsing
- `jsonwebtoken` - JWT generation (GitHub App)
- `hmac`, `sha2` - HMAC-SHA256 for webhook verification
- `git2` - libgit2 bindings for git operations

**GitLab Alternatives:**
- No JWT needed (use OAuth access tokens)
- HMAC verification: GitLab uses X-Gitlab-Token header (plain token match, not HMAC)

---

## 10. File Manifest (All GitHub-Related Files)

### Backend (Rust) - 21 files

**Services:**
1. `crates/services/src/services/github.rs` - Main GitHub service
2. `crates/services/src/services/github/cli.rs` - gh CLI wrapper
3. `crates/services/src/services/pr_monitor.rs` - PR status monitoring
4. `crates/services/src/services/mod.rs` - Exports GitHubService

**Server Routes:**
5. `crates/server/src/routes/task_attempts/pr.rs` - PR API endpoints
6. `crates/server/src/routes/task_attempts/gh_cli_setup.rs` - CLI setup endpoint
7. `crates/server/src/routes/task_attempts.rs` - Includes PR routes
8. `crates/server/src/routes/mod.rs` - Route registration

**Remote (GitHub App):**
9. `crates/remote/src/github_app/mod.rs` - Public interface
10. `crates/remote/src/github_app/service.rs` - GitHub App API client
11. `crates/remote/src/github_app/jwt.rs` - JWT generation
12. `crates/remote/src/github_app/webhook.rs` - Signature verification
13. `crates/remote/src/github_app/pr_review.rs` - PR review orchestration
14. `crates/remote/src/db/github_app.rs` - Database layer
15. `crates/remote/src/routes/github_app.rs` - HTTP routes
16. `crates/remote/src/routes/mod.rs` - Route registration
17. `crates/remote/src/auth/provider.rs` - OAuth providers (includes GitHubOAuthProvider)

**Review Tool:**
18. `crates/review/src/github.rs` - PR review CLI helpers

**Database:**
19. `crates/remote/migrations/20251215000000_github_app_installations.sql` - Schema

**Tests:**
20. `crates/services/tests/git_workflow.rs` - Integration tests
21. `crates/services/src/services/config/*.rs` - Config version migrations (references GitHub)

---

### Frontend (TypeScript/React) - 15 files

**Dialogs:**
22. `frontend/src/components/dialogs/auth/GhCliSetupDialog.tsx` - CLI setup
23. `frontend/src/components/dialogs/tasks/CreatePRDialog.tsx` - PR creation
24. `frontend/src/components/dialogs/tasks/GitHubCommentsDialog.tsx` - Comment import
25. `frontend/src/components/dialogs/global/OAuthDialog.tsx` - OAuth flow (mentions GitHub)

**UI Components:**
26. `frontend/src/components/ui/github-comment-card.tsx` - Comment display
27. `frontend/src/components/ui/wysiwyg/nodes/github-comment-node.tsx` - Editor node
28. `frontend/src/components/ui/wysiwyg.tsx` - Includes GitHub node registration

**Task Components:**
29. `frontend/src/components/tasks/TaskFollowUpSection.tsx` - Shows PR comments
30. `frontend/src/components/tasks/TaskDetails/preview/NoServerContent.tsx` - GitHub mentions
31. `frontend/src/components/panels/PreviewPanel.tsx` - PR preview
32. `frontend/src/components/layout/Navbar.tsx` - OAuth links

**Hooks:**
33. `frontend/src/hooks/usePrComments.ts` - Fetch PR comments (likely exists, not read)

**API:**
34. `frontend/src/lib/api.ts` - API client (includes PR endpoints)

**Remote Frontend:**
35. `remote-frontend/src/pages/OrganizationPage.tsx` - GitHub App setup UI
36. `remote-frontend/src/api.ts` - API client

---

### Shared - 1 file

37. `shared/types.ts` - TypeScript types (auto-generated from Rust)

---

## 11. Critical Patterns to Replicate for GitLab

### Pattern 1: CLI Wrapper Pattern
- **GitHub:** `GhCli` struct wraps `gh` command
- **GitLab:** Need `GlabCli` struct wrapping `glab` command
- **Challenge:** `glab` is less mature than `gh`, may need REST API fallback

### Pattern 2: Unified Comment Type
- **GitHub:** `UnifiedPrComment` enum (General vs Review)
- **GitLab:** Simpler - all notes are similar structure
- **Benefit:** GitLab is easier here

### Pattern 3: OAuth + Webhook Integration
- **GitHub:** GitHub App model (installation ID, JWT, per-repo permissions)
- **GitLab:** Group/project access tokens (simpler but less granular)
- **Challenge:** Different auth models require different DB schema

### Pattern 4: Auto-Description via Coding Agent
- **GitHub:** Triggers follow-up prompt with `{pr_number}`, `{pr_url}`
- **GitLab:** Same pattern but use MR number/URL
- **Benefit:** Easy to port

### Pattern 5: Multi-Repo Support
- **GitHub:** `workspace_repo` table tracks PR per repo
- **GitLab:** Same pattern applies
- **Benefit:** Already designed for multi-repo

---

## 12. Migration Strategy Recommendations

### Phase 1: Core GitLab Support (Local)
1. **Add GitLab CLI wrapper:** `crates/services/src/services/gitlab.rs`
   - `GitLabService` struct
   - `GlabCli` wrapper
   - `create_merge_request()`, `get_mr_notes()`
2. **Server endpoints:** `crates/server/src/routes/task_attempts/mr.rs`
   - `create_gitlab_mr()`
   - `attach_existing_mr()`
   - `get_mr_notes()`
3. **Frontend dialogs:**
   - `GlabCliSetupDialog.tsx`
   - `CreateMRDialog.tsx`
   - `GitLabNotesDialog.tsx`

### Phase 2: GitLab OAuth Provider (Remote)
4. **OAuth provider:** `crates/remote/src/auth/provider.rs`
   - Add `GitLabOAuthProvider` struct
   - Handle token refresh (GitLab tokens expire!)
5. **Config:** Add GitLab OAuth env vars

### Phase 3: GitLab Webhooks (Remote)
6. **Webhook handler:** `crates/remote/src/routes/gitlab_webhooks.rs`
   - Handle: Merge Request Hook, Push Hook, Note Hook
   - Trigger MR reviews on open
   - Support `!reviewfast` in comments
7. **Database schema:** New migration for `gitlab_group_tokens`, `gitlab_projects`
8. **GitLab API client:** `crates/remote/src/gitlab/service.rs`
   - Token-based auth (no JWT)
   - `get_project_details()`, `post_mr_note()`, `clone_repo()`

### Phase 4: Abstraction Layer
9. **Trait-based abstraction:**
   ```rust
   pub trait GitProvider {
       fn create_pr_or_mr(...) -> Result<PullRequestInfo>;
       fn get_comments(...) -> Result<Vec<Comment>>;
       fn parse_remote_url(...) -> Result<RepoInfo>;
   }
   ```
10. **Config:** Add `git_provider: GitHub | GitLab` to workspace/repo config

---

## 13. Unresolved Questions

1. **GitLab CLI maturity:** Is `glab` CLI reliable enough, or should we use REST API directly?
2. **GitLab App model:** GitLab doesn't have "Apps" like GitHub - use Group Access Tokens or OAuth Apps?
3. **Webhook triggers:** Does GitLab support comment-based triggers like `!reviewfast`? (Yes, via Note Hooks)
4. **Draft MRs:** GitLab uses `WIP:` prefix or `Draft:` - how to handle in UI?
5. **Multi-instance support:** Should we support self-hosted GitLab (gitlab.example.com)?
6. **Token expiry:** GitLab tokens expire - need refresh logic in background job?
7. **Review comments:** GitLab has "discussions" on MRs - do we treat them like GitHub review comments?

---

## 14. Estimated Complexity

**Effort:** ~2-3 weeks for full GitLab parity
- Phase 1 (Local MR support): 5-7 days
- Phase 2 (OAuth): 2-3 days
- Phase 3 (Webhooks): 5-7 days
- Phase 4 (Abstraction): 3-4 days

**Risk Areas:**
- GitLab CLI (`glab`) may require REST API fallback
- Webhook signature verification different from GitHub
- Token refresh logic more complex than GitHub
- Self-hosted GitLab support adds configuration complexity

---

## 15. Conclusion

GitHub integration is extensive but follows clear patterns:
1. **CLI wrapper** for local operations
2. **REST API client** for remote operations
3. **OAuth provider** for user auth
4. **Webhook receiver** for automation
5. **Database layer** for state tracking

GitLab port requires parallel implementation of each layer. Key differences:
- **Auth:** Tokens instead of JWT
- **CLI:** `glab` less mature than `gh`
- **Webhooks:** Different event model and signature verification
- **Terminology:** Merge Request vs Pull Request, Notes vs Comments

**Recommendation:** Start with Phase 1 (local MR support) to prove viability before investing in full webhook integration.

