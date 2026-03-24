import { expect, test } from '@playwright/test';
import { openWorkspace } from './helpers';

test('renders MCP runtime panel against the local workspace server', async ({ page }) => {
  await openWorkspace(page);

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

test('keeps the bottom panel collapsed until the operator opens it', async ({ page }) => {
  await openWorkspace(page);

  await expect(page.getByTestId('workbench-bottom-panel')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /problems 0/i })).toBeVisible();
  await expect(page.getByRole('contentinfo')).toContainText('已连接到 workspace。');
});
