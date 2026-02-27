-- Add optional first and last name to users for display name (default: fallback to email).
ALTER TABLE users ADD COLUMN first_name TEXT;
ALTER TABLE users ADD COLUMN last_name TEXT;
