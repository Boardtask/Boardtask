import { test, expect } from '@playwright/test';

test.describe('smoke', () => {
  test('root redirects to login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/login/);
  });
});
