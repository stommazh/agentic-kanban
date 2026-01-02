# Merge Request API Documentation

This document describes the merge request (MR) and pull request (PR) API endpoints in Vibe Kanban. These endpoints work with both GitHub (PRs) and GitLab (MRs) through a unified abstraction layer.

## Overview

Vibe Kanban automatically detects the git provider (GitHub or GitLab) from your repository's remote URL and routes API calls to the appropriate provider. The API surface is identical regardless of provider.

### Provider Detection

Provider detection is automatic based on git remote URL:

```bash
# GitHub
git@github.com:owner/repo.git          → GitHub
https://github.com/owner/repo.git      → GitHub

# GitLab
git@gitlab.com:group/project.git       → GitLab
https://gitlab.com/group/project.git   → GitLab
git@gitlab.example.com:group/proj.git  → GitLab (self-hosted)
```

### Authentication

The API uses the authentication method you've configured:

- **GitHub:** gh CLI or GH_TOKEN/GITHUB_TOKEN
- **GitLab:** glab CLI or GITLAB_TOKEN + GITLAB_BASE_URL

See [GitLab Setup Guide](../gitlab-setup.md) and [GitHub CLI documentation](https://cli.github.com/) for authentication setup.

## Endpoints

### POST /api/workspaces/:workspace_id/merge-requests

Create a new merge request (GitLab) or pull request (GitHub).

**Authentication:** Required (workspace extension)

**Request Body:**

```typescript
{
  title: string;                    // MR/PR title
  body?: string;                    // MR/PR description (optional)
  target_branch?: string;           // Base branch (default: workspace target_branch)
  draft?: boolean;                  // Mark as draft/WIP (default: false)
  repo_id: string;                  // UUID of repository
  auto_generate_description: boolean; // Auto-improve description via AI (default: false)
}
```

**Response:**

```typescript
{
  success: true,
  data: string                      // MR/PR web URL
}
```

**Error Responses:**

```typescript
{
  success: false,
  error: {
    type: "github_cli_not_installed" | "github_cli_not_logged_in" |
          "git_cli_not_installed" | "git_cli_not_logged_in" |
          "target_branch_not_found";
    branch?: string;                // Only for target_branch_not_found
  }
}
```

**Example:**

```bash
curl -X POST http://localhost:3000/api/workspaces/123e4567-e89b-12d3-a456-426614174000/merge-requests \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Add user authentication",
    "body": "Implements JWT-based authentication for API endpoints",
    "target_branch": "main",
    "draft": false,
    "repo_id": "987fcdeb-51a2-43f1-b456-789012345678",
    "auto_generate_description": false
  }'
```

**Response:**

```json
{
  "success": true,
  "data": "https://gitlab.com/group/project/-/merge_requests/42"
}
```

**Behavior:**

1. Pushes current workspace branch to remote
2. Creates MR/PR via provider CLI (falls back to API if CLI unavailable)
3. Stores MR/PR metadata in database
4. Opens MR/PR in browser automatically
5. If `auto_generate_description: true`, triggers AI agent to improve title/description

**Notes:**

- Provider auto-detected from repo remote URL
- Branch must have commits (cannot create MR/PR with no changes)
- Target branch must exist on remote
- For GitLab MRs, terminology uses "merge request"; for GitHub, "pull request"

### POST /api/workspaces/:workspace_id/merge-requests/attach

Attach an existing MR/PR to the workspace.

**Authentication:** Required (workspace extension)

**Request Body:**

```typescript
{
  repo_id: string;                  // UUID of repository
}
```

**Response:**

```typescript
{
  success: true,
  data: {
    pr_attached: boolean;           // True if MR/PR found and attached
    pr_url?: string;                // MR/PR web URL
    pr_number?: number;             // MR/PR number (IID for GitLab)
    pr_status?: "open" | "merged" | "closed" | "unknown";
  }
}
```

**Example:**

```bash
curl -X POST http://localhost:3000/api/workspaces/123e4567-e89b-12d3-a456-426614174000/merge-requests/attach \
  -H "Content-Type: application/json" \
  -d '{
    "repo_id": "987fcdeb-51a2-43f1-b456-789012345678"
  }'
```

**Response:**

```json
{
  "success": true,
  "data": {
    "pr_attached": true,
    "pr_url": "https://github.com/owner/repo/pull/123",
    "pr_number": 123,
    "pr_status": "open"
  }
}
```

**Behavior:**

1. Checks if MR/PR already attached for this workspace+repo
2. If not, queries provider API for MRs/PRs matching workspace branch
3. Takes first result (prefers open, but accepts merged/closed)
4. Stores MR/PR info in database
5. If MR/PR is merged, marks parent task as "done"

**Notes:**

- Useful when MR/PR created outside Vibe Kanban (e.g., via CLI or web UI)
- Returns existing attachment if already attached
- Does not create new MR/PR (use POST /merge-requests for that)

### GET /api/workspaces/:workspace_id/merge-requests/comments

Fetch comments/notes from attached MR/PR.

**Authentication:** Required (workspace extension)

**Query Parameters:**

```typescript
{
  repo_id: string;                  // UUID of repository
}
```

**Response:**

```typescript
{
  success: true,
  data: {
    comments: Array<GeneralComment | ReviewComment>
  }
}
```

**Comment Types:**

```typescript
// General comment (conversation/note)
type GeneralComment = {
  comment_type: "general";
  id: string;
  author: string;
  author_association: string;       // "OWNER", "MEMBER", "CONTRIBUTOR", etc.
  body: string;
  created_at: string;               // ISO 8601 timestamp
  url: string;
}

// Inline review comment (code review)
type ReviewComment = {
  comment_type: "review";
  id: number;
  author: string;
  author_association: string;
  body: string;
  created_at: string;
  url: string;
  path: string;                     // File path
  line?: number;                    // Line number (if available)
  diff_hunk: string;                // Code context
}
```

**Error Responses:**

```typescript
{
  success: false,
  error: {
    type: "no_pr_attached" | "github_cli_not_installed" | "github_cli_not_logged_in";
  }
}
```

**Example:**

```bash
curl -X GET "http://localhost:3000/api/workspaces/123e4567-e89b-12d3-a456-426614174000/merge-requests/comments?repo_id=987fcdeb-51a2-43f1-b456-789012345678"
```

**Response:**

```json
{
  "success": true,
  "data": {
    "comments": [
      {
        "comment_type": "general",
        "id": "1234567890",
        "author": "reviewer123",
        "author_association": "MEMBER",
        "body": "Looks good! Just a few minor suggestions.",
        "created_at": "2025-01-01T14:30:00Z",
        "url": "https://github.com/owner/repo/pull/123#issuecomment-1234567890"
      },
      {
        "comment_type": "review",
        "id": 9876543210,
        "author": "reviewer456",
        "author_association": "CONTRIBUTOR",
        "body": "Consider using a constant here instead of magic number.",
        "created_at": "2025-01-01T15:45:00Z",
        "url": "https://github.com/owner/repo/pull/123#discussion_r9876543210",
        "path": "src/config.ts",
        "line": 42,
        "diff_hunk": "@@ -40,7 +40,7 @@ export const config = {\n-  timeout: 5000,\n+  timeout: 10000,"
      }
    ]
  }
}
```

**Behavior:**

1. Verifies MR/PR attached to workspace for specified repo
2. Fetches comments from provider API
3. Returns both general comments and inline review comments
4. Comments sorted by creation time (oldest first)

**Notes:**

- Requires MR/PR already attached (via create or attach endpoint)
- GitLab returns "notes" (both discussion notes and diff notes)
- GitHub returns "issue comments" and "review comments"
- Some system-generated notes may not appear
- For GitLab, uses API directly (glab CLI doesn't handle comments well)

## Provider-Specific Behavior

### GitHub

- Uses `gh` CLI when available, falls back to GitHub API
- PR numbers are repository-scoped (unique per repo)
- Author associations: OWNER, MEMBER, CONTRIBUTOR, FIRST_TIME_CONTRIBUTOR, etc.
- Rate limit: 5000 requests/hour (authenticated)

### GitLab

- Uses `glab` CLI when available, falls back to GitLab API
- MR numbers are project-scoped (IID, not global ID)
- Author associations mapped from GitLab roles: Owner, Maintainer, Developer, Reporter, Guest
- Comments API uses "notes" endpoint (includes diff notes)
- Rate limit: 2000 requests/minute (default, configurable for self-hosted)

### Self-Hosted GitLab

For self-hosted GitLab instances:

1. Set `GITLAB_BASE_URL` environment variable:
   ```bash
   export GITLAB_BASE_URL="https://gitlab.example.com/api/v4"
   ```

2. Authenticate glab with custom host:
   ```bash
   glab auth login --hostname gitlab.example.com
   ```

3. API endpoints work identically
4. See [Self-Hosted GitLab Guide](../self-hosted-gitlab.md) for details

## Error Handling

### Common Errors

| Error Type | HTTP Status | Description | Solution |
|------------|-------------|-------------|----------|
| `github_cli_not_installed` | 200 (error in body) | gh/glab CLI not found | Install CLI |
| `github_cli_not_logged_in` | 200 (error in body) | CLI not authenticated | Run `gh auth login` or `glab auth login` |
| `git_cli_not_installed` | 200 (error in body) | git command not found | Install git |
| `git_cli_not_logged_in` | 200 (error in body) | git push authentication failed | Configure git credentials |
| `target_branch_not_found` | 200 (error in body) | Base branch doesn't exist | Check branch name, fetch latest |
| `no_pr_attached` | 200 (error in body) | No MR/PR linked to workspace | Create or attach MR/PR first |

**Note:** Errors returned in response body with `success: false`, not as HTTP error codes.

### Retry Logic

The API implements automatic retry with fallback:

1. **Try CLI first** - gh or glab (if installed and authenticated)
2. **Fall back to API** - Direct HTTP API calls (if token available)
3. **Return error** - If both methods fail

This ensures maximum reliability across different environments.

## Rate Limiting

### GitHub

- **Authenticated:** 5000 requests/hour
- **Unauthenticated:** 60 requests/hour
- Check limits: `gh api rate_limit`

### GitLab.com

- **Authenticated:** 2000 requests/minute
- Check limits via response headers:
  - `RateLimit-Limit`
  - `RateLimit-Remaining`
  - `RateLimit-Reset`

### Self-Hosted GitLab

Rate limits configurable by administrator. Contact admin if hitting limits.

## Best Practices

### Performance

1. **Use CLI when available** - Faster and more reliable than API
2. **Attach existing MRs** - Don't recreate if MR already exists
3. **Cache comment data** - Don't fetch comments on every page load
4. **Batch operations** - Create MRs for multiple repos in sequence, not parallel

### Security

1. **Never expose tokens** - Keep GH_TOKEN/GITLAB_TOKEN secret
2. **Use minimal scopes** - Only grant necessary permissions
3. **Rotate tokens regularly** - Recommended every 90 days
4. **Use OAuth when possible** - CLI OAuth flow preferred over tokens

### Error Handling

1. **Check CLI availability** - Handle both CLI and API errors
2. **Validate branch existence** - Check target branch before creating MR/PR
3. **Handle rate limits** - Implement backoff and retry logic
4. **Show user-friendly errors** - Map provider errors to actionable messages

## Examples

### Create GitHub PR with Auto-Description

```typescript
const response = await fetch('/api/workspaces/workspace-id/merge-requests', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    title: 'Feature: User authentication',
    body: 'Initial implementation',
    target_branch: 'main',
    draft: false,
    repo_id: 'repo-uuid',
    auto_generate_description: true  // AI agent will improve description
  })
});

const data = await response.json();
if (data.success) {
  console.log('PR created:', data.data);  // URL
} else {
  console.error('Error:', data.error.type);
}
```

### Attach Existing GitLab MR

```typescript
const response = await fetch('/api/workspaces/workspace-id/merge-requests/attach', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    repo_id: 'repo-uuid'
  })
});

const data = await response.json();
if (data.success && data.data.pr_attached) {
  console.log('MR attached:', data.data.pr_number, data.data.pr_url);
  console.log('Status:', data.data.pr_status);
} else {
  console.log('No MR found for this branch');
}
```

### Fetch Comments

```typescript
const params = new URLSearchParams({ repo_id: 'repo-uuid' });
const response = await fetch(`/api/workspaces/workspace-id/merge-requests/comments?${params}`);

const data = await response.json();
if (data.success) {
  const { comments } = data.data;

  // Separate general and review comments
  const generalComments = comments.filter(c => c.comment_type === 'general');
  const reviewComments = comments.filter(c => c.comment_type === 'review');

  console.log(`${generalComments.length} general comments`);
  console.log(`${reviewComments.length} review comments`);
} else {
  if (data.error.type === 'no_pr_attached') {
    console.log('No MR/PR attached yet');
  }
}
```

## Related Documentation

- [GitLab Setup Guide](../gitlab-setup.md) - Setup and authentication
- [Self-Hosted GitLab Guide](../self-hosted-gitlab.md) - Custom instance configuration
- [Troubleshooting Guide](../troubleshooting-git-providers.md) - Common issues

## TypeScript Types

TypeScript definitions generated via ts-rs from Rust types:

```typescript
// See shared/types.ts for generated types
import type {
  ProviderType,
  RepoIdentifier,
  PrState,
  PrInfo,
  UnifiedComment,
} from '../shared/types';
```

Types automatically kept in sync with Rust backend via `pnpm run generate-types`.

## Support

For API issues:

1. Check [Troubleshooting Guide](../troubleshooting-git-providers.md)
2. Verify authentication (CLI and tokens)
3. Enable debug logging: `export RUST_LOG=debug`
4. Open [GitHub Discussion](https://github.com/BloopAI/vibe-kanban/discussions) with sanitized logs

## Changelog

### v0.0.143 (2025-01-01)

- Added GitLab support (gitlab.com and self-hosted)
- Provider abstraction layer (unified GitHub/GitLab API)
- Auto-detection based on git remote URL
- CLI fallback to API for both providers
- Unified comment types (general + review)
- Multi-repository support per workspace

### Earlier Versions

- GitHub-only support (gh CLI + GitHub API)
