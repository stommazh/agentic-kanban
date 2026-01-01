# Phase 5: Remote Integration (OPTIONAL)

**Effort:** 5-7 days
**Priority:** P2 (Enhancement)
**Prerequisites:** Phase 1-4 complete
**Status:** Optional - Future enhancement

## Context

Research:
- [GitLab API Analysis](./research/researcher-01-gitlab-api.md) - OAuth2, webhooks, token management
- [GitHub App Integration](./scout/scout-01-github-integration.md) - Remote deployment patterns

Phases 1-4 provide full local GitLab support. This optional phase adds server-side integration for automated MR reviews, webhooks, and GitLab OAuth authentication provider.

## Overview

Implement GitLab remote integration parallel to existing GitHub App functionality: OAuth provider for user authentication, webhook receiver for MR events, automated code reviews triggered by MR creation.

## Key Insights

1. **No GitLab Apps** - GitLab lacks "App" model like GitHub, use OAuth Apps + Group Access Tokens
2. **Token expiry** - GitLab OAuth tokens expire (unlike GitHub), need refresh token flow
3. **Webhook model** - Similar to GitHub but different event names and payload structure
4. **Group-level permissions** - GitLab groups (not orgs) are top-level entities
5. **Self-hosted complexity** - Must support multiple GitLab instances (gitlab.com + custom domains)

## Requirements

### Functional
- [ ] GitLab OAuth provider for user authentication
- [ ] Webhook receiver for MR events (opened, commented)
- [ ] Automated MR review on creation (parallel to GitHub PR review)
- [ ] Group Access Token management for API operations
- [ ] Support self-hosted GitLab instances in OAuth
- [ ] Token refresh logic (GitLab tokens expire)

### Non-Functional
- [ ] Webhook signature verification (X-Gitlab-Token)
- [ ] Token refresh happens transparently
- [ ] Support multiple GitLab instances simultaneously
- [ ] Webhook processing < 2s latency

## Architecture

### Database Schema
```sql
-- GitLab OAuth applications (one per instance)
CREATE TABLE gitlab_applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_host TEXT NOT NULL UNIQUE, -- "gitlab.com" or "gitlab.example.com"
    application_id TEXT NOT NULL,
    application_secret TEXT NOT NULL, -- encrypted
    webhook_secret TEXT NOT NULL, -- encrypted
    base_url TEXT NOT NULL, -- "https://gitlab.com/api/v4"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Group access tokens (parallel to GitHub installations)
CREATE TABLE gitlab_group_integrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    gitlab_application_id UUID NOT NULL REFERENCES gitlab_applications(id),
    gitlab_group_id BIGINT NOT NULL,
    gitlab_group_path TEXT NOT NULL, -- "my-org" or "parent/child"
    access_token TEXT NOT NULL, -- encrypted
    refresh_token TEXT NOT NULL, -- encrypted
    token_expires_at TIMESTAMPTZ NOT NULL,
    installed_by_user_id UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, gitlab_application_id, gitlab_group_id)
);

-- Projects with review enabled flag
CREATE TABLE gitlab_projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    integration_id UUID NOT NULL REFERENCES gitlab_group_integrations(id) ON DELETE CASCADE,
    gitlab_project_id BIGINT NOT NULL,
    project_path TEXT NOT NULL, -- "group/project"
    review_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(integration_id, gitlab_project_id)
);

-- Pending OAuth installations (temp state)
CREATE TABLE gitlab_pending_oauth (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gitlab_application_id UUID NOT NULL REFERENCES gitlab_applications(id),
    state_token TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### GitLab OAuth Provider
```rust
pub struct GitLabOAuthProvider {
    client_id: String,
    client_secret: SecretString,
    base_url: String, // "https://gitlab.com" or self-hosted
}

impl OAuthProvider for GitLabOAuthProvider {
    fn authorization_url(&self, state: &str, redirect_uri: &str) -> String {
        format!(
            "{}/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&state={}&scope=read_user+api",
            self.base_url, self.client_id, redirect_uri, state
        )
    }

    async fn exchange_code(&self, code: &str, redirect_uri: &str)
        -> Result<OAuthTokens, OAuthError> {
        let response = self.http_client
            .post(&format!("{}/oauth/token", self.base_url))
            .json(&serde_json::json!({
                "client_id": self.client_id,
                "client_secret": self.client_secret.expose_secret(),
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": redirect_uri,
            }))
            .send()
            .await?;

        let tokens: GitLabTokenResponse = response.json().await?;

        Ok(OAuthTokens {
            access_token: tokens.access_token,
            refresh_token: Some(tokens.refresh_token), // Important: GitLab tokens expire
            expires_at: Some(Utc::now() + Duration::seconds(tokens.expires_in)),
        })
    }

    async fn refresh_token(&self, refresh_token: &str)
        -> Result<OAuthTokens, OAuthError> {
        let response = self.http_client
            .post(&format!("{}/oauth/token", self.base_url))
            .json(&serde_json::json!({
                "client_id": self.client_id,
                "client_secret": self.client_secret.expose_secret(),
                "refresh_token": refresh_token,
                "grant_type": "refresh_token",
            }))
            .send()
            .await?;

        let tokens: GitLabTokenResponse = response.json().await?;

        Ok(OAuthTokens {
            access_token: tokens.access_token,
            refresh_token: Some(tokens.refresh_token),
            expires_at: Some(Utc::now() + Duration::seconds(tokens.expires_in)),
        })
    }

    async fn get_user_info(&self, access_token: &str)
        -> Result<UserInfo, OAuthError> {
        let response = self.http_client
            .get(&format!("{}/api/v4/user", self.base_url))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        let user: GitLabUser = response.json().await?;

        Ok(UserInfo {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
        })
    }
}
```

### Webhook Handler
```rust
// crates/remote/src/routes/gitlab_webhooks.rs

pub async fn handle_gitlab_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, AppError> {
    // Verify webhook signature
    let token = headers.get("X-Gitlab-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // Look up GitLab application by webhook secret
    let app = state.db.get_gitlab_app_by_webhook_secret(token).await?;

    // Parse event type
    let event_type = headers.get("X-Gitlab-Event")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::BadRequest("Missing X-Gitlab-Event header"))?;

    let payload: serde_json::Value = serde_json::from_str(&body)?;

    match event_type {
        "Merge Request Hook" => {
            handle_merge_request_event(&state, &app, payload).await?;
        }
        "Note Hook" => {
            handle_note_event(&state, &app, payload).await?;
        }
        "Push Hook" => {
            handle_push_event(&state, &app, payload).await?;
        }
        _ => {
            tracing::debug!("Ignoring GitLab event: {}", event_type);
        }
    }

    Ok(StatusCode::OK)
}

async fn handle_merge_request_event(
    state: &AppState,
    app: &GitLabApplication,
    payload: serde_json::Value,
) -> Result<(), AppError> {
    let action = payload["object_attributes"]["action"].as_str()
        .unwrap_or("");

    if action != "open" {
        return Ok(()); // Only trigger on MR open
    }

    let project_id = payload["project"]["id"].as_i64().unwrap();
    let mr_iid = payload["object_attributes"]["iid"].as_i64().unwrap();

    // Check if review enabled for this project
    let integration = state.db
        .get_gitlab_integration_by_project_id(project_id)
        .await?;

    if !integration.review_enabled {
        return Ok(());
    }

    // Trigger MR review (parallel to GitHub PR review)
    tokio::spawn(async move {
        if let Err(e) = review_gitlab_mr(state.clone(), integration, mr_iid as u64).await {
            tracing::error!("GitLab MR review failed: {}", e);
        }
    });

    Ok(())
}
```

### Token Refresh Background Job
```rust
// Refresh tokens expiring within 24h
pub async fn refresh_expiring_gitlab_tokens(db: &Database) -> Result<(), Error> {
    let threshold = Utc::now() + Duration::hours(24);

    let integrations = db
        .get_gitlab_integrations_expiring_before(threshold)
        .await?;

    for integration in integrations {
        let app = db.get_gitlab_application(integration.gitlab_application_id).await?;
        let provider = GitLabOAuthProvider::new(
            app.application_id,
            app.application_secret,
            app.base_url,
        );

        match provider.refresh_token(&integration.refresh_token).await {
            Ok(tokens) => {
                db.update_gitlab_integration_tokens(
                    &integration.id,
                    &tokens.access_token,
                    &tokens.refresh_token.unwrap(),
                    tokens.expires_at.unwrap(),
                ).await?;

                tracing::info!("Refreshed GitLab token for integration {}", integration.id);
            }
            Err(e) => {
                tracing::error!("Failed to refresh GitLab token: {}", e);
                // Consider notifying user or marking integration as invalid
            }
        }
    }

    Ok(())
}
```

## Related Code Files

**To Create:**
- `crates/remote/src/gitlab_integration/mod.rs` - Public interface
- `crates/remote/src/gitlab_integration/oauth.rs` - OAuth provider
- `crates/remote/src/gitlab_integration/webhooks.rs` - Webhook verification
- `crates/remote/src/gitlab_integration/mr_review.rs` - Automated review orchestration
- `crates/remote/src/db/gitlab_integration.rs` - Database queries
- `crates/remote/src/routes/gitlab_webhooks.rs` - HTTP routes
- `crates/remote/migrations/YYYYMMDD_gitlab_integration.sql` - Schema

**To Update:**
- `crates/remote/src/auth/provider.rs` - Add `GitLabOAuthProvider`
- `crates/remote/src/routes/mod.rs` - Register GitLab routes
- `crates/remote/src/config.rs` - Add GitLab config
- `remote-frontend/src/pages/OrganizationPage.tsx` - GitLab integration UI

**Reference:**
- `crates/remote/src/github_app/` - Pattern to replicate
- `crates/remote/src/routes/github_app.rs` - Route structure

## Implementation Steps

### Step 1: Database Schema (1d)
1. Create migration with 4 new tables
2. Add indexes on foreign keys
3. Implement database queries in `db/gitlab_integration.rs`
4. CRUD for applications, integrations, projects

### Step 2: OAuth Provider (1.5d)
1. Implement `GitLabOAuthProvider` in `auth/provider.rs`
2. Add token exchange endpoint
3. Add refresh token logic
4. Support multiple instances (gitlab.com + self-hosted)
5. Test OAuth flow end-to-end

### Step 3: Webhook Handler (1.5d)
1. Create `routes/gitlab_webhooks.rs`
2. Implement signature verification (X-Gitlab-Token)
3. Parse "Merge Request Hook", "Note Hook"
4. Route to event handlers
5. Test with mock webhook payloads

### Step 4: MR Review Integration (1.5d)
1. Create `gitlab_integration/mr_review.rs`
2. Clone repo with OAuth token
3. Run code review (reuse existing review crate)
4. Post review as MR note via API
5. Handle errors gracefully (comment if review fails)

### Step 5: Token Refresh Job (0.5d)
1. Background task to refresh expiring tokens
2. Run every 6 hours
3. Update database with new tokens
4. Log failures for monitoring

### Step 6: Frontend Integration (1d)
1. Update `remote-frontend/src/pages/OrganizationPage.tsx`
2. Add "Setup GitLab Integration" button
3. OAuth flow: redirect → callback → store tokens
4. List GitLab projects with review toggle
5. Show token expiry warning

### Step 7: API Routes (1d)
1. `GET /v1/organizations/{org_id}/gitlab/oauth-url` - Generate OAuth URL
2. `GET /v1/gitlab/oauth/callback` - Handle callback
3. `POST /v1/gitlab/webhook` - Webhook receiver
4. `GET /v1/organizations/{org_id}/gitlab/projects` - List projects
5. `PATCH /v1/organizations/{org_id}/gitlab/projects/{id}/review-enabled` - Toggle

### Step 8: Testing (1d)
1. Unit tests for OAuth token exchange
2. Unit tests for webhook verification
3. Integration test for MR review flow
4. Test token refresh job
5. Test multi-instance support

## Todo List

- [ ] Create database migration with 4 new tables
- [ ] Implement `GitLabOAuthProvider` with token refresh
- [ ] Create webhook handler with signature verification
- [ ] Implement MR review orchestration
- [ ] Create token refresh background job
- [ ] Add frontend UI for GitLab integration setup
- [ ] Create API routes for OAuth flow
- [ ] Implement project listing and review toggle
- [ ] Write unit tests for OAuth and webhooks
- [ ] Write integration test for automated MR review
- [ ] Document OAuth application setup in GitLab
- [ ] Add environment variables for GitLab config
- [ ] Test with self-hosted GitLab instance
- [ ] Monitor token refresh job in production

## Success Criteria

### Must Have
- [x] Users can authenticate via GitLab OAuth
- [x] Webhooks trigger automated MR reviews
- [x] Token refresh happens automatically
- [x] Support self-hosted GitLab instances
- [x] Review comments posted to MR successfully
- [x] Frontend shows GitLab integration status

### Nice to Have
- [ ] Multi-instance support (multiple self-hosted GitLabs)
- [ ] Token expiry warnings in UI
- [ ] Webhook event replay for debugging
- [ ] Metrics on review success rate

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Token refresh failures | High | High | Retry logic, user notifications, graceful degradation |
| Webhook signature bypass | Medium | Critical | Strict signature verification, rotate secrets |
| Self-hosted instance incompatibility | Medium | High | Document minimum version, test against v13/v14/v15 |
| OAuth state CSRF attacks | Medium | Critical | Strong random state tokens, short expiry |
| Webhook processing delays | Low | Medium | Async processing, queue if needed |

## Security Considerations

- **Token storage**: Encrypt access_token and refresh_token at rest
- **Webhook secrets**: Strong random generation, rotate periodically
- **OAuth state**: Cryptographically random, expire after 10 min
- **Token refresh**: Rate limit refresh attempts to prevent abuse
- **Webhook replay**: Add timestamp validation, reject old events
- **Multi-tenant isolation**: Ensure org A cannot access org B's GitLab integration

## Next Steps

After Phase 5 completion:
1. Monitor webhook delivery and review success rates
2. Gather user feedback on GitLab integration experience
3. Consider additional features (MR approvals, CI integration)

## Unresolved Questions

1. **Multi-instance UI**: How to present multiple GitLab instances to users (dropdown, tabs)?
2. **Token refresh failures**: Should we auto-disable integration after N consecutive failures?
3. **Webhook retries**: Does GitLab retry failed webhooks? If so, how to handle duplicates?
4. **Group vs project tokens**: Use Group Access Tokens or Project Access Tokens? (Group recommended)
5. **OAuth scopes**: Are `read_user + api` scopes sufficient or need more granular permissions?
6. **Self-hosted SSL**: How to handle self-signed certificates (allow bypass or require valid cert)?
