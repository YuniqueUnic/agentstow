import { expect, test } from '@playwright/test';

import { openWorkspace } from './helpers';

test('env view emits a shell script for the selected export set', async ({ page }) => {
  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  await nav.getByRole('button', { name: 'Env', exact: true }).click();

  await expect(page.getByText('Export Sets')).toBeVisible();
  await page.getByRole('button', { name: '生成脚本', exact: true }).click();

  const editor = page.getByLabel('Shell 预览').locator('.cm-content');
  await expect(editor).toContainText('OPENAI_API_KEY');
  await expect(page.getByRole('button', { name: '复制脚本', exact: true })).toBeEnabled();
});

test('links view can apply the declared target and report a healthy status', async ({ page }) => {
  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  const resultsPanel = page.getByRole('region', { name: 'Links 结果面板' });
  await nav.getByRole('button', { name: 'Links', exact: true }).click();

  await expect(page.getByRole('button', { name: 'hello_copy hello@base' })).toBeVisible();
  await page.getByRole('button', { name: 'Apply', exact: true }).click();

  const appliedRow = resultsPanel.locator('.result-row').filter({ hasText: 'hello_copy' });
  await expect(appliedRow).toBeVisible();
  await expect(appliedRow).toContainText('applied');
  await resultsPanel.getByRole('tab', { name: 'Status', exact: true }).click();
  await expect(resultsPanel).toContainText('healthy');
});
