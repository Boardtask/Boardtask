-- Initial migration.
-- A simple settings table to verify the migration system is working.
-- Replace or extend this with your real schema as the app grows.

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
