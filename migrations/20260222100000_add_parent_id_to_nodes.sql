-- Add optional parent_id to nodes for Cytoscape-style compounds (groups).
-- A group is any node whose id appears as another node's parent_id.
-- ON DELETE SET NULL: when a group node is deleted, children get parent_id set to NULL (ungrouped, not deleted).

ALTER TABLE nodes ADD COLUMN parent_id TEXT REFERENCES nodes(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_nodes_parent_id ON nodes(parent_id);
