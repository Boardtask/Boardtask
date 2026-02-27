-- Backfill NULL first_name/last_name so app can treat them as required (non-optional).
UPDATE users SET first_name = '' WHERE first_name IS NULL;
UPDATE users SET last_name = '' WHERE last_name IS NULL;
