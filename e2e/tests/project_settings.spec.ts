import { test, expect } from '@playwright/test';

test.describe('project default view mode', () => {
  test('change default view from graph settings, then project link lands on list view', async ({
    page,
  }) => {
    // Already authenticated via storageState
    await page.goto('/app/projects/new');

    await page.getByLabel(/project title/i).fill('Default View Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    // Click project - should land on graph view (default)
    await page.getByRole('link', { name: /default view test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/); // graph route (no /list)

    // Get project ID, PATCH via API (settings drawer radios can be flaky in headless)
    const projectUrl = page.url();
    const projectId = projectUrl.split('/projects/')[1]?.split('/')[0]?.split('?')[0];
    expect(projectId).toBeTruthy();
    const patchResponse = await page.request.patch(`/api/projects/${projectId}`, {
      data: { default_view_mode: 'list' },
      headers: { 'Content-Type': 'application/json' },
    });
    expect(patchResponse.ok()).toBeTruthy();

    // Go back to projects list (full reload to get updated hrefs from server)
    await page.getByRole('link', { name: /back to projects/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/);
    await page.waitForLoadState('networkidle');

    // Click project again - should land on list view (href includes /list when default is list)
    const projectLink = page.getByRole('link', { name: /default view test/i }).first();
    await expect(projectLink).toHaveAttribute('href', /\/list/);
    await projectLink.click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+\/list/);
  });

  test('graph view settings drawer has no defaultViewMode console errors', async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('defaultViewMode') && text.includes('not defined')) {
        errors.push(text);
      }
    });

    // Already authenticated via storageState
    await page.goto('/app/projects/new');

    await page.getByLabel(/project title/i).fill('Console Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    await page.getByRole('link', { name: /console test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);

    // Open settings drawer (triggers Alpine binding of defaultViewMode radios)
    await page.getByRole('button', { name: /project settings/i }).click();
    await page.waitForTimeout(500);

    expect(errors, errors.length ? `Console had defaultViewMode errors: ${errors.join('; ')}` : '').toHaveLength(0);
  });

  test('change default view from list settings, then project link lands on graph view', async ({
    page,
  }) => {
    // Already authenticated via storageState
    await page.goto('/app/projects/new');

    await page.getByLabel(/project title/i).fill('List to Graph Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    // Click project -> lands on graph view, then navigate to list view via List view button
    await page.getByRole('link', { name: /list to graph test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);
    await page.getByRole('link', { name: /list view/i }).click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+\/list/);

    // Open settings from list view, set default to Graph
    await page.getByRole('button', { name: /project settings/i }).click();
    await page.getByRole('radio', { name: /^graph$/i }).check();
    await page.waitForTimeout(1500); // Allow PATCH to complete
    await page.getByRole('button', { name: /^done$/i }).click();

    // Go back to projects list
    await page.getByRole('link', { name: /back to projects/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/);

    // Click project - should land on graph view
    await page.getByRole('link', { name: /list to graph test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);
    await expect(page).not.toHaveURL(/\/list/);
  });

  test('new project loads in graph view by default, list link navigates to list view', async ({
    page,
  }) => {
    // Already authenticated via storageState
    await page.goto('/app/projects/new');

    await page.getByLabel(/project title/i).fill('Graph Default Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    // Click project - should land on graph view (default)
    await page.getByRole('link', { name: /graph default test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);
    await expect(page).not.toHaveURL(/\/list/);

    // List view link should be visible and work
    await page.getByRole('link', { name: /list view/i }).click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+\/list/);
  });
});
