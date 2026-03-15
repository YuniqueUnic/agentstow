import { expect, test } from '@playwright/test';

test('renders MCP runtime panel against the local workspace server', async ({ page }) => {
  await page.goto('/');

  const nav = page.getByRole('navigation', { name: '主导航' });
  await expect(nav.getByRole('button', { name: 'MCP', exact: true })).toBeVisible();
  await nav.getByRole('button', { name: 'MCP', exact: true }).click();

  await expect(page.getByText('Runtime Inspector')).toBeVisible();
  await expect(page.getByTestId('mcp-env-bindings')).toBeVisible();

  const renderedConfig = page.getByTestId('mcp-rendered-config');
  await expect(renderedConfig).toBeVisible();
  await expect(renderedConfig).not.toContainText('暂无 rendered config preview');

  await page.getByTestId('mcp-test-run').click();
  await expect(page.getByTestId('mcp-test-checks')).toBeVisible();
});
