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

test.describe('first node zoom', () => {
  test('adding first standalone node does not zoom in excessively', async ({
    page,
  }) => {
    const email = `e2e-first-node-${Date.now()}@example.com`;
    const password = 'Password123';

    // Sign up and verify
    await page.goto('/signup');
    await page.getByLabel(/first name/i).fill('E2E');
    await page.getByLabel(/last name/i).fill('FirstNode');
    await page.getByLabel(/email/i).first().fill(email);
    await page.getByLabel(/^password$/i).fill(password);
    await page.getByLabel(/confirm password/i).fill(password);
    await page.getByRole('button', { name: /create account/i }).click();
    await expect(page).toHaveURL(/\/check-email/, { timeout: 15000 });

    const token = getVerificationToken();
    await page.goto(`/verify-email?token=${token}`);
    await expect(page).toHaveURL(/\/app($|\/)/);

    // Create project (empty graph)
    await page.goto('/app/projects/new');
    await page.getByLabel(/project title/i).fill('First Node Zoom Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    // Open project (graph view)
    await page.getByRole('link', { name: /first node zoom test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);

    // Wait for graph to load
    await page.waitForSelector('.graph-container', { timeout: 5000 });

    // Open add menu and add standalone node
    await page.getByRole('button', { name: /add node or group/i }).click();
    await page.getByRole('button', { name: /add standalone node/i }).click();

    // Wait for node to appear (cy-node is the HTML label class)
    await page.waitForSelector('.graph-container .cy-node', { timeout: 10000 });

    const node = page.locator('.graph-container .cy-node').first();
    await expect(node).toBeVisible();

    // Assert node is not over-zoomed: its height should be reasonable (not dominating viewport)
    // A single card is ~95px; if zoomed in huge it could be 400+ px
    const box = await node.boundingBox();
    const viewport = page.viewportSize();
    const viewportHeight = viewport?.height ?? 800;
    expect(box).toBeTruthy();
    expect(box!.height).toBeLessThan(viewportHeight * 0.5);
  });
});
