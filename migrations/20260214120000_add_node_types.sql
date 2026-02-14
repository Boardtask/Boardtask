CREATE TABLE node_types (
    id TEXT PRIMARY KEY,
    user_id TEXT,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_node_types_system_unique
  ON node_types(name) WHERE user_id IS NULL;

CREATE UNIQUE INDEX idx_node_types_user_unique
  ON node_types(user_id, name) WHERE user_id IS NOT NULL;

CREATE INDEX idx_node_types_user ON node_types(user_id);