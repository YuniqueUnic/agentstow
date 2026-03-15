import { expect, test } from '@playwright/test';

async function openWorkspace(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/');
  await expect(page.getByTestId('artifact-tree-item:hello')).toBeVisible();
  await expect(page.getByTestId('artifact-source-editor').locator('.cm-content')).toBeVisible();
}

test('source editor content syncs when switching artifact files', async ({ page }) => {
  await openWorkspace(page);

  const sourceEditor = page.getByTestId('artifact-source-editor');
  const sourceContent = sourceEditor.locator('.cm-content');

  await page.getByTestId('artifact-tree-item:hello').click();
  await expect(page.getByTestId('artifact-source-path')).toContainText('artifacts/hello.txt.tera');
  await expect(sourceContent).toContainText('Hello {{ name }} from Git!');

  await page.getByTestId('artifact-tree-item:bye').click();
  await expect(page.getByTestId('artifact-source-path')).toContainText('artifacts/bye.txt.tera');
  await expect(sourceContent).toContainText('Bye from AgentStow.');
});

test('artifact preview pane state persists across view switches and save refreshes', async ({ page }) => {
  await openWorkspace(page);

  const mod = process.platform === 'darwin' ? 'Meta' : 'Control';
  const nav = page.getByRole('navigation', { name: '主导航' });
  const sourceEditor = page.getByTestId('artifact-source-editor');
  const sourceContent = sourceEditor.locator('.cm-content');

  await page.getByTestId('artifact-tree-item:hello').click();
  await page.getByRole('button', { name: '隐藏 Preview', exact: true }).click();
  await expect(page.getByRole('button', { name: '显示 Preview', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Rendered' })).toHaveCount(0);

  await nav.getByRole('button', { name: 'MCP', exact: true }).click();
  await nav.getByRole('button', { name: 'Artifacts', exact: true }).click();

  await expect(page.getByRole('button', { name: '显示 Preview', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Rendered' })).toHaveCount(0);

  await sourceContent.click();
  await page.keyboard.press(`${mod}+End`);
  await page.keyboard.insertText('\nPreview pane should stay closed after save.\n');
  await page.keyboard.press(`${mod}+S`);

  await expect(page.getByRole('button', { name: '已保存', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: '显示 Preview', exact: true })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Rendered' })).toHaveCount(0);
});

test('artifact source editor retains an internal scroll container for long content', async ({ page }) => {
  await openWorkspace(page);

  const sourceEditor = page.getByTestId('artifact-source-editor');
  const sourceContent = sourceEditor.locator('.cm-content');
  const longDocument = Array.from({ length: 180 }, (_, index) => `Line ${index + 1} {{ name }}`).join('\n');

  await page.getByTestId('artifact-tree-item:hello').click();
  await sourceContent.click();
  await page.keyboard.press(`${process.platform === 'darwin' ? 'Meta' : 'Control'}+A`);
  await page.keyboard.insertText(longDocument);

  const scrollMetrics = await sourceEditor.locator('.cm-scroller').evaluate((node) => {
    const el = node as HTMLElement;
    el.scrollTop = el.scrollHeight;
    return {
      clientHeight: el.clientHeight,
      scrollHeight: el.scrollHeight,
      scrollTop: el.scrollTop
    };
  });

  expect(scrollMetrics.scrollHeight).toBeGreaterThan(scrollMetrics.clientHeight);
  expect(scrollMetrics.scrollTop).toBeGreaterThan(0);
});

test('inserting a new artifact snippet updates the editor and opens the bootstrapped source after save', async ({
  page
}) => {
  await openWorkspace(page);

  await page.getByRole('button', { name: /agentstow\.toml/ }).click();
  await page.locator('select').first().selectOption('artifact');
  await page.getByRole('button', { name: '插入模板', exact: true }).click();

  const manifestEditor = page.getByTestId('artifact-source-editor').locator('.cm-content');
  await expect(manifestEditor).toContainText('[artifacts.new_artifact]');
  await expect(manifestEditor).toContainText('source = "artifacts/new_artifact.md.tera"');

  await page.getByRole('button', { name: '保存', exact: true }).click();

  const sourceEditor = page.getByTestId('artifact-source-editor').locator('.cm-content');
  await expect(page.getByTestId('artifact-source-path')).toContainText('artifacts/new_artifact.md.tera');
  await expect(sourceEditor).toContainText('# new_artifact');
});

test('clipboard fallback still copies MCP launcher when Clipboard API write fails', async ({ page }) => {
  await page.addInitScript(() => {
    const win = window as Window & {
      __agentstowCopiedText__?: string;
    };

    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: async () => {
          throw new Error('Clipboard API denied');
        }
      }
    });

    const execCommand = (commandId: string) => {
      if (commandId !== 'copy') {
        return false;
      }

      const activeElement = document.activeElement;
      if (activeElement instanceof HTMLTextAreaElement || activeElement instanceof HTMLInputElement) {
        win.__agentstowCopiedText__ = activeElement.value;
        return true;
      }

      return false;
    };

    Object.defineProperty(Document.prototype, 'execCommand', {
      configurable: true,
      value: execCommand
    });
  });

  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  await nav.getByRole('button', { name: 'MCP', exact: true }).click();

  await page.getByRole('button', { name: '复制 launcher' }).click();
  await expect.poll(() => page.evaluate(() => (window as Window & { __agentstowCopiedText__?: string }).__agentstowCopiedText__)).toBe(
    'npx -y @modelcontextprotocol/server-filesystem .'
  );
});

test('artifact history compare uses structured diff rendering', async ({ page }) => {
  await openWorkspace(page);

  await page.getByTestId('artifact-tree-item:hello').click();

  const history = page.getByTestId('artifact-git-history');
  await expect(history).toBeVisible();

  const compareButtons = history.getByRole('button', { name: /对比|对比中/ });
  await expect(compareButtons).toHaveCount(2);
  await compareButtons.last().click();

  const diff = page.getByTestId('artifact-diff-viewer');
  await expect(diff).toBeVisible();
  await expect(diff).toContainText('@@ -1,1 +1,');
  await expect(diff).toContainText('Hello {{ name }}!');
  await expect(diff).toContainText('Hello {{ name }} from Git!');
});

test('MCP view exposes validate render and dry-run test loop', async ({ page }) => {
  await openWorkspace(page);

  const nav = page.getByRole('navigation', { name: '主导航' });
  await nav.getByRole('button', { name: 'MCP', exact: true }).click();

  await expect(page.getByTestId('mcp-rendered-config')).toContainText('"mcpServers"');
  await expect(page.getByTestId('mcp-env-bindings')).toContainText('OPENAI_API_KEY');

  await page.getByTestId('mcp-validate-run').click();
  await expect(page.getByText('校验通过，未发现问题')).toBeVisible();

  await page.getByTestId('mcp-test-run').click();
  const checks = page.getByTestId('mcp-test-checks');
  await expect(checks).toBeVisible();
  await expect(checks).toContainText('validate');
  await expect(checks).toContainText('render');
});

test('watch trace panel shows recent source save events', async ({ page }) => {
  await openWorkspace(page);

  const mod = process.platform === 'darwin' ? 'Meta' : 'Control';
  const sourceEditor = page.getByTestId('artifact-source-editor');
  const sourceContent = sourceEditor.locator('.cm-content');

  await page.getByTestId('artifact-tree-item:hello').click();
  await sourceContent.click();
  await page.keyboard.press(`${mod}+A`);
  await page.keyboard.type('Hello {{ name }} from Watch Trace!\n');

  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.getByRole('button', { name: '已保存', exact: true })).toBeVisible();

  await page.getByTestId('watch-trace-toggle').click();

  const tracePanel = page.getByTestId('watch-trace-panel');
  await expect(tracePanel).toBeVisible();
  await expect(tracePanel).toContainText('Recent Events');
  await expect(tracePanel).toContainText('artifacts/hello.txt.tera', { timeout: 10_000 });
});
