import { expect, test } from '@playwright/test';

import { createWorkbenchWorkspace, openWorkspace } from './helpers';

test('env view auto-emits a shell script for the selected export set', async ({ page }) => {
  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  await nav.getByRole('button', { name: 'Env', exact: true }).click();

  await expect(page.getByText('Export Sets')).toBeVisible();
  const editor = page.getByLabel('Shell 预览').locator('.cm-content');
  await expect(editor).toContainText("export OPENAI_API_KEY='agentstow-playwright-token'");
  await expect(page.getByRole('button', { name: '重新生成', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '重新生成', exact: true }).click();
  await expect(editor).toContainText("export OPENAI_API_KEY='agentstow-playwright-token'");
  await expect(page.getByRole('button', { name: '复制脚本', exact: true })).toBeEnabled();
});

test('links view can apply the declared target and report a healthy status', async ({ page }) => {
  await openWorkspace(page, await createWorkbenchWorkspace('agentstow-links-flow-workspace-'));

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

test('scripts view runs stdin-aware script and shows captured stdout', async ({ page }) => {
  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  await nav.getByRole('button', { name: 'Scripts', exact: true }).click();

  const scriptButton = page.getByRole('button', { name: 'sync shell', exact: true });
  await expect(scriptButton).toBeVisible();
  await scriptButton.click();

  const stdinEditor = page.getByRole('region', { name: 'stdin editor' }).locator('.cm-content');
  await stdinEditor.click();
  await page.keyboard.insertText('payload-123');

  await page.getByRole('button', { name: '运行', exact: true }).click();

  await expect(page.getByRole('region', { name: '脚本检查器' })).toContainText('OPENAI_API_KEY');
  await page.getByRole('tab', { name: 'stdout', exact: true }).click();
  await expect(page.getByRole('tab', { name: 'stdout', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'stderr', exact: true })).toBeVisible();
  await expect(page.getByText('sync:agentstow-playwright-token:payload-123')).toBeVisible();
});
