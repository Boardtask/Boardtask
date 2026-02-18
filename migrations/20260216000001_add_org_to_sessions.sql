-- Add organization_id to sessions table
-- We delete existing sessions to avoid issues with the new NOT NULL constraint
DELETE FROM sessions;

ALTER TABLE sessions ADD COLUMN organization_id TEXT NOT NULL REFERENCES organizations(id);
CREATE INDEX idx_sessions_organization_id ON sessions(organization_id);
