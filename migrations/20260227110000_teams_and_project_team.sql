-- Teams and team membership; projects require a team.
-- Every org gets a default team; existing org members are added to that team; existing projects get team_id set.

-- Teams (one per org at least; default team created per org)
CREATE TABLE IF NOT EXISTS teams (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_teams_organization_id ON teams(organization_id);

-- Team membership (user can be in many teams)
CREATE TABLE IF NOT EXISTS team_members (
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (team_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_team_members_team_id ON team_members(team_id);
CREATE INDEX IF NOT EXISTS idx_team_members_user_id ON team_members(user_id);

-- Backfill: one default team per existing organization (id = deterministic from org id for repeatability)
INSERT INTO teams (id, organization_id, name, created_at)
SELECT 'team_' || id, id, name, strftime('%s','now') FROM organizations
WHERE NOT EXISTS (SELECT 1 FROM teams t WHERE t.organization_id = organizations.id);

-- Add team_id to projects (nullable for migration; app enforces required)
ALTER TABLE projects ADD COLUMN team_id TEXT REFERENCES teams(id);
UPDATE projects SET team_id = (SELECT id FROM teams t WHERE t.organization_id = projects.organization_id LIMIT 1);
CREATE INDEX IF NOT EXISTS idx_projects_team_id ON projects(team_id);

-- Backfill team_members: add every org member to their org's default team
INSERT OR IGNORE INTO team_members (team_id, user_id, created_at)
SELECT t.id, om.user_id, strftime('%s','now')
FROM organization_members om
JOIN teams t ON t.organization_id = om.organization_id;
