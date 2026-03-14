import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import * as path from 'path';

function getVerificationToken(): string {
  const dbPath = path.join(process.cwd(), '..', 'boardtask-e2e.db');
  const token = execSync(
    `sqlite3 "${dbPath}" "SELECT token FROM email_verification_tokens ORDER BY created_at DESC LIMIT 1"`
  )
    .toString()
    .trim();
  if (!token) {
    throw new Error('No verification token found in database');
  }
  return token;
}

test.describe('auth', () => {
  test('user can sign up', async ({ page }) => {
    const email = `e2e-signup-${Date.now()}@example.com`;
    await page.goto('/signup');
    await page.getByLabel(/first name/i).fill('E2E');
    await page.getByLabel(/last name/i).fill('Tester');
    await page.getByLabel(/email/i).first().fill(email);
    await page.getByLabel(/^password$/i).fill('Password123');
    await page.getByLabel(/confirm password/i).fill('Password123');
    await page.getByRole('button', { name: /create account/i }).click();

    await expect(page).toHaveURL(/\/check-email/, { timeout: 15000 });
  });

  test('user can sign up, verify via DB token, login, and logout', async ({
    page,
  }) => {
    const email = `e2e-flow-${Date.now()}@example.com`;
    const password = 'Password123';

    // Sign up
    await page.goto('/signup');
    await page.getByLabel(/first name/i).fill('E2E');
    await page.getByLabel(/last name/i).fill('Flow');
    await page.getByLabel(/email/i).first().fill(email);
    await page.getByLabel(/^password$/i).fill(password);
    await page.getByLabel(/confirm password/i).fill(password);
    await page.getByRole('button', { name: /create account/i }).click();

    await expect(page).toHaveURL(/\/check-email/, { timeout: 15000 });

    // Verify via DB token
    const token = getVerificationToken();
    await page.goto(`/verify-email?token=${token}`);

    // Verify redirects to /app (dashboard)
    await expect(page).toHaveURL(/\/app/);

    // Logout (redirects to / then / redirects unauthenticated to /login)
    await page.goto('/app/account');
    await page.getByRole('button', { name: /log out/i }).click();
    await expect(page).toHaveURL(/\/login/);

    // Login
    await page.goto('/login');
    await page.getByLabel(/email/i).fill(email);
    await page.getByLabel(/^password$/i).fill(password);
    await page.getByRole('button', { name: /log in/i }).click();

    await expect(page).toHaveURL(/\/app/);

    // Logout again (redirects to / then / redirects unauthenticated to /login)
    await page.goto('/app/account');
    await page.getByRole('button', { name: /log out/i }).click();
    await expect(page).toHaveURL(/\/login/);
  });
});
