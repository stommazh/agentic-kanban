#!/bin/bash
# Sync fork with upstream vibe-kanban
# Usage: ./scripts/sync-upstream.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "🔄 Syncing with upstream vibe-kanban..."

# Ensure upstream remote exists
if ! git remote | grep -q "^upstream$"; then
    echo "📌 Adding upstream remote..."
    git remote add upstream https://github.com/BloopAI/vibe-kanban.git
fi

# Fetch upstream
echo "📥 Fetching upstream..."
git fetch upstream

# Check current branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "⚠️  You are on branch '$CURRENT_BRANCH', not 'main'"
    read -p "Switch to main? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git checkout main
    else
        echo "❌ Aborting. Please checkout main first."
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "❌ You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Show what's new
echo ""
echo "📊 Changes from upstream:"
git log --oneline HEAD..upstream/main | head -20
COMMIT_COUNT=$(git rev-list --count HEAD..upstream/main 2>/dev/null || echo "0")
echo "   Total: $COMMIT_COUNT new commits"
echo ""

if [ "$COMMIT_COUNT" = "0" ]; then
    echo "✅ Already up to date with upstream!"
    exit 0
fi

# Confirm merge
read -p "Merge these changes? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Aborted."
    exit 1
fi

# Merge upstream
echo "🔀 Merging upstream/main..."
if git merge upstream/main --no-edit; then
    echo ""
    echo "✅ Successfully merged upstream changes!"
    echo ""
    echo "Next steps:"
    echo "  1. Build: cargo build --release --bin server"
    echo "  2. Test:  cargo test --package services -- git_provider"
    echo "  3. Push:  git push origin main"
else
    echo ""
    echo "⚠️  Merge conflicts detected!"
    echo ""
    echo "Resolve conflicts using these guidelines:"
    echo "  - For gitlab.rs, gitlab/*, worktree_manager.rs → keep OURS"
    echo "  - For docs/gitlab*.md → keep OURS"
    echo "  - For everything else → usually keep THEIRS"
    echo ""
    echo "Commands:"
    echo "  git checkout --ours <file>    # Keep our version"
    echo "  git checkout --theirs <file>  # Keep their version"
    echo "  git add <file>                # Mark resolved"
    echo "  git commit                    # Complete merge"
    exit 1
fi
