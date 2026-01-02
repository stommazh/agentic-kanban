# Fork Development Workflow

This document explains how to maintain our custom fork of vibe-kanban while pulling updates from the original project.

## Repository Setup

```
origin   → git@github.com:stommazh/agentic-kanban.git  (our fork)
upstream → https://github.com/BloopAI/vibe-kanban.git  (original)
```

## Our Custom Changes

The following changes are unique to our fork:

### 1. GitLab Integration (Simplified)
- `crates/services/src/services/git_provider/gitlab.rs` - CLI-first approach
- `crates/services/src/services/git_provider/gitlab/api.rs` - Minimal API for comments
- `crates/services/src/services/git_provider/gitlab/cli.rs` - glab CLI wrapper
- `docs/gitlab-setup.md` - Setup documentation
- `docs/self-hosted-gitlab.md` - Self-hosted guide

### 2. Git Submodule Support
- `crates/services/src/services/worktree_manager.rs` - Fixed gitlink file handling

## Workflow: Sync with Upstream

### Quick Sync (Recommended)

```bash
# Run the sync script
./scripts/sync-upstream.sh
```

### Manual Steps

```bash
# 1. Fetch upstream changes
git fetch upstream

# 2. Checkout your main branch
git checkout main

# 3. Merge upstream (keeping our commits on top)
git merge upstream/main --no-edit

# 4. If conflicts occur, resolve them prioritizing our changes for:
#    - crates/services/src/services/git_provider/gitlab*
#    - crates/services/src/services/worktree_manager.rs
#    - docs/gitlab*.md

# 5. Push to origin
git push origin main
```

## Handling Merge Conflicts

### Priority Rules

| File Pattern | Priority |
|-------------|----------|
| `gitlab.rs`, `gitlab/*` | **Ours** (our implementation) |
| `worktree_manager.rs` | **Ours** (submodule fix) |
| `docs/gitlab*.md` | **Ours** (our docs) |
| Everything else | **Theirs** (upstream) |

### Conflict Resolution

```bash
# For files where we want OUR version:
git checkout --ours <file>
git add <file>

# For files where we want THEIR version:
git checkout --theirs <file>
git add <file>

# Continue merge
git commit
```

## Workflow: Feature Development

### Creating a Feature Branch

```bash
git checkout main
git pull origin main
git checkout -b feature/my-feature
# ... make changes ...
git push origin feature/my-feature
```

### After Upstream Sync

```bash
# Rebase feature branch on updated main
git checkout feature/my-feature
git rebase main
git push origin feature/my-feature --force-with-lease
```

## Testing After Sync

After syncing with upstream, always:

```bash
# 1. Build
cargo build --release --bin server

# 2. Run tests
cargo test --package services -- git_provider

# 3. Start app and verify
PORT=3001 ./target/release/server
```

## Tips

1. **Sync regularly** - Don't let your fork drift too far from upstream
2. **Small commits** - Keep your custom changes in focused commits
3. **Document changes** - Update this file when adding new customizations
4. **Test after merge** - Always run tests after syncing
