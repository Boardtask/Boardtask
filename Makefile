APP_NAME = boardtask

.PHONY: setup hosts dev caddy run build test migrate clean db-reset-wal db-reset

# Install required dev tools
setup:
	@echo "Installing sqlx-cli..."
	cargo install sqlx-cli --no-default-features --features sqlite
	@echo "Installing cargo-watch..."
	cargo install cargo-watch
	@echo ""
	@echo "✓ Dev tools installed!"
	@echo "Don't forget to install Caddy: brew install caddy"

# Add $(APP_NAME).local to /etc/hosts
hosts:
	@if grep -q "$(APP_NAME).local" /etc/hosts; then \
		echo "$(APP_NAME).local already in /etc/hosts"; \
	else \
		echo "Adding $(APP_NAME).local to /etc/hosts (requires sudo)"; \
		echo "127.0.0.1 $(APP_NAME).local" | sudo tee -a /etc/hosts; \
	fi

# Run in dev mode with auto-reload on file changes
dev:
	cargo watch -x run

# Run Caddy reverse proxy (run this in a separate terminal)
caddy:
	caddy run

# Run once (no auto-reload)
run:
	cargo run

# Build optimised release binary
build:
	cargo build --release

# Run all tests (warnings as errors)
test:
	RUSTFLAGS="-D warnings" cargo test

# Create the SQLite database and run all migrations
migrate:
	sqlx database create --database-url sqlite:$(APP_NAME).db
	sqlx migrate run --database-url sqlite:$(APP_NAME).db

# Run database seeds (uses DATABASE_URL from .env, default sqlite:boardtask.db)
seed:
	cargo run --bin seed

# Force re-run all seeds
seed-force:
	cargo run --bin seed -- --force-all

# Clear SQLite WAL/shm files (fixes disk I/O error 522). Stop the app first.
db-reset-wal:
	rm -f $(APP_NAME).db-shm $(APP_NAME).db-wal
	@echo "WAL/shm cleared. Start the app again (e.g. make dev)."

# Remove corrupted/broken database so next run recreates it via migrations. Stop the app first. Data is lost.
db-reset:
	@echo "This will DELETE $(APP_NAME).db and WAL/shm. All data will be lost."
	@printf "Type 'yes' to confirm: " && read confirm && [ "$$confirm" = "yes" ] || (echo "Aborted."; exit 1)
	rm -f $(APP_NAME).db $(APP_NAME).db-shm $(APP_NAME).db-wal
	@echo "Database removed. Next 'make dev' or 'make run' will create a fresh DB and run migrations."

# Remove build artifacts and the local database
clean:
	cargo clean
	rm -f $(APP_NAME).db $(APP_NAME).db-shm $(APP_NAME).db-wal
