import { render } from 'vitest-browser-svelte';
import { expect, test, vi } from 'vitest';

import { envSetFixture } from '../../../../tests/fixtures/workbench';

import EnvView from './EnvView.svelte';

test('renders env object preview and opens usage referrers from the inspector', async () => {
  const onOpenUsageRef = vi.fn();

  const screen = await render(EnvView, {
    envSets: [envSetFixture],
    selectedEnvSet: envSetFixture.id,
    activeEnvSet: envSetFixture,
    selectedShell: 'bash',
    shellChoices: ['bash', 'zsh'],
    envScript: { text: "export OPENAI_API_KEY='token'" },
    busyEnvEmit: false,
    errorMessage: null,
    statusLine: 'ready',
    onSelectEnvSet: vi.fn(),
    onSelectShell: vi.fn(),
    onEnvEmit: vi.fn(async () => {}),
    onCopyToClipboard: vi.fn(async () => {}),
    onOpenUsageRef,
    onOpenManifestEditor: vi.fn(),
    onCreateManifestObject: vi.fn()
  });

  await expect.element(screen.getByTestId('env-object-preview')).toHaveTextContent(
    '"OPENAI_API_KEY": "${OPENAI_API_KEY}"'
  );
  await expect.element(screen.getByTestId('env-object-preview')).toHaveAttribute('data-language', 'json');
  await expect.element(screen.getByTestId('env-binding-guide')).toHaveTextContent('kind = "literal"');
  await expect.element(screen.getByTestId('env-referrer-list')).toHaveTextContent('MCP local');

  await screen
    .getByTestId('env-referrer-list')
    .getByRole('button', { name: /MCP local/ })
    .first()
    .click();

  expect(onOpenUsageRef).toHaveBeenCalledWith(
    expect.objectContaining({
      owner_kind: 'mcp_server',
      owner_id: 'local'
    })
  );
});
