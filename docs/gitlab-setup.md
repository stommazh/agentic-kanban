# GitLab Setup Guide

Vibe Kanban supports GitLab.com and self-hosted GitLab instances for creating merge requests and managing your repositories.

## Quick Start

GitLab support works with just `glab` CLI:

```bash
# 1. Install glab CLI
brew install glab  # macOS

# 2. Authenticate with GitLab
glab auth login

# 3. That's it! MR operations just work
```

## How It Works

| Feature | Method | Setup Required |
|---------|--------|----------------|
| Git Diff | Local git | None (always works) |
| Create MR | glab CLI | `glab auth login` |
| List MRs | glab CLI | `glab auth login` |
| MR Status | glab CLI | `glab auth login` |
| MR Comments | GitLab API | Configure token in Settings |

## Installation

### glab CLI (Required for MR Operations)

**macOS:**
```bash
brew install glab
```

**Linux:**
```bash
# Debian/Ubuntu
sudo apt install glab

# Fedora/RHEL
sudo dnf install glab

# Arch Linux
sudo pacman -S glab
```

**Windows:**
```bash
# Using Scoop
scoop install glab

# Using Chocolatey
choco install glab
```

For other methods, see [glab releases](https://gitlab.com/gitlab-org/cli/-/releases).

### Authentication

```bash
glab auth login
```

Follow the prompts:
1. Select **GitLab.com** or enter your self-hosted instance URL
2. Choose **HTTPS** protocol
3. Authenticate via **browser OAuth** (recommended)

Verify:
```bash
glab auth status
```

## MR Comments (Optional)

To view MR comments in the app, configure a GitLab Personal Access Token:

1. Go to **Settings > Integrations > GitLab** in the app
2. Enter your Personal Access Token

### Creating a Token

1. Visit [GitLab Access Tokens](https://gitlab.com/-/profile/personal_access_tokens)
2. Create token with scopes:
   - ✅ `api` - API access
   - ✅ `read_repository` - Read repository
3. Copy the token to app settings

If no token is configured, MR comments won't be displayed (create/list/status still work).

## Self-Hosted GitLab

For self-hosted instances:

```bash
# Authenticate with custom hostname
glab auth login --hostname gitlab.example.com
```

Or set environment variable:
```bash
export GITLAB_BASE_URL="https://gitlab.example.com"
```

See [Self-Hosted GitLab Configuration](./self-hosted-gitlab.md) for details.

## Provider Auto-Detection

Vibe Kanban detects GitLab from git remote URLs:

- `https://gitlab.com/group/project.git`
- `git@gitlab.com:group/project.git`
- `https://gitlab.example.com/group/subgroup/project.git`

Check your remote:
```bash
git remote -v
```

## Features

- **Git Diff** - View changes (works with local git)
- **Create MRs** - Open MRs from the UI
- **List MRs** - View existing MRs
- **MR Status** - Monitor state (open/merged/closed)
- **MR Comments** - View comments (requires API token)
- **Subgroups** - Full support for nested groups

## Troubleshooting

### glab not found

Install glab CLI and restart the app.

### glab not authenticated

Run `glab auth login` and follow prompts.

### Comments not showing

Configure GitLab API token in **Settings > Integrations > GitLab**.

### Self-hosted not detected

Set `GITLAB_BASE_URL` environment variable or use `glab auth login --hostname`.

## Next Steps

- [Self-Hosted GitLab Configuration](./self-hosted-gitlab.md)
- [Troubleshooting Guide](./troubleshooting-git-providers.md)
