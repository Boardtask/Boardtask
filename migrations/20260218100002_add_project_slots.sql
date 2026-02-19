CREATE TABLE project_slots (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_project_slots_project_name ON project_slots(project_id, name);
CREATE INDEX idx_project_slots_project_id ON project_slots(project_id);
