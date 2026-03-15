import { render } from 'vitest-browser-svelte';
import { afterEach, expect, test, vi } from 'vitest';

import WorkspaceBoot from './WorkspaceBoot.svelte';

const originalShowDirectoryPicker = Object.getOwnPropertyDescriptor(window, 'showDirectoryPicker');
const originalSecureContext = Object.getOwnPropertyDescriptor(window, 'isSecureContext');

afterEach(() => {
  if (originalShowDirectoryPicker) {
    Object.defineProperty(window, 'showDirectoryPicker', originalShowDirectoryPicker);
  } else {
    delete (window as Window & { showDirectoryPicker?: unknown }).showDirectoryPicker;
  }

  if (originalSecureContext) {
    Object.defineProperty(window, 'isSecureContext', originalSecureContext);
  }
});

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

test('uses folder picker path when runtime exposes a native path bridge', async () => {
  const onWorkspaceInput = vi.fn();

  Object.defineProperty(window, 'isSecureContext', {
    configurable: true,
    value: true
  });
  Object.defineProperty(window, 'showDirectoryPicker', {
    configurable: true,
    value: vi.fn().mockResolvedValue({
      name: 'agentstow-workspace',
      path: '/tmp/agentstow-workspace'
    })
  });

  const screen = await render(WorkspaceBoot, {
    workspaceInput: '',
    initGit: false,
    busy: false,
    errorMessage: null,
    statusLine: '等待连接到 AgentStow server…',
    onWorkspaceInput,
    onInitGit: vi.fn(),
    onOpenWorkspace: vi.fn(),
    onInitWorkspace: vi.fn()
  });

  await screen.getByRole('button', { name: '选择文件夹（实验）' }).click();

  expect(onWorkspaceInput).toHaveBeenCalledWith('/tmp/agentstow-workspace');
  await expect.element(screen.getByTestId('workspace-folder-picker-message')).toHaveTextContent(
    '并回填到路径输入框'
  );
});

test('keeps manual path fallback when picker returns only a directory handle', async () => {
  Object.defineProperty(window, 'isSecureContext', {
    configurable: true,
    value: true
  });
  Object.defineProperty(window, 'showDirectoryPicker', {
    configurable: true,
    value: vi.fn().mockResolvedValue({
      name: 'picked-without-path'
    })
  });

  const onWorkspaceInput = vi.fn();
  const screen = await render(WorkspaceBoot, {
    workspaceInput: '',
    initGit: false,
    busy: false,
    errorMessage: null,
    statusLine: '等待连接到 AgentStow server…',
    onWorkspaceInput,
    onInitGit: vi.fn(),
    onOpenWorkspace: vi.fn(),
    onInitWorkspace: vi.fn()
  });

  await screen.getByRole('button', { name: '选择文件夹（实验）' }).click();

  expect(onWorkspaceInput).not.toHaveBeenCalled();
  await expect.element(screen.getByTestId('workspace-folder-picker-message')).toHaveTextContent(
    '不暴露原生绝对路径'
  );
  await expect.element(screen.getByTestId('workspace-folder-picker-name')).toHaveTextContent(
    'picked-without-path'
  );
});
