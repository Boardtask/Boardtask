CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    node_type_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (node_type_id) REFERENCES node_types(id)
);

CREATE INDEX idx_nodes_project ON nodes(project_id);
CREATE INDEX idx_nodes_node_type ON nodes(node_type_id);

CREATE TABLE node_edges (
    parent_id TEXT NOT NULL,
    child_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (parent_id, child_id),
    FOREIGN KEY (parent_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (child_id) REFERENCES nodes(id) ON DELETE CASCADE,
    CHECK (parent_id != child_id)
);

CREATE INDEX idx_node_edges_parent ON node_edges(parent_id);
CREATE INDEX idx_node_edges_child ON node_edges(child_id);