APP_NAME = boardtask

.PHONY: setup hosts dev caddy run build test migrate clean

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

# Run all tests
test:
	cargo test

# Create the SQLite database and run all migrations
migrate:
	sqlx database create --database-url sqlite:$(APP_NAME).db
	sqlx migrate run --database-url sqlite:$(APP_NAME).db

# Remove build artifacts and the local database
clean:
	cargo clean
	rm -f $(APP_NAME).db $(APP_NAME).db-shm $(APP_NAME).db-wal
