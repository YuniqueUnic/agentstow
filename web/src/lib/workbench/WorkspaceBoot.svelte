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

  const primaryAction = $derived.by<
    { label: string; hint: string; disabled: boolean; run: 'probe' | 'open' | 'init' | 'none' }
  >(() => {
    if (!hasInput) {
      return {
        label: '先输入路径',
        hint: '输入绝对路径后，工作台才能判断是打开已有 workspace，还是直接初始化。',
        disabled: true,
        run: 'none'
      };
    }

    if (!workspaceProbe) {
      return {
        label: '检查并继续',
        hint: '先 probe 路径，再决定下一步动作。',
        disabled: busy || pickerBusy,
        run: 'probe'
      };
    }

    if (workspaceProbe.selectable && workspaceProbe.manifest_present) {
      return {
        label: '打开 workspace',
        hint: '路径已经包含 agentstow.toml，可以直接进入工作台。',
        disabled: busy || pickerBusy,
        run: 'open'
      };
    }

    if (workspaceProbe.initializable) {
      return {
        label: '创建并初始化',
        hint: workspaceProbe.exists
          ? '目录已存在但还没有 agentstow.toml，可以直接初始化成新 workspace。'
          : '路径不存在，AgentStow 会先创建目录，再初始化 workspace。',
        disabled: busy || pickerBusy,
        run: 'init'
      };
    }

    return {
      label: '重新检查路径',
      hint: workspaceProbe.reason ?? '当前路径不可直接打开，请先修正路径或重新探测。',
      disabled: busy || pickerBusy,
      run: 'probe'
    };
  });

  function runPrimaryAction(): void {
    if (primaryAction.disabled) {
      return;
    }

    if (primaryAction.run === 'probe') {
      void onProbeWorkspace();
      return;
    }
    if (primaryAction.run === 'open') {
      void onOpenWorkspace();
      return;
    }
    if (primaryAction.run === 'init') {
      void onInitWorkspace();
    }
  }
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

      <div class="boot__decision surface">
        <div class="boot__decision-copy">
          <p class="boot__decision-label">下一步</p>
          <strong>{primaryAction.label}</strong>
          <p>{primaryAction.hint}</p>
        </div>
        <button
          class="ui-button ui-button--primary boot__decision-action"
          data-testid="workspace-primary-action"
          disabled={primaryAction.disabled}
          type="button"
          onclick={runPrimaryAction}
        >
          {busy && primaryAction.run !== 'none' ? '处理中…' : primaryAction.label}
        </button>
      </div>

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
            <span class="inspector-row__label">目录状态</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.exists ? '已存在' : '尚不存在'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Manifest</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.manifest_present ? 'agentstow.toml 已就绪' : 'agentstow.toml 缺失'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">可直接打开</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.selectable ? '可以' : '不可以'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">可初始化</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.initializable ? '可以' : '不可以'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Git 仓库</span>
            <span class="inspector-row__value inspector-row__value--mono">
              {workspaceProbe.git_present ? '已存在' : '尚未初始化'}
            </span>
          </div>
          <div class="inspector-row">
            <span class="inspector-row__label">Manifest 路径</span>
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
  .boot__decision {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 18px;
    align-items: center;
    padding: 16px 18px;
    border-radius: 8px;
    border: 1px solid color-mix(in oklch, var(--line) 72%, transparent);
    background: color-mix(in oklch, var(--primary-soft) 42%, var(--panel-bg));
  }

  .boot__decision-copy {
    display: grid;
    gap: 4px;
    min-width: 0;
  }

  .boot__decision-label {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 11px;
    color: var(--ink-muted);
  }

  .boot__decision-copy strong {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 20px;
    line-height: 1;
    letter-spacing: -0.03em;
  }

  .boot__decision-copy p {
    margin: 0;
    color: var(--ink-soft);
  }

  .boot__decision-action {
    min-width: 180px;
  }

  .boot__probe {
    display: grid;
    gap: 12px;
  }

  .boot__probe-head {
    display: grid;
    gap: 8px;
  }

  .compound-action--boot {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  @media (max-width: 900px) {
    .boot__decision {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .compound-action--boot {
      grid-template-columns: 1fr;
    }

    .boot__decision-action {
      width: 100%;
    }
  }
</style>
