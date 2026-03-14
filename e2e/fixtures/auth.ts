import { execSync } from 'child_process';
import * as path from 'path';
import { expect } from '@playwright/test';
import type { Page } from '@playwright/test';

const dbPath = path.join(process.cwd(), '..', 'boardtask-e2e.db');

export function getVerificationToken(): string {
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

export async function signUpAndVerify(
  page: Page,
  email: string,
  password: string,
  firstName = 'E2E',
  lastName = 'Test'
): Promise<void> {
  await page.goto('/signup');
  await page.getByLabel(/first name/i).fill(firstName);
  await page.getByLabel(/last name/i).fill(lastName);
  await page.getByLabel(/email/i).first().fill(email);
  await page.getByLabel(/^password$/i).fill(password);
  await page.getByLabel(/confirm password/i).fill(password);
  await page.getByRole('button', { name: /create account/i }).click();
  await expect(page).toHaveURL(/\/check-email/, { timeout: 15000 });

  const token = getVerificationToken();
  await page.goto(`/verify-email?token=${token}`);
  await expect(page).toHaveURL(/\/app($|\/)/);
}

export async function login(
  page: Page,
  email: string,
  password: string
): Promise<void> {
  await page.goto('/login');
  await page.getByLabel(/email/i).fill(email);
  await page.getByLabel(/^password$/i).fill(password);
  await page.getByRole('button', { name: /log in/i }).click();
  await expect(page).toHaveURL(/\/app($|\/)/);
}
