# Self-Hosted GitLab Configuration

Vibe Kanban fully supports self-hosted GitLab Community Edition (CE) and Enterprise Edition (EE) instances. This guide covers configuration and authentication for custom GitLab deployments.

## Requirements

### Minimum GitLab Version

- **GitLab CE/EE 13.0 or higher** (released May 2020)
- HTTPS recommended (required for OAuth with glab CLI)

### Network Access

- GitLab instance accessible from your machine
- Port 443 (HTTPS) or 80 (HTTP) open
- DNS resolution for custom domains configured

### Permissions

- User account with project access (Reporter, Developer, or Maintainer role)
- Git SSH/HTTPS access configured

## Configuration

### Step 1: Install glab CLI

See [GitLab Setup Guide](./gitlab-setup.md#installation) for installation instructions.

### Step 2: Authenticate with Custom Instance

```bash
glab auth login --hostname gitlab.example.com
```

Follow interactive prompts:
1. Confirm hostname: `gitlab.example.com`
2. Choose protocol: **HTTPS** (recommended) or **HTTP**
3. Authentication method:
   - **Web browser** (OAuth) - Best option if HTTPS enabled
   - **Paste an authentication token** - Use if OAuth unavailable

### OAuth Authentication (HTTPS Required)

If your instance supports HTTPS, use OAuth flow:

```bash
glab auth login --hostname gitlab.example.com
# Select "Login with a web browser"
# Browser opens → Login → Authorize application
# Return to terminal
```

**Note:** OAuth requires HTTPS. HTTP-only instances must use token authentication.

### Token Authentication (HTTP or HTTPS)

If HTTPS unavailable or OAuth fails:

1. Create token at `https://gitlab.example.com/-/profile/personal_access_tokens`
2. Select scopes: `api`, `read_repository`, `write_repository`
3. Copy token
4. Run:
   ```bash
   glab auth login --hostname gitlab.example.com
   # Select "Paste an authentication token"
   # Paste token when prompted
   ```

### Multiple Instance Support

glab supports multiple GitLab instances simultaneously:

```bash
# Login to multiple instances
glab auth login --hostname gitlab.company.com
glab auth login --hostname gitlab.opensource.org

# Check all authenticated instances
glab auth status

# Switch default instance (set in repo)
cd /path/to/repo
glab config set host gitlab.company.com
```

## Environment Variable (Optional)

For automatic host detection, set the base URL:

```bash
export GITLAB_BASE_URL="https://gitlab.example.com"
```

Add to shell profile for persistence:
```bash
echo 'export GITLAB_BASE_URL="https://gitlab.example.com"' >> ~/.zshrc
source ~/.zshrc
```

## Repository Auto-Detection

Vibe Kanban automatically detects self-hosted instances from git remote URLs:

```bash
# Check your remote
git remote -v

# Supported formats:
# https://gitlab.example.com/group/project.git
# git@gitlab.example.com:group/project.git
# https://gitlab.company.com/group/subgroup/project.git
```

If remote URL contains a non-`gitlab.com` domain, Vibe Kanban will:
1. Extract hostname from URL
2. Use glab CLI configured for that host
3. Fall back to `GITLAB_BASE_URL` if set

## Verification Checklist

Before using Vibe Kanban with self-hosted GitLab:

- [ ] GitLab version ≥ 13.0
- [ ] Network connectivity from your machine
- [ ] DNS resolves GitLab hostname
- [ ] SSL certificate valid (for HTTPS)
- [ ] User has project access (Developer role minimum)
- [ ] glab CLI installed
- [ ] glab authenticated for custom hostname

## Troubleshooting

### Connection Refused

**Error:** `Connection refused` or `Failed to connect`

**Solutions:**
1. Verify GitLab instance is running:
   ```bash
   curl -I https://gitlab.example.com
   ```
2. Check firewall rules allow outbound connections
3. Verify DNS resolves correctly:
   ```bash
   nslookup gitlab.example.com
   ```

### SSL Certificate Errors

**Error:** `SSL certificate problem: self signed certificate`

**Solutions:**
1. Add CA certificate to system trust store
2. Use HTTP with token authentication (not recommended for production)

### glab Can't Find Instance

**Error:** `failed to get current user`

**Solutions:**
1. Re-authenticate glab:
   ```bash
   glab auth logout --hostname gitlab.example.com
   glab auth login --hostname gitlab.example.com
   ```
2. Set host explicitly in repo:
   ```bash
   cd /path/to/repo
   glab config set host gitlab.example.com
   ```

## Examples

### Example 1: Company Internal GitLab

```bash
# Setup
glab auth login --hostname gitlab.company.internal

# Verify
glab auth status

# Use with Vibe Kanban
cd /path/to/project
git remote -v  # origin git@gitlab.company.internal:team/project.git
npx vibe-kanban
```

### Example 2: Multiple Instances

```bash
# Authenticate to company GitLab
glab auth login --hostname gitlab.company.com

# Authenticate to client GitLab
glab auth login --hostname gitlab.client.com

# Use with project 1 (company)
cd /path/to/company-project
glab config set host gitlab.company.com
npx vibe-kanban

# Use with project 2 (client)
cd /path/to/client-project
glab config set host gitlab.client.com
npx vibe-kanban
```

## Security Best Practices

- **Use HTTPS** - Always use encrypted connections
- **Use OAuth when possible** - More secure than tokens
- **VPN/Private network** - Access internal GitLab over VPN
- **2FA required** - Enforce two-factor authentication

## Support

For issues specific to self-hosted GitLab:

1. Check [Troubleshooting Guide](./troubleshooting-git-providers.md)
2. Verify GitLab instance health and configuration
3. Contact GitLab administrator
4. Open [GitHub Discussion](https://github.com/BloopAI/vibe-kanban/discussions)

## Next Steps

- [GitLab Setup Guide](./gitlab-setup.md) - General setup instructions
- [Troubleshooting Guide](./troubleshooting-git-providers.md) - Common issues
