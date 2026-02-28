-- Add optional assignee (user) to nodes. Assignee must be org member of project's organization.
ALTER TABLE nodes ADD COLUMN assigned_user_id TEXT REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_nodes_assigned_user_id ON nodes(assigned_user_id);
