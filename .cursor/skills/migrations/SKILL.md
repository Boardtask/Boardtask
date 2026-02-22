---
name: migrations
description: How to add and run database migrations without changing existing ones. Use when adding schema changes, new migrations, or when the user asks about migrations, schema, or sqlx migrate.
---

# Migrations

## Append-only rule

**Never change existing migration files.** Migrations are append-only. Once applied (or committed), a migration file is immutable. To fix a mistake, add a new migration that corrects it.

- Do not edit, rename, or delete files in `migrations/` that already exist.
- Do not alter the contents of existing `.sql` migration files.
- New schema changes go in a **new** file in `migrations/`.

## Adding a new migration

1. **Create one new file** in `migrations/` with a timestamp prefix and a short descriptive name:
   - Pattern: `YYYYMMDDHHMMSS_short_description.sql`
   - Example: `20260222100000_add_foo_to_bar.sql`

2. **Use additive, non-destructive SQL** (see [No data destruction](#no-data-destruction) below).

3. **Run migrations:** `make migrate` (or `sqlx migrate run --database-url sqlite:boardtask.db`).

## No data destruction

Migrations must never drop or delete user or business data:

- Prefer **additive** changes: new tables, `ADD COLUMN`, new indexes.
- To **remove** a table or column: do it in two steps. First migration: add new schema and migrate/backfill data; later migration: drop the old table/column only after the app uses the new structure and data is preserved.
- Do not `DROP TABLE` / `DROP COLUMN` / `DELETE` in a way that loses the only copy of data. Document any drop (e.g. in comments) with where the data now lives or why the drop is safe.
- Idempotent additive statements are fine: `INSERT OR IGNORE`, `ADD COLUMN ... DEFAULT`, etc.

Reference: `.cursor/rules/migrations-no-data-loss.mdc`.

## Project details

- Migrations live in: `migrations/`.
- Run: `make migrate` (creates DB if needed, then runs all migrations).
- Raw SQL is allowed only in `migrations/*.sql`; application and test code use the db layer in `src/app/db/`, not raw SQL.
