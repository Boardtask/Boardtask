-- Per-project default view mode when opening from the projects list.
ALTER TABLE projects ADD COLUMN default_view_mode TEXT NOT NULL DEFAULT 'graph' CHECK(default_view_mode IN ('graph', 'list'));
