-- Single starter migration: full schema, indices, and system data.
-- Tables and indexes use IF NOT EXISTS so this is safe for fresh installs and for
-- existing DBs that already applied the previous migrations (squash scenario).

-- Settings (legacy/placeholder)
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Organizations (no deps)
CREATE TABLE IF NOT EXISTS organizations (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Users (depends on organizations)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    email_verified_at INTEGER,
    organization_id TEXT NOT NULL REFERENCES organizations(id)
);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_organization_id ON users(organization_id);

-- Organization members
CREATE TABLE IF NOT EXISTS organization_members (
    organization_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'member', 'viewer')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (organization_id, user_id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_organization_id ON sessions(organization_id);

-- Email verification tokens
CREATE TABLE IF NOT EXISTS email_verification_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_token ON email_verification_tokens(token);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_expires_at ON email_verification_tokens(expires_at);

-- Password reset tokens
CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_token ON password_reset_tokens(token);
CREATE INDEX IF NOT EXISTS idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);

-- Projects
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_projects_user_id ON projects(user_id);
CREATE INDEX IF NOT EXISTS idx_projects_organization_id ON projects(organization_id);

-- Node types
CREATE TABLE IF NOT EXISTS node_types (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_node_types_system_unique ON node_types(name) WHERE user_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_node_types_user_unique ON node_types(user_id, name) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_node_types_user ON node_types(user_id);

-- Task statuses (system rows inserted below)
CREATE TABLE IF NOT EXISTS task_statuses (
    id TEXT PRIMARY KEY,
    organization_id TEXT,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_statuses_system_unique ON task_statuses(name) WHERE organization_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_statuses_org_unique ON task_statuses(organization_id, name) WHERE organization_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_task_statuses_org ON task_statuses(organization_id);

INSERT OR IGNORE INTO task_statuses (id, organization_id, name, sort_order, created_at) VALUES
  ('01JSTATUS00000000TODO0000', NULL, 'To do', 0, unixepoch()),
  ('01JSTATUS00000000INPROG00', NULL, 'In progress', 1, unixepoch()),
  ('01JSTATUS00000000DONE0000', NULL, 'Done', 2, unixepoch());

-- Project slots
CREATE TABLE IF NOT EXISTS project_slots (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_project_slots_project_name ON project_slots(project_id, name);
CREATE INDEX IF NOT EXISTS idx_project_slots_project_id ON project_slots(project_id);

-- Nodes
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    node_type_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER,
    estimated_minutes INTEGER,
    status_id TEXT NOT NULL DEFAULT '01JSTATUS00000000TODO0000',
    slot_id TEXT REFERENCES project_slots(id) ON DELETE SET NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (node_type_id) REFERENCES node_types(id)
);
CREATE INDEX IF NOT EXISTS idx_nodes_project ON nodes(project_id);
CREATE INDEX IF NOT EXISTS idx_nodes_node_type ON nodes(node_type_id);
CREATE INDEX IF NOT EXISTS idx_nodes_status_id ON nodes(status_id);
CREATE INDEX IF NOT EXISTS idx_nodes_slot_id ON nodes(slot_id);

-- Node edges
CREATE TABLE IF NOT EXISTS node_edges (
    parent_id TEXT NOT NULL,
    child_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (parent_id, child_id),
    FOREIGN KEY (parent_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (child_id) REFERENCES nodes(id) ON DELETE CASCADE,
    CHECK (parent_id != child_id)
);
CREATE INDEX IF NOT EXISTS idx_node_edges_parent ON node_edges(parent_id);
CREATE INDEX IF NOT EXISTS idx_node_edges_child ON node_edges(child_id);

-- Integrations (registry; populated by seed)
CREATE TABLE IF NOT EXISTS integrations (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Per-org integration enablement
CREATE TABLE IF NOT EXISTS organization_integrations (
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    integration_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (organization_id, integration_id)
);
CREATE INDEX IF NOT EXISTS idx_organization_integrations_organization_id ON organization_integrations(organization_id);
CREATE INDEX IF NOT EXISTS idx_organization_integrations_integration_id ON organization_integrations(integration_id);
