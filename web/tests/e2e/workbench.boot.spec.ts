import fs from 'node:fs';

import { expect, test } from '@playwright/test';

import {
  PLAYWRIGHT_BOOTSTRAP_ROOT,
  PLAYWRIGHT_WORKSPACE_ROOT,
  goToWorkspaceBoot,
  openWorkspace
} from './helpers';

test('workspace boot flow opens an existing workspace through the actual UI', async ({ page }) => {
  await goToWorkspaceBoot(page);

  await page.getByRole('textbox', { name: 'Workspace 路径' }).fill(PLAYWRIGHT_WORKSPACE_ROOT);
  await page.getByRole('button', { name: '检查路径', exact: true }).click();

  const summary = page.getByTestId('workspace-probe-summary');
  await expect(summary).toContainText('workspace 已可直接打开');
  await expect(summary).toContainText('agentstow.toml');

  await page.getByRole('button', { name: '打开 workspace', exact: true }).click();
  await expect(page.getByTestId('artifact-tree-item:hello')).toBeVisible();
});

test('workspace boot flow can initialize a missing workspace and return to the main fixture', async ({
  page
}) => {
  fs.rmSync(PLAYWRIGHT_BOOTSTRAP_ROOT, { recursive: true, force: true });

  await goToWorkspaceBoot(page);
  await page.getByRole('textbox', { name: 'Workspace 路径' }).fill(PLAYWRIGHT_BOOTSTRAP_ROOT);
  await page.getByRole('button', { name: '检查路径', exact: true }).click();

  const summary = page.getByTestId('workspace-probe-summary');
  await expect(summary).toContainText('路径不存在，可直接创建并初始化');

  await page.getByRole('button', { name: '创建并初始化', exact: true }).click();
  await expect(page.getByTestId('artifact-tree-item:hello')).toBeVisible();

  await openWorkspace(page, PLAYWRIGHT_WORKSPACE_ROOT);
});
