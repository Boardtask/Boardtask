import { defineConfig, devices } from '@playwright/test';
import * as path from 'path';

// Use project-local browser cache so tests work from any context (Cursor, CLI, CI).
process.env.PLAYWRIGHT_BROWSERS_PATH =
  process.env.PLAYWRIGHT_BROWSERS_PATH ||
  path.join(__dirname, '.playwright-browsers');

export default defineConfig({
  testDir: 'tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 1,
  workers: 1,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:3001',
    headless: true,
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command:
      'cd .. && make migrate-e2e && PORT=3001 APP_URL=http://localhost:3001 DATABASE_URL=sqlite:boardtask-e2e.db cargo run',
    url: 'http://localhost:3001',
    reuseExistingServer: process.env.CI !== 'true',
    timeout: 180000, // Allow time for cargo build + first run
  },
});
