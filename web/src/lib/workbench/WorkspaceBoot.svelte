<script lang="ts">
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
        <span class="field__hint">服务端不会自动弹出文件选择框，直接输入本机路径即可。</span>
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

    {#if errorMessage}
      <p class="notice notice--error">{errorMessage}</p>
    {/if}
    <p class="boot__status" aria-live="polite">{statusLine}</p>
  </div>
</div>
