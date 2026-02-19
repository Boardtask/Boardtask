-- Registry of allowed integrations (populated by seed).
CREATE TABLE integrations (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Per-org link: which integrations are enabled for an org (no settings column).
CREATE TABLE organization_integrations (
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    integration_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (organization_id, integration_id)
);

CREATE INDEX idx_organization_integrations_organization_id ON organization_integrations(organization_id);
CREATE INDEX idx_organization_integrations_integration_id ON organization_integrations(integration_id);
