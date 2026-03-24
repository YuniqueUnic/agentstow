import os from 'node:os';
import path from 'node:path';

import { expect, type Page } from '@playwright/test';

export const PLAYWRIGHT_WORKSPACE_ROOT = path.join(os.tmpdir(), 'agentstow-playwright-workspace');
export const PLAYWRIGHT_BOOTSTRAP_ROOT = path.join(
  os.tmpdir(),
  'agentstow-playwright-generated-workspace'
);

export async function goToWorkspaceBoot(page: Page): Promise<void> {
  await page.goto('/');

  const bootRegion = page.getByRole('region', { name: 'Workspace 引导' });
  if (await bootRegion.isVisible().catch(() => false)) {
    await expect(bootRegion).toBeVisible();
    return;
  }

  await page.getByRole('button', { name: 'Workspace', exact: true }).click();
  await expect(bootRegion).toBeVisible();
}

export async function openWorkspace(page: Page, workspaceRoot = PLAYWRIGHT_WORKSPACE_ROOT): Promise<void> {
  await goToWorkspaceBoot(page);
  await page.getByRole('textbox', { name: 'Workspace 路径' }).fill(workspaceRoot);
  await page.getByRole('button', { name: '检查路径', exact: true }).click();
  await expect(page.getByTestId('workspace-probe-summary')).toContainText(workspaceRoot);
  await page.getByRole('button', { name: '打开 workspace', exact: true }).click();
  await expect(page.getByTestId('artifact-tree-item:hello')).toBeVisible();
  await expect(page.getByTestId('artifact-source-editor').locator('.cm-content')).toBeVisible();
}
