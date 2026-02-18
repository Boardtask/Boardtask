-- Cleanup existing data to allow adding NOT NULL columns
DELETE FROM users;
DELETE FROM projects;
DELETE FROM nodes;
DELETE FROM node_edges;

-- Add organizations table
CREATE TABLE organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE organization_members (
    organization_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member', 'viewer')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (organization_id, user_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

ALTER TABLE users ADD COLUMN organization_id TEXT NOT NULL REFERENCES organizations(id);
CREATE INDEX idx_users_organization_id ON users(organization_id);

ALTER TABLE projects ADD COLUMN organization_id TEXT NOT NULL REFERENCES organizations(id);
CREATE INDEX idx_projects_organization_id ON projects(organization_id);
