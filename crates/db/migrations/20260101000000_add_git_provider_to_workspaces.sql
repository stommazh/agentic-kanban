-- Add git_provider field to workspaces for caching provider detection
-- Values: 'github', 'gitlab', or NULL (auto-detect on demand)
ALTER TABLE workspaces ADD COLUMN git_provider TEXT;
