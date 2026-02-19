ALTER TABLE nodes ADD COLUMN slot_id TEXT REFERENCES project_slots(id) ON DELETE SET NULL;
CREATE INDEX idx_nodes_slot_id ON nodes(slot_id);
