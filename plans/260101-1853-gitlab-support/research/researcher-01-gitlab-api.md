# GitLab API Integration Research

## 1. Authentication Methods

### Personal Access Tokens (PAT)
- **Primary method** for API authentication
- Created in user profile settings with configurable scopes
- Sent via `PRIVATE-TOKEN` header or `private_token` query param
- Alternative to OAuth2 for programmatic access
- Also works for Git over HTTP using Basic Auth (username: token value)
- Supports expiration dates and fine-grained permissions

### OAuth2 Token
- Sent via `Authorization: Bearer <token>` header or `access_token` param
- Supports standard OAuth2 flows: authorization code, implicit, resource owner password credentials
- Token endpoint: `POST /oauth/token`
- Authorization endpoint: `GET /oauth/authorize`
- PKCE (Proof Key for Code Exchange) supported for enhanced security
- Requires OAuth application registration in GitLab instance

### OAuth2 Device Authorization Grant Flow
- Status: **Not yet implemented** (tracked in issue #332682 from June 2021)
- Intended for headless apps (CLI tools, Git credential managers)
- Currently unavailable - must use PAT or other OAuth flows for CLI integrations

### Project Access Tokens
- Scoped to specific project, not user-owned
- Similar capabilities to PAT but project-level permissions
- Useful for CI/CD, automation without user context

### Session Cookie
- Used for browser-based authentication
- Not recommended for API integrations

### GitLab CI/CD Job Token
- Available within CI/CD pipelines via `$CI_JOB_TOKEN`
- Limited to job execution context

## 2. Key API Endpoints

### Repository Operations
- **Projects API**: `/api/v4/projects` - CRUD operations
- **Repository Files**: `/api/v4/projects/:id/repository/files/:file_path`
- **Branches**: `/api/v4/projects/:id/repository/branches`
- **Commits**: `/api/v4/projects/:id/repository/commits`
- **Clone URL**: Retrieved from project object (`http_url_to_repo`, `ssh_url_to_repo`)
- **Push detection**: Use webhooks (push events) or poll commits endpoint

### Issues & Merge Requests
- **Issues**: `/api/v4/projects/:id/issues`
- **Merge Requests**: `/api/v4/projects/:id/merge_requests`
- **Notes/Comments**: `/api/v4/projects/:id/issues/:issue_iid/notes`
- Supports filtering, pagination, state changes

### User & Project Info
- **Current User**: `/api/v4/user`
- **Users**: `/api/v4/users`
- **Projects**: `/api/v4/projects` (list all accessible)
- **Groups**: `/api/v4/groups`
- **Project Members**: `/api/v4/projects/:id/members`

### Webhooks
- **Project Webhooks API**: `/api/v4/projects/:id/hooks`
- Create, update, delete, test webhooks programmatically
- Events: push, issues, merge_requests, wiki_page, deployment, release, etc.
- Supports secret tokens for verification
- SSL verification configurable
- Branch filtering supported
- Webhook timeout: must respond quickly or GitLab retries

## 3. Self-Managed vs GitLab.com

### Base URL Patterns
- **GitLab.com**: `https://gitlab.com/api/v4/`
- **Self-hosted**: `https://<your-instance>/api/v4/`
- API path structure identical, only domain differs
- Users must specify correct base URL for their instance

### Version Compatibility
- Single codebase for both SaaS and self-managed
- API versioned independently (currently v4)
- Self-managed instances may be on older versions (check `/api/v4/version`)
- Feature parity depends on subscription tier (Free, Premium, Ultimate)
- CE (Community Edition) vs EE (Enterprise Edition) affects features, not API structure
- No formal compatibility matrix published - test against target version

### Feature Differences
- GitLab.com has more features on Free tier than self-managed CE
- Self-managed admins can customize limits and features
- Some enterprise features (SAML, advanced security) only in EE/Premium+

## 4. Rate Limiting

### GitLab.com Limits
- **Authenticated API traffic**: 2,000 requests/minute per user
- **Unauthenticated Projects API**: 400 requests/hour per IP
- **Raw file endpoint**: 5 calls/minute for files >10MB
- **Projects/Groups/Users API**: Additional limits announced (not specified in search)
- Headers: `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`

### Self-Managed Limits
- **Default**: Same as GitLab.com (2,000 req/min for authenticated)
- **Configurable**: Admins can modify via Rails console
- Can set to 0 (unlimited) on self-managed
- Some limits (e.g., diff commits per MR: 1M) may differ

### Best Practices
- Monitor rate limit headers in responses
- Implement exponential backoff for 429 errors
- Use webhooks instead of polling where possible
- Authenticate requests to get higher limits

## 5. Implementation Recommendations

### Authentication Strategy
1. **Primary**: Personal Access Token (simplest, widest support)
   - User creates token with required scopes (read_api, read_repository, write_repository)
   - Store securely, pass via `PRIVATE-TOKEN` header
2. **Future**: OAuth2 authorization code flow with PKCE
   - For user-facing apps requiring delegated access
   - Device flow unavailable, must use browser-based flow
3. **Project Automation**: Project Access Tokens for scoped operations

### URL Configuration
- Allow users to specify base URL (default: `https://gitlab.com`)
- Validate format: `https://<domain>/api/v4`
- Test connectivity with `/api/v4/version` endpoint

### Rate Limit Handling
- Parse `RateLimit-*` headers from responses
- Implement request queue with rate limiting
- Fallback to webhooks for real-time updates instead of polling

### Webhook Integration
- For push detection, issues, MR updates
- Use project webhooks API to register endpoint
- Verify webhook secret token
- Handle retries (idempotent processing)

## Unresolved Questions

1. **Device flow timeline**: When will OAuth2 device authorization grant be available? (Issue #332682 open since 2021)
2. **API versioning policy**: Deprecation timeline for API v4? Future v5 plans?
3. **Webhook retry policy**: Exact retry intervals and max attempts when endpoint fails?
4. **Token rotation**: Best practices for PAT rotation in long-running integrations?
5. **GraphQL API**: Should we also support GraphQL alongside REST for better performance on complex queries?
