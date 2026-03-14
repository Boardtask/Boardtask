-- Add bio and preference columns to users for account settings.
ALTER TABLE users ADD COLUMN bio TEXT;
ALTER TABLE users ADD COLUMN email_notifications INTEGER NOT NULL DEFAULT 1;
ALTER TABLE users ADD COLUMN theme_mode TEXT NOT NULL DEFAULT 'light';
ALTER TABLE users ADD COLUMN language TEXT NOT NULL DEFAULT 'en-US';
