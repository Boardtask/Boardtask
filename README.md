# Boardtask

A Rust web application built with [Axum](https://github.com/tokio-rs/axum), [SQLite](https://www.sqlite.org/), and [Askama](https://github.com/djc/askama).

## Prerequisites

- **Rust** — install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **SQLite3** — pre-installed on macOS. On Linux: `sudo apt install sqlite3 libsqlite3-dev`
- **Caddy** — install via Homebrew:
  ```bash
  brew install caddy
  ```

## Quick Start

### 1. Install dev tools

```bash
make setup
```

This installs:
- **sqlx-cli** — database migration runner
- **cargo-watch** — auto-recompile on file changes

### 2. Configure local domain

Add `boardtask.local` to your hosts file:

```bash
make hosts
```

This adds `127.0.0.1 boardtask.local` to `/etc/hosts` (requires sudo password).

### 3. Configure environment

Create a `.env` file in the project root (already created with defaults):

```
DATABASE_URL=sqlite:boardtask.db
```

### 4. Set up the database

```bash
make migrate
```

This creates the SQLite file and runs all migrations in `migrations/`.

### 5. Run in development mode

You need **two terminals**:

**Terminal 1** — Start Caddy (HTTPS reverse proxy + static files):
```bash
make caddy
```

**Terminal 2** — Start Axum (the Rust app):
```bash
make dev
```

The server starts at **https://boardtask.local** and automatically restarts when you edit any source file.

For a single run without watching:

```bash
make run
```

## Building for Production

```bash
make build
```

The optimised binary is at `target/release/boardtask`. Run it with:

```bash
DATABASE_URL=sqlite:boardtask.db ./target/release/boardtask
```

## Project Structure

```
boardtask/
├── Cargo.toml              # Dependencies and project metadata
├── askama.toml              # Askama template configuration
├── Caddyfile                # Caddy reverse proxy config
├── Makefile                 # Dev workflow commands
├── .env                     # Environment variables (not committed)
├── migrations/              # SQL migration files (run in order)
│   └── 20260207000000_initial.sql
├── public/                  # Static assets (served by Caddy)
│   ├── css/
│   │   └── app.css
│   └── js/
│       └── app.mjs
└── src/
    ├── main.rs              # Entry point — boots the server
    ├── app/                 # Router + shared AppState
    │   ├── mod.rs
    │   ├── db/               # DB models, queries
    │   ├── domain/           # Domain types
    │   ├── error.rs          # Error handling
    │   └── features/         # Feature slices
    │       └── auth/         # Authentication
    │           ├── mod.rs
    │           ├── service.rs # Auth service layer
    │           ├── signup.rs  # Signup form + handlers + routes
    │           ├── signup.html
    │           ├── login.rs   # Login form + handlers + routes
    │           └── login.html
    └── site/                 # Marketing website
        ├── mod.rs
        ├── home.rs           # Home feature slice (handlers + routes)
        └── home.html         # Colocated template
```

## Architecture & Conventions

This project uses vertical slice architecture with colocated Askama templates. For detailed conventions (how to add features, auth layering, template paths), see `.cursor/rules/`.

## Common Commands

| Command        | What it does                              |
| -------------- | ----------------------------------------- |
| `make setup`   | Install sqlx-cli and cargo-watch          |
| `make hosts`   | Add boardtask.local to /etc/hosts            |
| `make dev`     | Run Axum with auto-reload                 |
| `make caddy`   | Run Caddy reverse proxy (separate terminal) |
| `make run`     | Run Axum                             |
| `make build`   | Build release binary                       |
| `make migrate` | Create DB + run migrations                 |
| `make clean`   | Remove build artifacts and local database |

## How It Works

### Request Flow

1. Browser requests `https://boardtask.local`
2. Caddy checks if the request matches a file in `public/` (CSS, JS, images)
3. If yes → Caddy serves it directly
4. If no → Caddy reverse proxies to Axum on `localhost:3000`
5. Axum handles the request, renders Askama templates, queries SQLite

### SSL in Development

Caddy automatically provisions a local root CA and TLS certificate for `boardtask.local` on first run. It may prompt for your system password to trust the CA. After that, `https://boardtask.local` works without any certificate warnings.

### Static Files

Static files (CSS, MJS) are served by Caddy directly from `public/`, never hitting Axum. This keeps the Rust code focused on dynamic routes only.
## Troubleshooting

**"Failed to bind to port 3000"** — Another process is using port 3000. Kill it or change the port in `src/main.rs`.

**"DATABASE_URL must be set"** — Make sure `.env` exists and contains `DATABASE_URL=sqlite:boardtask.db`.

**"boardtask.local not found"** — Run `make hosts` to add it to `/etc/hosts`.

**Caddy SSL errors** — On first run, Caddy may need your password to install its root CA. Enter it when prompted.
