<script lang="ts">
  import { onMount } from 'svelte';

  import type { WorkspacePickerCapability } from '$lib/types';

  type Props = {
    workspaceInput: string;
    initGit: boolean;
    busy: boolean;
    errorMessage: string | null;
    statusLine: string;
    onWorkspaceInput: (next: string) => void;
    onInitGit: (next: boolean) => void;
    onOpenWorkspace: () => void | Promise<void>;
    onInitWorkspace: () => void | Promise<void>;
  };

  type DirectoryHandleLike = {
    name?: string;
    path?: string;
    fullPath?: string;
    nativePath?: string;
    __nativePath?: string;
  };

  type WindowWithDirectoryPicker = Window & {
    showDirectoryPicker?: (options?: {
      id?: string;
      mode?: 'read' | 'readwrite';
      startIn?: 'desktop' | 'documents' | 'downloads' | 'music' | 'pictures' | 'videos';
    }) => Promise<DirectoryHandleLike>;
  };

  const DEFAULT_PICKER_CAPABILITY: WorkspacePickerCapability = {
    supported: false,
    secureContext: false,
    supportsPathExtraction: false,
    reason: '当前运行环境未暴露 folder picker。'
  };

  let {
    workspaceInput,
    initGit,
    busy,
    errorMessage,
    statusLine,
    onWorkspaceInput,
    onInitGit,
    onOpenWorkspace,
    onInitWorkspace
  }: Props = $props();

  let pickerCapability = $state<WorkspacePickerCapability>(DEFAULT_PICKER_CAPABILITY);
  let pickerBusy = $state(false);
  let pickerMessage = $state<string | null>(null);
  let pickedDirectoryName = $state<string | null>(null);

  function describePickerCapability(): WorkspacePickerCapability {
    if (typeof window === 'undefined') {
      return DEFAULT_PICKER_CAPABILITY;
    }

    const secureContext = window.isSecureContext;
    const pickerWindow = window as WindowWithDirectoryPicker;
    const supported = typeof pickerWindow.showDirectoryPicker === 'function';

    if (!supported) {
      return {
        supported: false,
        secureContext,
        supportsPathExtraction: false,
        reason: '当前浏览器不支持 File System Access directory picker。'
      };
    }

    if (!secureContext) {
      return {
        supported: true,
        secureContext,
        supportsPathExtraction: false,
        reason: 'folder picker 需要 secure context，当前环境通常只能做能力探测。'
      };
    }

    return {
      supported: true,
      secureContext,
      supportsPathExtraction: false,
      reason: '可选择目录句柄；若运行时未桥接原生路径，仍需手动粘贴绝对路径。'
    };
  }

  function extractDirectoryPath(handle: DirectoryHandleLike | null | undefined): string | null {
    if (!handle) {
      return null;
    }

    for (const key of ['path', 'fullPath', 'nativePath', '__nativePath'] as const) {
      const value = handle[key];
      if (typeof value === 'string' && value.trim()) {
        return value.trim();
      }
    }

    return null;
  }

  function describePickerError(error: unknown): string {
    if (error instanceof DOMException && error.name === 'AbortError') {
      return '已取消目录选择。';
    }
    if (error instanceof Error && error.message) {
      return error.message;
    }
    return '目录选择失败。';
  }

  async function handlePickWorkspace(): Promise<void> {
    const pickerWindow = window as WindowWithDirectoryPicker;
    if (typeof pickerWindow.showDirectoryPicker !== 'function') {
      pickerMessage = '当前运行环境未提供 folder picker。';
      return;
    }

    pickerBusy = true;
    pickerMessage = null;

    try {
      const handle = await pickerWindow.showDirectoryPicker({
        id: 'agentstow-workspace',
        mode: 'readwrite',
        startIn: 'documents'
      });
      pickedDirectoryName = handle.name ?? '未命名目录';

      const nativePath = extractDirectoryPath(handle);
      pickerCapability = {
        ...pickerCapability,
        supportsPathExtraction: Boolean(nativePath)
      };

      if (nativePath) {
        onWorkspaceInput(nativePath);
        pickerMessage = `已选择目录“${pickedDirectoryName}”，并回填到路径输入框。`;
        return;
      }

      pickerMessage = `已选择目录“${pickedDirectoryName}”。当前浏览器只返回 FileSystemDirectoryHandle，不暴露原生绝对路径；请手动粘贴路径，或等待原生 bridge/协议接入。`;
    } catch (error) {
      pickerMessage = describePickerError(error);
    } finally {
      pickerBusy = false;
    }
  }

  onMount(() => {
    pickerCapability = describePickerCapability();
  });
</script>

<div class="boot">
  <div class="boot__panel" role="region" aria-label="Workspace 引导">
    <div class="boot__rail" aria-hidden="true">
      <span></span>
      <span></span>
      <span></span>
    </div>
    <p class="boot__eyebrow">AgentStow Workbench</p>
    <h1>把 workspace 接进编辑器</h1>
    <p class="boot__lead">
      你可以打开一个已经包含 <code>agentstow.toml</code> 的目录，也可以在任意目录里
      初始化一个新的 workspace。工作台会直接把 artifacts、targets、env sets、scripts 和 MCP
      当成一等对象展示。
    </p>

    <div class="boot__capability" data-testid="workspace-folder-picker-capability">
      <span class={['pill', pickerCapability.supported ? 'pill--ok' : 'pill--warn'].join(' ')}>
        {pickerCapability.supported ? 'folder picker ready' : 'manual path only'}
      </span>
      <div class="boot__capability-copy">
        <strong>{pickerCapability.supported ? 'Folder Picker' : 'Manual Path'}</strong>
        <span>{pickerCapability.reason ?? '可直接输入本机路径。'}</span>
      </div>
    </div>

    <div class="boot__form">
      <label class="field">
        <span class="field__label">Workspace 路径</span>
        <input
          class="field__input mono"
          type="text"
          placeholder="/path/to/workspace"
          value={workspaceInput}
          oninput={(event) => {
            const target = event.currentTarget as HTMLInputElement | null;
            onWorkspaceInput(target?.value ?? '');
          }}
        />
        <span class="field__hint">
          {#if pickerCapability.supported}
            可先点“选择文件夹（实验）”探测目录句柄；如果运行时不暴露绝对路径，仍需手动粘贴本机路径。
          {:else}
            服务端不会自动弹出文件选择框，直接输入本机路径即可。
          {/if}
        </span>
      </label>

      <label class="toggle boot__toggle" aria-label="初始化选项">
        <input
          class="toggle__control"
          type="checkbox"
          checked={initGit}
          onchange={(event) => {
            const target = event.currentTarget as HTMLInputElement | null;
            onInitGit(Boolean(target?.checked));
          }}
        />
        <span>初始化时执行 <code>git init</code></span>
      </label>

      <div class="boot__actions">
        <button
          class="ui-button"
          disabled={busy || pickerBusy || !pickerCapability.supported}
          type="button"
          onclick={() => void handlePickWorkspace()}
        >
          {pickerBusy ? '选择中…' : '选择文件夹（实验）'}
        </button>
        <button class="ui-button ui-button--ghost" disabled={busy} type="button" onclick={() => void onOpenWorkspace()}>
          打开 workspace
        </button>
        <button
          class="ui-button ui-button--primary"
          disabled={busy}
          type="button"
          onclick={() => void onInitWorkspace()}
        >
          {busy ? '处理中…' : '初始化 workspace'}
        </button>
      </div>
    </div>

    {#if pickerMessage}
      <p class="notice" data-testid="workspace-folder-picker-message">{pickerMessage}</p>
    {/if}
    {#if pickedDirectoryName}
      <p class="boot__status mono" data-testid="workspace-folder-picker-name">last picked: {pickedDirectoryName}</p>
    {/if}
    {#if errorMessage}
      <p class="notice notice--error">{errorMessage}</p>
    {/if}
    <p class="boot__status" aria-live="polite">{statusLine}</p>
  </div>
</div>

<style>
  .boot__capability {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px 14px;
    margin: 0 0 18px;
    border: 1px solid color-mix(in oklch, var(--line) 82%, transparent);
    background: color-mix(in oklch, var(--canvas-elevated) 72%, transparent);
  }

  .boot__capability-copy {
    display: grid;
    gap: 4px;
  }

  .boot__capability-copy strong {
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .boot__capability-copy span {
    color: var(--ink-soft);
    line-height: 1.45;
  }
</style>
