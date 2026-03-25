import { expect, test } from '@playwright/test';

import { goToWorkspaceBoot, openWorkspace } from './helpers';

test('captures desktop UI audit screenshots for boot and workbench views', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1024 });

  await goToWorkspaceBoot(page);
  const bootPath = testInfo.outputPath('boot-desktop.png');
  await page.screenshot({ path: bootPath, fullPage: true });
  await testInfo.attach('boot-desktop', { path: bootPath, contentType: 'image/png' });

  await openWorkspace(page);
  const workbenchPath = testInfo.outputPath('workbench-desktop.png');
  await page.screenshot({ path: workbenchPath, fullPage: true });
  await testInfo.attach('workbench-desktop', { path: workbenchPath, contentType: 'image/png' });

  await page.getByRole('button', { name: 'Light', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Light', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true'
  );
  const lightPath = testInfo.outputPath('workbench-light-desktop.png');
  await page.screenshot({ path: lightPath, fullPage: true });
  await testInfo.attach('workbench-light-desktop', { path: lightPath, contentType: 'image/png' });

  const nav = page.getByRole('navigation', { name: '主导航' });

  await nav.getByRole('button', { name: 'Env', exact: true }).click();
  await expect(page.getByRole('button', { name: '重新生成', exact: true })).toBeVisible();
  await expect(page.getByLabel('Shell 预览').locator('.cm-content')).toContainText(
    'agentstow-playwright-token'
  );
  const envPath = testInfo.outputPath('env-desktop.png');
  await page.screenshot({ path: envPath, fullPage: true });
  await testInfo.attach('env-desktop', { path: envPath, contentType: 'image/png' });

  await nav.getByRole('button', { name: 'Links', exact: true }).click();
  await expect(page.getByRole('button', { name: 'hello_copy hello@base' })).toBeVisible();
  await page.getByRole('button', { name: 'Plan', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Links 结果面板' })).toContainText('hello_copy');
  const linksPath = testInfo.outputPath('links-desktop.png');
  await page.screenshot({ path: linksPath, fullPage: true });
  await testInfo.attach('links-desktop', { path: linksPath, contentType: 'image/png' });

  await page.setViewportSize({ width: 1440, height: 780 });
  const linksShortPath = testInfo.outputPath('links-short.png');
  await page.screenshot({ path: linksShortPath, fullPage: true });
  await testInfo.attach('links-short', { path: linksShortPath, contentType: 'image/png' });
});

test('captures mobile UI audit screenshots and guards against horizontal overflow', async ({
  page
}, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });

  await openWorkspace(page);
  const mobilePath = testInfo.outputPath('workbench-mobile.png');
  await page.screenshot({ path: mobilePath, fullPage: true });
  await testInfo.attach('workbench-mobile', { path: mobilePath, contentType: 'image/png' });

  const overflow = await page.evaluate(() => {
    const root = document.documentElement;
    return root.scrollWidth - root.clientWidth;
  });

  expect(overflow).toBeLessThanOrEqual(1);
});
