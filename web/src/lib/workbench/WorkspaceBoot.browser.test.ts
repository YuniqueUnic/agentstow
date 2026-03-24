import { render } from 'vitest-browser-svelte';
import { expect, test, vi } from 'vitest';

import WorkspaceBoot from './WorkspaceBoot.svelte';

test('accepts workspace input and fires boot actions', async () => {
  const onWorkspaceInput = vi.fn();
  const onInitGit = vi.fn();
  const onProbeWorkspace = vi.fn();
  const onPickWorkspace = vi.fn();
  const onOpenWorkspace = vi.fn();
  const onInitWorkspace = vi.fn();

  const screen = await render(WorkspaceBoot, {
    workspaceInput: '/tmp/agentstow-workspace',
    workspaceProbe: null,
    initGit: false,
    busy: false,
    pickerBusy: false,
    errorMessage: null,
    statusLine: '等待连接到 AgentStow server…',
    onWorkspaceInput,
    onInitGit,
    onProbeWorkspace,
    onPickWorkspace,
    onOpenWorkspace,
    onInitWorkspace
  });

  await screen.getByRole('textbox', { name: 'Workspace 路径' }).fill('/tmp/agentstow-workspace-next');
  expect(onWorkspaceInput).toHaveBeenLastCalledWith('/tmp/agentstow-workspace-next');

  await screen.getByRole('checkbox').click();
  expect(onInitGit).toHaveBeenLastCalledWith(true);

  await screen.getByRole('button', { name: '选择文件夹' }).click();
  await screen.getByRole('button', { name: '检查路径' }).click();
  await screen.getByTestId('workspace-primary-action').click();

  expect(onPickWorkspace).toHaveBeenCalledTimes(1);
  expect(onProbeWorkspace).toHaveBeenCalledTimes(2);
  expect(onOpenWorkspace).toHaveBeenCalledTimes(0);
  expect(onInitWorkspace).toHaveBeenCalledTimes(0);
});

test('renders probe details for missing workspace paths', async () => {
  const screen = await render(WorkspaceBoot, {
    workspaceInput: '/tmp/future-workspace',
    workspaceProbe: {
      requested_workspace_root: '/tmp/future-workspace',
      resolved_workspace_root: '/tmp/future-workspace',
      exists: false,
      is_directory: false,
      manifest_present: false,
      manifest_path: '/tmp/future-workspace/agentstow.toml',
      git_present: false,
      selectable: false,
      initializable: true,
      reason: '路径不存在，可初始化新的 workspace。'
    },
    initGit: true,
    busy: false,
    pickerBusy: false,
    errorMessage: null,
    statusLine: '目标路径不存在，可直接创建并初始化 workspace。',
    onWorkspaceInput: vi.fn(),
    onInitGit: vi.fn(),
    onProbeWorkspace: vi.fn(),
    onPickWorkspace: vi.fn(),
    onOpenWorkspace: vi.fn(),
    onInitWorkspace: vi.fn()
  });

  await expect.element(screen.getByTestId('workspace-probe-summary')).toHaveTextContent(
    '路径不存在，可直接创建并初始化'
  );
  await expect.element(screen.getByTestId('workspace-probe-summary')).toHaveTextContent(
    'agentstow.toml'
  );
  await expect.element(screen.getByTestId('workspace-primary-action')).toHaveTextContent(
    '创建并初始化'
  );
});

test('renders a direct open action for selectable workspaces', async () => {
  const onOpenWorkspace = vi.fn();

  const screen = await render(WorkspaceBoot, {
    workspaceInput: '/tmp/existing-workspace',
    workspaceProbe: {
      requested_workspace_root: '/tmp/existing-workspace',
      resolved_workspace_root: '/tmp/existing-workspace',
      exists: true,
      is_directory: true,
      manifest_present: true,
      manifest_path: '/tmp/existing-workspace/agentstow.toml',
      git_present: true,
      selectable: true,
      initializable: false,
      reason: null
    },
    initGit: false,
    busy: false,
    pickerBusy: false,
    errorMessage: null,
    statusLine: '已确认路径可直接打开。',
    onWorkspaceInput: vi.fn(),
    onInitGit: vi.fn(),
    onProbeWorkspace: vi.fn(),
    onPickWorkspace: vi.fn(),
    onOpenWorkspace,
    onInitWorkspace: vi.fn()
  });

  await expect.element(screen.getByTestId('workspace-primary-action')).toHaveTextContent(
    '打开 workspace'
  );
  await screen.getByTestId('workspace-primary-action').click();
  expect(onOpenWorkspace).toHaveBeenCalledTimes(1);
});
