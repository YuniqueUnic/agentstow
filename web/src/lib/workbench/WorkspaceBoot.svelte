<script lang="ts">
  import type { WorkspaceProbeResponse } from '$lib/types';

  type Props = {
    workspaceInput: string;
    workspaceProbe: WorkspaceProbeResponse | null;
    initGit: boolean;
    busy: boolean;
    pickerBusy: boolean;
    errorMessage: string | null;
    statusLine: string;
    onWorkspaceInput: (next: string) => void;
    onInitGit: (next: boolean) => void;
    onProbeWorkspace: () => void | Promise<void>;
    onPickWorkspace: () => void | Promise<void>;
    onOpenWorkspace: () => void | Promise<void>;
    onInitWorkspace: () => void | Promise<void>;
  };

  let {
    workspaceInput,
    workspaceProbe,
    initGit,
    busy,
    pickerBusy,
    errorMessage,
    statusLine,
    onWorkspaceInput,
    onInitGit,
    onProbeWorkspace,
    onPickWorkspace,
    onOpenWorkspace,
    onInitWorkspace
  }: Props = $props();

  const hasInput = $derived(workspaceInput.trim().length > 0);
  const probeTone = $derived.by(() => {
    if (!workspaceProbe) {
      return 'neutral';
    }
    if (workspaceProbe.selectable) {
      return 'ok';
    }
    if (workspaceProbe.initializable) {
      return 'warn';
    }
    return 'warn';
  });
  const probeHeadline = $derived.by(() => {
    if (!workspaceProbe) {
      return '尚未检查路径';
    }
    if (workspaceProbe.selectable && workspaceProbe.manifest_present) {
      return 'workspace 已可直接打开';
    }
    if (workspaceProbe.initializable && !workspaceProbe.exists) {
      return '路径不存在，可直接创建并初始化';
    }
    if (workspaceProbe.initializable) {
      return '目录存在但尚未初始化';
    }
    return '当前路径不可用';
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
      使用本地目录选择器或直接输入绝对路径。工作台会先 probe 路径，再决定是打开现有
      workspace，还是在该目录创建并初始化新的 workspace。
    </p>

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
          支持直接粘贴本机绝对路径；若路径不存在，会提示你是否创建并初始化该 workspace。
        </span>
      </label>

      <div class="compound-action compound-action--boot">
        <button
          class="ui-button"
          disabled={busy || pickerBusy}
          type="button"
          onclick={() => void onPickWorkspace()}
        >
          {pickerBusy ? '选择中…' : '选择文件夹'}
        </button>
        <button
          class="ui-button ui-button--ghost"
          disabled={busy || pickerBusy || !hasInput}
          type="button"
          onclick={() => void onProbeWorkspace()}
        >
          检查路径
        </button>
        <button
          class="ui-button ui-button--ghost"
          disabled={busy || pickerBusy || !hasInput}
          type="button"
          onclick={() => void onOpenWorkspace()}
        >
          打开 workspace
        </button>
        <button
          class="ui-button ui-button--primary"
          disabled={busy || pickerBusy || !hasInput}
          type="button"
          onclick={() => void onInitWorkspace()}
        >
          {busy ? '处理中…' : '创建并初始化'}
        </button>
      </div>

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
    </div>

    <div class="boot__probe" data-testid="workspace-probe-summary">
      <div class="boot__probe-head">
        <span class={['pill', `pill--${probeTone}`].join(' ')}>{probeHeadline}</span>
        {#if workspaceProbe}
          <span class="mono">{workspaceProbe.resolved_workspace_root}</span>
        {/if}
      </div>

      {#if workspaceProbe}
        <div class="inspector-table">
          <div class="inspector-row">
            <span class="inspector-row__label">Exists</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.exists ? 'yes' : 'no'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Manifest</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.manifest_present ? 'ready' : 'missing'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Selectable</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.selectable ? 'yes' : 'no'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Initializable</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.initializable ? 'yes' : 'no'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Git</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.git_present ? 'present' : 'absent'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Manifest Path</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.manifest_path}
            </span>
          </div>
        </div>

        {#if workspaceProbe.reason}
          <p class="notice">{workspaceProbe.reason}</p>
        {/if}
      {:else}
        <p class="empty empty--flush">先选择文件夹或输入路径，再用“检查路径”确认状态。</p>
      {/if}
    </div>

    {#if errorMessage}
      <p class="notice notice--error">{errorMessage}</p>
    {/if}
    <p class="boot__status" aria-live="polite">{statusLine}</p>
  </div>
</div>

<style>
  .boot__probe {
    display: grid;
    gap: 12px;
  }

  .boot__probe-head {
    display: grid;
    gap: 8px;
  }

  .compound-action--boot {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  @media (max-width: 900px) {
    .compound-action--boot {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .compound-action--boot {
      grid-template-columns: 1fr;
    }
  }
</style>
