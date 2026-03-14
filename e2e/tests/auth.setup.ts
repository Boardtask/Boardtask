import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import { login, signUpAndVerify } from '../fixtures/auth';

test('auth setup', async ({ page }) => {
  const email = 'e2e@example.com';
  const password = 'Password123';

  // Try login first (user may exist from prior run)
  await page.goto('/login');
  await page.getByLabel(/email/i).fill(email);
  await page.getByLabel(/^password$/i).fill(password);
  await page.getByRole('button', { name: /log in/i }).click();

  const url = page.url();
  if (url.match(/\/app($|\/)/)) {
    // Login succeeded, already verified
  } else {
    // Login failed (user doesn't exist or unverified) - sign up and verify
    await signUpAndVerify(page, email, password, 'E2E', 'Setup');
  }

  // Ensure we're on /app
  await page.goto('/app');
  await expect(page).toHaveURL(/\/app($|\/)/);

  // Create .auth dir and save storage state
  const authDir = path.join(process.cwd(), '.auth');
  fs.mkdirSync(authDir, { recursive: true });
  await page.context().storageState({ path: path.join(authDir, 'user.json') });
});
