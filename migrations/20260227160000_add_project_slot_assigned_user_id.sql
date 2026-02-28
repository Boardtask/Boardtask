-- Add optional assigned user to project slots (user represents the slot / all nodes in that slot).
ALTER TABLE project_slots ADD COLUMN assigned_user_id TEXT REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_project_slots_assigned_user_id ON project_slots(assigned_user_id);
