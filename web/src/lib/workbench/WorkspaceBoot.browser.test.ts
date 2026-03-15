import { render } from 'vitest-browser-svelte';
import { expect, test, vi } from 'vitest';

import WorkspaceBoot from './WorkspaceBoot.svelte';

test('accepts workspace input and fires boot actions', async () => {
  const onWorkspaceInput = vi.fn();
  const onInitGit = vi.fn();
  const onOpenWorkspace = vi.fn();
  const onInitWorkspace = vi.fn();

  const screen = await render(WorkspaceBoot, {
    workspaceInput: '',
    initGit: false,
    busy: false,
    errorMessage: null,
    statusLine: '等待连接到 AgentStow server…',
    onWorkspaceInput,
    onInitGit,
    onOpenWorkspace,
    onInitWorkspace
  });

  await screen.getByRole('textbox', { name: 'Workspace 路径' }).fill('/tmp/agentstow-workspace');
  expect(onWorkspaceInput).toHaveBeenLastCalledWith('/tmp/agentstow-workspace');

  await screen.getByRole('checkbox').click();
  expect(onInitGit).toHaveBeenLastCalledWith(true);

  await screen.getByRole('button', { name: '打开 workspace' }).click();
  await screen.getByRole('button', { name: '初始化 workspace' }).click();

  expect(onOpenWorkspace).toHaveBeenCalledTimes(1);
  expect(onInitWorkspace).toHaveBeenCalledTimes(1);
});
