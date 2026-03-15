import { render } from 'vitest-browser-svelte';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import {
  mcpRenderFixture,
  mcpServerFixture,
  mcpTestFixture,
  mcpValidateFixture
} from '../../../../tests/fixtures/workbench';

const renderMcpServerMock = vi.fn();
const validateMcpServerMock = vi.fn();
const testMcpServerMock = vi.fn();

vi.mock('$lib/api/client', async () => {
  const actual = await vi.importActual<typeof import('$lib/api/client')>('$lib/api/client');
  return {
    ...actual,
    renderMcpServer: (...args: Parameters<typeof renderMcpServerMock>) => renderMcpServerMock(...args),
    validateMcpServer: (...args: Parameters<typeof validateMcpServerMock>) =>
      validateMcpServerMock(...args),
    testMcpServer: (...args: Parameters<typeof testMcpServerMock>) => testMcpServerMock(...args)
  };
});

import McpView from './McpView.svelte';

describe('McpView', () => {
  beforeEach(() => {
    renderMcpServerMock.mockReset();
    validateMcpServerMock.mockReset();
    testMcpServerMock.mockReset();

    renderMcpServerMock.mockResolvedValue(mcpRenderFixture);
    validateMcpServerMock.mockResolvedValue(mcpValidateFixture);
    testMcpServerMock.mockResolvedValue(mcpTestFixture);
  });

  test('renders structured validate and dry-run inspector results', async () => {
    const screen = await render(McpView, {
      mcpServers: [mcpServerFixture],
      selectedMcpServerId: mcpServerFixture.id,
      activeMcpServer: mcpServerFixture,
      errorMessage: null,
      statusLine: 'ready',
      onSelectMcpServer: vi.fn(),
      onCopyToClipboard: vi.fn(async () => {}),
      onOpenManifestEditor: vi.fn(),
      onCreateManifestObject: vi.fn()
    });

    await expect.element(screen.getByTestId('mcp-launcher-preview')).toHaveTextContent(
      '@modelcontextprotocol/server-filesystem'
    );
    await expect.element(screen.getByTestId('mcp-rendered-config')).toHaveTextContent(
      '"OPENAI_API_KEY": "${OPENAI_API_KEY}"'
    );

    await screen.getByTestId('mcp-validate-run').click();
    await expect.element(screen.getByTestId('mcp-validate-issues')).toHaveTextContent(
      'OPENAI_API_KEY 尚未绑定到当前运行环境。'
    );

    await screen.getByTestId('mcp-test-run').click();
    await expect.element(screen.getByTestId('mcp-test-checks')).toHaveTextContent('env_missing');
    await expect.element(screen.getByTestId('mcp-env-bindings')).toHaveTextContent(
      'source env: OPENAI_API_KEY'
    );
  });
});
