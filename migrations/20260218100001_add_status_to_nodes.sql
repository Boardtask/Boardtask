-- SQLite does not allow ADD COLUMN with both REFERENCES and DEFAULT; we rely on app logic for referential integrity.
ALTER TABLE nodes ADD COLUMN status_id TEXT NOT NULL DEFAULT '01JSTATUS00000000TODO0000';
CREATE INDEX idx_nodes_status_id ON nodes(status_id);
