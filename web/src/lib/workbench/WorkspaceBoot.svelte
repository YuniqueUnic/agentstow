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

  function activateOnKey(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }
    event.preventDefault();
    action();
  }
</script>

<div class="boot">
  <div class="boot__panel surface" role="region" aria-label="Workspace 引导">
    <p class="boot__eyebrow">AgentStow Workbench</p>
    <h1>选择或初始化 Workspace</h1>
    <p class="boot__lead">
      你可以打开一个已经包含 <code>agentstow.toml</code> 的目录，也可以在任意目录里
      初始化一个新的 workspace。
    </p>

    <div class="boot__form">
      <md-outlined-text-field
        label="Workspace 路径"
        placeholder="/path/to/workspace"
        value={workspaceInput}
        oninput={(event) => {
          const target = event.currentTarget as { value?: string } | null;
          onWorkspaceInput(typeof target?.value === 'string' ? target.value : '');
        }}
        supporting-text="服务端不会自动弹出文件选择框，直接输入本机路径即可。"
      ></md-outlined-text-field>

      <div class="boot__toggle" role="group" aria-label="初始化选项">
        <md-checkbox
          checked={initGit}
          onchange={(event: Event) => {
            const target = event.target as unknown as { checked?: unknown } | null;
            onInitGit(Boolean(target?.checked));
          }}
          aria-label="初始化时执行 git init"
        ></md-checkbox>
        <span>初始化时执行 <code>git init</code></span>
      </div>

      <div class="boot__actions">
        <md-outlined-button
          disabled={busy}
          onclick={() => void onOpenWorkspace()}
          onkeydown={(event) => activateOnKey(event, () => void onOpenWorkspace())}
          role="button"
          tabindex="0"
        >
          打开 workspace
        </md-outlined-button>
        <md-filled-tonal-button
          disabled={busy}
          onclick={() => void onInitWorkspace()}
          onkeydown={(event) => activateOnKey(event, () => void onInitWorkspace())}
          role="button"
          tabindex="0"
        >
          {busy ? '处理中…' : '初始化 workspace'}
        </md-filled-tonal-button>
      </div>
    </div>

    {#if errorMessage}
      <p class="notice notice--error">{errorMessage}</p>
    {/if}
    <p class="boot__status" aria-live="polite">{statusLine}</p>
  </div>
</div>

