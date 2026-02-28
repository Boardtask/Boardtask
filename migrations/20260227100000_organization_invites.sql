-- Organization invites: token-based invite links for adding members to an org.
CREATE TABLE IF NOT EXISTS organization_invites (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member', 'viewer')),
    invited_by_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_organization_invites_token ON organization_invites(token);
CREATE INDEX IF NOT EXISTS idx_organization_invites_organization_id ON organization_invites(organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_invites_expires_at ON organization_invites(expires_at);
