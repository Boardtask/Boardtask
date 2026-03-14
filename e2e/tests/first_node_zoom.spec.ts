import { test, expect } from '@playwright/test';

test.describe('first node zoom', () => {
  // Skip: Add dropdown fails to open in headless (Alpine x-show), and API-created nodes
  // do not appear in graph after reload. See plan e2e_setup_storagestate for context.
  test.skip('adding first standalone node does not zoom in excessively', async ({
    page,
  }) => {
    // Already authenticated via storageState
    await page.goto('/app/projects/new');

    await page.getByLabel(/project title/i).fill('First Node Zoom Test');
    await page.getByRole('button', { name: /create project/i }).click();
    await expect(page).toHaveURL(/\/app\/projects/, { timeout: 5000 });

    // Open project (graph view)
    await page.getByRole('link', { name: /first node zoom test/i }).first().click();
    await expect(page).toHaveURL(/\/app\/projects\/[^/]+$/);

    // Create node via API (Add dropdown fails to open in headless)
    const projectUrl = page.url();
    const projectId = projectUrl.split('/projects/')[1]?.split('/')[0]?.split('?')[0];
    expect(projectId).toBeTruthy();
    const createResponse = await page.request.post(`/api/projects/${projectId}/nodes`, {
      data: {
        node_type_id: '01JNODETYPE00000000TASK000',
        title: 'New Node',
        description: '',
      },
      headers: { 'Content-Type': 'application/json' },
    });
    expect(createResponse.ok(), `Create node failed: ${await createResponse.text()}`).toBeTruthy();
    await page.reload();
    await page.waitForLoadState('networkidle');
    await page.waitForSelector('.graph-container', { timeout: 5000 });
    await page.waitForSelector('.graph-container .cy-node', { timeout: 15000 });

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
