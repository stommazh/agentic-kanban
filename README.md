# Agentic Kanban (Fork of Vibe-Kanban)

This is a dedicated fork of [Vibe-Kanban](https://github.com/BloopAI/vibe-kanban), optimized for specific team workflows and GitLab integration.

## Updates & Changelog

- **Simplified GitLab Integration**: Core MR operations (create, list, status) now use `glab` CLI for a seamless, credential-free experience.
- **Minimal GitLab API Client**: Added specific support for fetching MR comments via optional API token in settings.
- **Git Submodule Support**: Fixed logic for repositories where `.git` is a file (gitlink), enabling `git diff` and worktrees in submodules.
- **Self-Hosted GitLab Support**: Improved auto-detection for custom hostnames and nested groups/subgroups.
- **Development Workflow**: Added `./scripts/sync-upstream.sh` for easy upstream updates and `./scripts/dev.sh` for simplified local setup.
- **Dynamic Port Fix**: Resolution for backend/frontend communication during development server startup.

## Development & Maintenance

For detailed instructions on maintaining this fork, see:
- [Fork Workflow Guide](./docs/FORK-WORKFLOW.md)
- [GitLab Setup Documentation](./docs/gitlab-setup.md)
- [Self-Hosted GitLab Guide](./docs/self-hosted-gitlab.md)

### Quick Commands
```bash
# Start dev servers (backend + frontend)
./scripts/dev.sh

# Sync with original project
./scripts/sync-upstream.sh
```
