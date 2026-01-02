# Troubleshooting Git Providers

This guide covers common issues with GitHub and GitLab integration in Vibe Kanban, including authentication failures, API errors, and network problems.

## Table of Contents

- [Quick Diagnostics](#quick-diagnostics)
- [Authentication Issues](#authentication-issues)
- [Merge Request Creation Failures](#merge-request-creation-failures)
- [Comment Fetching Issues](#comment-fetching-issues)
- [Provider Detection Problems](#provider-detection-problems)
- [Network and Connectivity](#network-and-connectivity)
- [Self-Hosted Instance Issues](#self-hosted-instance-issues)
- [Performance Problems](#performance-problems)
- [Common Error Messages](#common-error-messages)

## Quick Diagnostics

Run these commands to quickly diagnose issues:

### Check Git Configuration

```bash
# Verify remote URL
git remote -v

# Check current branch
git branch --show-current

# Verify remote branch exists
git ls-remote --heads origin
```

### Check CLI Authentication

**GitHub:**
```bash
# Check gh CLI installation
gh --version

# Check authentication
gh auth status

# Test API access
gh api user
```

**GitLab:**
```bash
# Check glab CLI installation
glab --version

# Check authentication
glab auth status

# Test API access
glab api /user
```

### Check Environment Variables

```bash
# GitHub
echo $GH_TOKEN
echo $GITHUB_TOKEN

# GitLab
echo $GITLAB_TOKEN
echo $GITLAB_BASE_URL
```

## Authentication Issues

### GitHub CLI Not Installed

**Error:** `GitHub CLI not installed` or `gh: command not found`

**Solutions:**

1. **Install gh CLI:**
   ```bash
   # macOS
   brew install gh

   # Linux (Debian/Ubuntu)
   sudo apt install gh

   # Windows (Scoop)
   scoop install gh
   ```

2. **Verify installation:**
   ```bash
   gh --version
   ```

3. **Restart Vibe Kanban** to detect gh CLI

### GitHub CLI Not Authenticated

**Error:** `GitHub CLI not authenticated` or `gh: authentication required`

**Solutions:**

1. **Login to GitHub:**
   ```bash
   gh auth login
   ```

2. **Choose authentication method:**
   - **GitHub.com** (or GitHub Enterprise)
   - **HTTPS** protocol
   - **Login with a web browser** (recommended)

3. **Verify authentication:**
   ```bash
   gh auth status
   ```

4. **Token expired:** If token expired, re-authenticate:
   ```bash
   gh auth refresh
   ```

### GitLab CLI Not Installed

**Error:** `GitLab CLI not installed` or `glab: command not found`

**Solutions:**

1. **Install glab CLI:**
   ```bash
   # macOS
   brew install glab

   # Linux (Debian/Ubuntu)
   sudo apt install glab

   # Windows (Scoop)
   scoop install glab
   ```

2. **Verify installation:**
   ```bash
   glab --version
   ```

3. **Restart Vibe Kanban**

### GitLab CLI Not Authenticated

**Error:** `GitLab CLI not authenticated` or `glab: authentication required`

**Solutions:**

1. **Login to GitLab:**
   ```bash
   glab auth login
   ```

2. **For self-hosted instances:**
   ```bash
   glab auth login --hostname gitlab.example.com
   ```

3. **Verify authentication:**
   ```bash
   glab auth status
   ```

4. **Re-authenticate if expired:**
   ```bash
   glab auth logout
   glab auth login
   ```

### Personal Access Token Issues

**Error:** `401 Unauthorized` with token

**Solutions:**

1. **Check token scopes:**
   - GitHub: `repo`, `read:org`
   - GitLab: `api`, `read_repository`, `write_repository`

2. **Verify token not expired:**
   - Check expiration date in provider settings
   - Regenerate if expired

3. **Test token directly:**
   ```bash
   # GitHub
   curl -H "Authorization: Bearer $GH_TOKEN" https://api.github.com/user

   # GitLab
   curl -H "PRIVATE-TOKEN: $GITLAB_TOKEN" https://gitlab.com/api/v4/user
   ```

4. **Ensure environment variable set:**
   ```bash
   # GitHub
   export GH_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"

   # GitLab
   export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
   ```

5. **Check token not revoked:**
   - GitHub: Settings → Developer settings → Personal access tokens
   - GitLab: Settings → Access Tokens

## Merge Request Creation Failures

### Branch Not Pushed

**Error:** `Branch not found` or `source branch does not exist`

**Solutions:**

1. **Push branch to remote:**
   ```bash
   git push -u origin <branch-name>
   ```

2. **Verify branch pushed:**
   ```bash
   git ls-remote --heads origin <branch-name>
   ```

3. **Try creating MR again** in Vibe Kanban

### Target Branch Not Found

**Error:** `Target branch not found: main`

**Solutions:**

1. **Check target branch name:**
   ```bash
   git branch -r | grep main
   ```

2. **Common default branches:**
   - `main` (newer repositories)
   - `master` (older repositories)
   - `develop` (some workflows)

3. **Update target branch** in Vibe Kanban workspace settings

4. **Fetch latest branches:**
   ```bash
   git fetch origin
   ```

### Already Exists

**Error:** `Merge request already exists for this branch`

**Solutions:**

1. **Use "Attach Existing MR" feature** in Vibe Kanban instead of creating new one

2. **Find existing MR:**
   ```bash
   # GitHub
   gh pr list --head <branch-name>

   # GitLab
   glab mr list --source-branch <branch-name>
   ```

3. **Close old MR** if duplicate:
   ```bash
   # GitHub
   gh pr close <number>

   # GitLab
   glab mr close <number>
   ```

### Permission Denied

**Error:** `403 Forbidden` or `Permission denied`

**Solutions:**

1. **Check repository access:**
   - Verify you have write/push permissions
   - Minimum role: Developer (GitLab) or Write (GitHub)

2. **Check branch protection rules:**
   - Protected branches may require review before MR
   - Verify you're allowed to push to branch

3. **Verify organization membership:**
   - Ensure you're member of organization/group
   - Check if SSO/SAML required

4. **Check token permissions:**
   - Token must have `write` scope
   - Re-generate with correct permissions

## Comment Fetching Issues

### No PR/MR Attached

**Error:** `No PR/MR attached to this task`

**Solutions:**

1. **Create or attach MR first** using Vibe Kanban UI

2. **Verify MR exists:**
   ```bash
   # GitHub
   gh pr view --web

   # GitLab
   glab mr view --web
   ```

3. **Use "Attach Existing MR" feature** if MR created outside Vibe Kanban

### API Rate Limiting

**Error:** `429 Too Many Requests` or `Rate limit exceeded`

**Solutions:**

1. **Wait for rate limit reset:**
   - GitHub: 5000 requests/hour (authenticated)
   - GitLab: 2000 requests/minute (self-hosted configurable)

2. **Check remaining quota:**
   ```bash
   # GitHub
   gh api rate_limit

   # GitLab
   curl -H "PRIVATE-TOKEN: $GITLAB_TOKEN" https://gitlab.com/api/v4/user
   # Check RateLimit-Remaining header
   ```

3. **Reduce API calls:**
   - Use CLI over API when possible
   - Cache results
   - Avoid polling too frequently

4. **For self-hosted GitLab:**
   - Contact admin to increase limits
   - Whitelist server IP

### Empty Comments

**Symptom:** Comments dialog shows "No comments found" but MR has comments

**Solutions:**

1. **Verify MR has comments:**
   ```bash
   # GitHub
   gh pr view <number>

   # GitLab
   glab mr view <number>
   ```

2. **Check comment types supported:**
   - General comments (conversation)
   - Inline review comments (code reviews)
   - System notes may not appear

3. **Refresh comments** in Vibe Kanban UI

4. **Check API response:**
   - Enable debug logging
   - Verify API returns comments

## Provider Detection Problems

### Wrong Provider Detected

**Symptom:** GitLab repo detected as GitHub (or vice versa)

**Solutions:**

1. **Verify remote URL format:**
   ```bash
   git remote -v
   ```

2. **Common patterns:**
   - GitHub: `github.com`
   - GitLab: `gitlab.com` or custom domain

3. **Fix remote URL if incorrect:**
   ```bash
   git remote set-url origin git@gitlab.com:group/project.git
   ```

4. **Restart Vibe Kanban** after changing remote

### Unknown Provider

**Error:** `Unknown provider for URL: ...`

**Solutions:**

1. **Verify provider supported:**
   - ✅ GitHub (github.com)
   - ✅ GitLab (gitlab.com, self-hosted)
   - ❌ Bitbucket (not yet supported)
   - ❌ Azure DevOps (not yet supported)

2. **Check remote URL format:**
   ```bash
   git remote -v
   # Should be HTTPS or SSH format
   ```

3. **Fix malformed URL:**
   ```bash
   # Correct formats:
   git remote set-url origin git@gitlab.com:owner/repo.git
   git remote set-url origin https://github.com/owner/repo.git
   ```

### Subgroups Not Working (GitLab)

**Error:** `Repository not found` with nested groups

**Solutions:**

1. **Verify subgroup URL format:**
   ```bash
   # Correct: group/subgroup/project
   git remote -v
   ```

2. **Encode URL if needed:**
   - Some special characters need encoding
   - Spaces should be `%20`

3. **Check API path:**
   ```bash
   # Test project access
   glab api /projects/group%2Fsubgroup%2Fproject
   ```

## Network and Connectivity

### Connection Timeout

**Error:** `Connection timeout` or `Network unreachable`

**Solutions:**

1. **Check internet connectivity:**
   ```bash
   ping github.com
   ping gitlab.com
   ```

2. **Check DNS resolution:**
   ```bash
   nslookup github.com
   nslookg gitlab.com
   ```

3. **Check firewall rules:**
   - Allow outbound HTTPS (port 443)
   - Allow outbound SSH (port 22) if using SSH remotes

4. **Check proxy settings:**
   ```bash
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

5. **Configure git proxy if needed:**
   ```bash
   git config --global http.proxy http://proxy.example.com:8080
   git config --global https.proxy https://proxy.example.com:8080
   ```

### SSL Certificate Errors

**Error:** `SSL certificate problem: unable to get local issuer certificate`

**Solutions:**

1. **Update CA certificates:**
   ```bash
   # macOS
   brew upgrade ca-certificates

   # Linux (Debian/Ubuntu)
   sudo apt update && sudo apt upgrade ca-certificates

   # Windows
   certutil -generateSSTFromWWW <url>
   ```

2. **For self-signed certificates:**
   - Add CA cert to system trust store (see [Self-Hosted GitLab Guide](./self-hosted-gitlab.md#ssltls-configuration))
   - Or skip verification (not recommended):
     ```bash
     export GITLAB_INSECURE=true
     ```

3. **Check certificate validity:**
   ```bash
   openssl s_client -connect gitlab.example.com:443 -showcerts
   ```

### Proxy Issues

**Error:** `407 Proxy Authentication Required` or `Bad Gateway`

**Solutions:**

1. **Configure git proxy:**
   ```bash
   git config --global http.proxy http://username:password@proxy.example.com:8080
   git config --global https.proxy http://username:password@proxy.example.com:8080
   ```

2. **Configure CLI proxy:**
   ```bash
   export HTTP_PROXY=http://proxy.example.com:8080
   export HTTPS_PROXY=http://proxy.example.com:8080
   ```

3. **Bypass proxy for internal hosts:**
   ```bash
   export NO_PROXY=localhost,127.0.0.1,gitlab.internal.com
   ```

4. **Test proxy connection:**
   ```bash
   curl -x http://proxy.example.com:8080 https://github.com
   ```

## Self-Hosted Instance Issues

See [Self-Hosted GitLab Configuration](./self-hosted-gitlab.md#troubleshooting) for detailed troubleshooting of self-hosted instances.

### Common Self-Hosted Problems

- **Connection refused** - Firewall blocking access
- **API v4 not found** - API disabled or unsupported version
- **SSL certificate errors** - Self-signed certificates
- **Rate limiting** - Custom limits too restrictive

## Performance Problems

### Slow MR Creation

**Symptom:** Creating merge requests takes > 10 seconds

**Solutions:**

1. **Use CLI over API:**
   - CLI generally faster
   - Verify CLI installed and authenticated

2. **Check network latency:**
   ```bash
   ping github.com
   ping gitlab.com
   ```

3. **Check API response times:**
   ```bash
   time gh api user
   time glab api /user
   ```

4. **For self-hosted:**
   - Check GitLab server load
   - Verify network path optimized

### Slow Comment Loading

**Symptom:** Comments take > 5 seconds to load

**Solutions:**

1. **Check number of comments:**
   - Large MRs with 100+ comments slow
   - Consider pagination

2. **Check network:**
   ```bash
   # Test API speed
   time gh api repos/owner/repo/pulls/123/comments
   time glab api /projects/:id/merge_requests/:mr/notes
   ```

3. **Clear cache:**
   - Restart Vibe Kanban
   - Clear browser cache if applicable

## Common Error Messages

### "glab: command not found"

**Fix:** Install glab CLI (see [GitLab Setup](./gitlab-setup.md#installation))

### "gh: command not found"

**Fix:** Install gh CLI (see [GitHub documentation](https://cli.github.com/))

### "401 Unauthorized"

**Fix:** Re-authenticate or regenerate token

### "403 Forbidden"

**Fix:** Check repository permissions and token scopes

### "404 Not Found"

**Fix:** Verify repository exists and remote URL correct

### "422 Unprocessable Entity"

**Fix:** Check request body (usually invalid branch names or missing fields)

### "429 Too Many Requests"

**Fix:** Wait for rate limit reset or reduce API call frequency

### "500 Internal Server Error"

**Fix:** Provider issue - try again later or check status page

### "502 Bad Gateway" / "503 Service Unavailable"

**Fix:** Provider downtime - check status page:
- GitHub: https://www.githubstatus.com/
- GitLab.com: https://status.gitlab.com/

## Debug Logging

Enable debug logging for more detailed error information:

### Environment Variables

```bash
# Enable debug logging
export RUST_LOG=debug

# Or more specific
export RUST_LOG=services::git_provider=debug
```

### Check Logs

Vibe Kanban logs include provider operations:

```
DEBUG services::git_provider: Detecting provider from URL
DEBUG services::git_provider::gitlab: Creating MR via glab CLI
DEBUG services::git_provider::gitlab: glab CLI failed, falling back to API
```

Review logs to identify exact failure point.

## Getting Help

If troubleshooting doesn't resolve your issue:

1. **Check existing issues:**
   - [GitHub Issues](https://github.com/BloopAI/vibe-kanban/issues)
   - Search for similar problems

2. **Open discussion:**
   - [GitHub Discussions](https://github.com/BloopAI/vibe-kanban/discussions)
   - Provide full error message and steps to reproduce

3. **File bug report:**
   - Include Vibe Kanban version
   - Include provider type (GitHub/GitLab)
   - Include sanitized logs (remove tokens!)
   - Include steps to reproduce

### Information to Provide

When reporting issues, include:

- Vibe Kanban version: `npx vibe-kanban --version`
- Operating system: macOS/Linux/Windows version
- Provider: GitHub or GitLab (cloud/self-hosted)
- CLI version: `gh --version` or `glab --version`
- Git remote URL format (sanitized)
- Full error message
- Steps to reproduce
- Relevant logs (sanitized)

**Important:** Remove all tokens and sensitive data before sharing logs!

## Related Documentation

- [GitLab Setup Guide](./gitlab-setup.md)
- [Self-Hosted GitLab Configuration](./self-hosted-gitlab.md)
- [API Documentation](./api/merge-requests.md)

## Quick Reference

### Authentication Commands

```bash
# GitHub
gh auth login
gh auth status
gh auth refresh

# GitLab
glab auth login
glab auth status
glab auth logout
```

### Testing Commands

```bash
# Test GitHub API
gh api user
gh pr list

# Test GitLab API
glab api /user
glab mr list

# Test with curl
curl -H "Authorization: Bearer $GH_TOKEN" https://api.github.com/user
curl -H "PRIVATE-TOKEN: $GITLAB_TOKEN" https://gitlab.com/api/v4/user
```

### Environment Variables

```bash
# GitHub
export GH_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"
export GITHUB_TOKEN="ghp_xxxxxxxxxxxxxxxxxxxx"

# GitLab
export GITLAB_TOKEN="glpat-xxxxxxxxxxxxxxxxxxxx"
export GITLAB_BASE_URL="https://gitlab.com/api/v4"

# Debug
export RUST_LOG=debug
```
