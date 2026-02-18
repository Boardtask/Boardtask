CREATE TABLE task_statuses (
    id TEXT PRIMARY KEY,
    organization_id TEXT,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE UNIQUE INDEX idx_task_statuses_system_unique
  ON task_statuses(name) WHERE organization_id IS NULL;

CREATE UNIQUE INDEX idx_task_statuses_org_unique
  ON task_statuses(organization_id, name) WHERE organization_id IS NOT NULL;

CREATE INDEX idx_task_statuses_org ON task_statuses(organization_id);

-- System statuses (fixed IDs for default and references)
INSERT INTO task_statuses (id, organization_id, name, sort_order, created_at) VALUES
  ('01JSTATUS00000000TODO0000', NULL, 'To do', 0, unixepoch()),
  ('01JSTATUS00000000INPROG00', NULL, 'In progress', 1, unixepoch()),
  ('01JSTATUS00000000DONE0000', NULL, 'Done', 2, unixepoch());
