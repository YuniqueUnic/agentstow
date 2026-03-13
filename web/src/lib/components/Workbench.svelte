<script lang="ts">
  import { onMount } from 'svelte';

  import { Pane, Splitpanes } from 'svelte-splitpanes';

  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import {
    ApiClientError,
    getArtifactSource,
    getWatchStatus,
    getWorkspaceState,
    getWorkspaceSummary,
    initWorkspace,
    renderArtifact,
    selectWorkspace,
    updateArtifactSource
  } from '$lib/api/client';
  import type {
    ArtifactSourceResponse,
    WorkspaceStateResponse,
    WorkspaceSummaryResponse,
    WatchStatusResponse
  } from '$lib/types';
  import {
    basenameFromPath,
    formatRelativeTime,
    truncateMiddle
  } from '$lib/utils/format';

  type ViewKey = 'artifacts' | 'links' | 'env' | 'scripts' | 'mcp' | 'impact';

  let view = $state<ViewKey>('artifacts');

  let workspaceState = $state<WorkspaceStateResponse | null>(null);
  let workspaceInput = $state('');
  let initGit = $state(false);

  let summary = $state<WorkspaceSummaryResponse | null>(null);
  let watchStatus = $state<WatchStatusResponse | null>(null);

  let selectedArtifact = $state<string | null>(null);
  let selectedProfile = $state<string | null>(null);

  let source = $state<ArtifactSourceResponse | null>(null);
  let editorText = $state('');
  let previewText = $state('');
  let dirty = $state(false);

  let busy = $state({
    boot: false,
    summary: false,
    watch: false,
    source: false,
    save: false,
    preview: false
  });

  let errorMessage = $state<string | null>(null);
  let statusLine = $state('等待连接到 AgentStow server…');

  const manifestPresent = $derived(workspaceState?.manifest_present ?? false);
  const workspaceRoot = $derived(
    workspaceState?.workspace_root ?? summary?.workspace_root ?? null
  );
  const workspaceLabel = $derived(
    workspaceRoot ? basenameFromPath(workspaceRoot) : '未选择 workspace'
  );

  const artifacts = $derived(summary?.artifacts ?? []);
  const profiles = $derived(summary?.profiles ?? []);
  const fileArtifacts = $derived(artifacts.filter((artifact) => artifact.kind === 'file'));
  const profileIds = $derived(profiles.map((profile) => profile.id));

  const watchPill = $derived.by(() => {
    if (!watchStatus) {
      return { tone: 'neutral', label: 'watcher 未连接' };
    }
    if (!watchStatus.healthy) {
      return { tone: 'warn', label: watchStatus.mode };
    }
    return { tone: 'ok', label: watchStatus.mode };
  });

  const watchActivity = $derived.by(() => {
    if (!watchStatus?.last_event) {
      return '等待文件变化';
    }
    return `${watchStatus.last_event} · ${formatRelativeTime(watchStatus.last_event_at)}`;
  });

  function describeError(error: unknown, fallback: string): string {
    if (error instanceof ApiClientError) {
      return error.message;
    }
    if (error instanceof Error && error.message) {
      return error.message;
    }
    return fallback;
  }

  function activateOnKey(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }

    event.preventDefault();
    action();
  }

  async function refreshWorkspaceState(): Promise<void> {
    try {
      workspaceState = await getWorkspaceState();
      if (workspaceInput.trim().length === 0 && workspaceState.workspace_root) {
        workspaceInput = workspaceState.workspace_root;
      }
    } catch (error) {
      errorMessage = describeError(error, '无法读取 workspace 状态。');
    }
  }

  async function refreshSummary(): Promise<void> {
    busy.summary = true;
    errorMessage = null;
    try {
      const nextSummary = await getWorkspaceSummary();
      summary = nextSummary;

      if (!selectedArtifact) {
        selectedArtifact =
          nextSummary.artifacts.find((artifact) => artifact.kind === 'file')?.id ?? null;
      }
      if (!selectedProfile) {
        selectedProfile = nextSummary.profiles[0]?.id ?? null;
      }
    } catch (error) {
      errorMessage = describeError(error, '无法读取 workspace 数据。');
      summary = null;
    } finally {
      busy.summary = false;
    }
  }

  async function refreshWatchStatus(): Promise<void> {
    busy.watch = true;
    try {
      watchStatus = await getWatchStatus();
    } catch (error) {
      watchStatus = null;
      errorMessage ??= describeError(error, '无法读取 watcher 状态。');
    } finally {
      busy.watch = false;
    }
  }

  async function bootstrapConfigured(): Promise<void> {
    await Promise.all([refreshSummary(), refreshWatchStatus()]);
    statusLine = '已连接到 workspace。';
  }

  async function handleSelectWorkspace(): Promise<void> {
    busy.boot = true;
    errorMessage = null;
    try {
      const resp = await selectWorkspace(workspaceInput.trim());
      workspaceState = {
        workspace_root: resp.workspace_root,
        manifest_present: resp.manifest_present
      };
      if (!resp.manifest_present) {
        statusLine = '已选择目录，但还没有 agentstow.toml。你可以点击“初始化 workspace”。';
        return;
      }
      await bootstrapConfigured();
    } catch (error) {
      errorMessage = describeError(error, '选择 workspace 失败。');
    } finally {
      busy.boot = false;
    }
  }

  async function handleInitWorkspace(): Promise<void> {
    busy.boot = true;
    errorMessage = null;
    try {
      const resp = await initWorkspace({
        workspace_root: workspaceInput.trim(),
        git_init: initGit
      });
      workspaceState = {
        workspace_root: resp.workspace_root,
        manifest_present: true
      };
      statusLine = resp.created ? '已初始化 workspace。' : 'workspace 已存在 manifest，已直接打开。';
      await bootstrapConfigured();
    } catch (error) {
      errorMessage = describeError(error, '初始化 workspace 失败。');
    } finally {
      busy.boot = false;
    }
  }

  async function loadArtifactSource(artifactId: string): Promise<void> {
    busy.source = true;
    errorMessage = null;
    try {
      source = await getArtifactSource(artifactId);
      editorText = source.content;
      dirty = false;
    } catch (error) {
      errorMessage = describeError(error, '无法读取 artifact source。');
      source = null;
      editorText = '';
      dirty = false;
    } finally {
      busy.source = false;
    }
  }

  async function saveArtifactSource(): Promise<void> {
    if (!selectedArtifact) {
      return;
    }

    busy.save = true;
    errorMessage = null;
    try {
      source = await updateArtifactSource(selectedArtifact, editorText);
      dirty = false;
      statusLine = '已保存 source。';
    } catch (error) {
      errorMessage = describeError(error, '保存失败。');
    } finally {
      busy.save = false;
    }
  }

  async function refreshPreview(): Promise<void> {
    if (!selectedArtifact || !selectedProfile) {
      return;
    }

    busy.preview = true;
    errorMessage = null;
    try {
      const resp = await renderArtifact(selectedArtifact, selectedProfile);
      previewText = resp.text;
    } catch (error) {
      previewText = '';
      errorMessage = describeError(error, '渲染预览失败。');
    } finally {
      busy.preview = false;
    }
  }

  function selectArtifact(id: string): void {
    selectedArtifact = id;
    statusLine = `已选择 artifact：${id}`;
  }

  function selectProfile(id: string): void {
    selectedProfile = id;
    statusLine = `已选择 profile：${id}`;
  }

  onMount(() => {
    void (async () => {
      await refreshWorkspaceState();
      if (manifestPresent) {
        await bootstrapConfigured();
      } else if (workspaceState?.workspace_root) {
        statusLine = '当前目录没有 manifest。你可以初始化或切换到已有 workspace。';
      } else {
        statusLine = '请选择一个 workspace。';
      }
    })();
  });

  $effect(() => {
    if (!manifestPresent) {
      return;
    }
    if (!selectedArtifact) {
      return;
    }

    void loadArtifactSource(selectedArtifact);
  });

  $effect(() => {
    if (!manifestPresent) {
      return;
    }
    if (!selectedArtifact || !selectedProfile) {
      return;
    }

    void refreshPreview();
  });
</script>

<div class="frame">
  {#if !manifestPresent}
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
              workspaceInput = typeof target?.value === 'string' ? target.value : '';
            }}
            supporting-text="服务端不会自动弹出文件选择框，直接输入本机路径即可。"
          ></md-outlined-text-field>

          <div class="boot__toggle" role="group" aria-label="初始化选项">
            <md-checkbox
              checked={initGit}
              onchange={(e) => (initGit = Boolean((e.target as { checked?: unknown }).checked))}
              aria-label="初始化时执行 git init"
            ></md-checkbox>
            <span>初始化时执行 <code>git init</code></span>
          </div>

          <div class="boot__actions">
            <md-outlined-button
              disabled={busy.boot}
              onclick={() => void handleSelectWorkspace()}
              onkeydown={(event) => activateOnKey(event, () => void handleSelectWorkspace())}
              role="button"
              tabindex="0"
            >
              打开 workspace
            </md-outlined-button>
            <md-filled-tonal-button
              disabled={busy.boot}
              onclick={() => void handleInitWorkspace()}
              onkeydown={(event) => activateOnKey(event, () => void handleInitWorkspace())}
              role="button"
              tabindex="0"
            >
              {busy.boot ? '处理中…' : '初始化 workspace'}
            </md-filled-tonal-button>
          </div>
        </div>

        {#if errorMessage}
          <p class="notice notice--error">{errorMessage}</p>
        {/if}
        <p class="boot__status" aria-live="polite">{statusLine}</p>
      </div>
    </div>
  {:else}
    <div class="workbench">
      <header class="topbar surface">
        <div class="topbar__brand">
          <span class="mark" aria-hidden="true"></span>
          <div>
            <strong>AgentStow</strong>
            <span class="muted">workbench</span>
          </div>
        </div>

        <div class="topbar__workspace" title={workspaceRoot ?? ''}>
          <span class="muted">Workspace</span>
          <span class="mono">{workspaceLabel}</span>
          <md-text-button onclick={() => (workspaceState = { workspace_root: workspaceRoot, manifest_present: false })}>
            切换
          </md-text-button>
        </div>

        <div class="topbar__status">
          <span class={['pill', `pill--${watchPill.tone}`].join(' ')}>
            {watchPill.label}
          </span>
          <span class="muted" title={watchActivity}>
            {truncateMiddle(watchActivity, 28)}
          </span>
        </div>

        <div class="topbar__actions">
          <md-outlined-button disabled={busy.summary} onclick={() => void bootstrapConfigured()}>
            刷新
          </md-outlined-button>
        </div>
      </header>

      <nav class="rail surface" aria-label="主导航">
        {#each [
          ['artifacts', 'A'],
          ['links', 'L'],
          ['env', 'E'],
          ['scripts', 'S'],
          ['mcp', 'M'],
          ['impact', 'I']
        ] as [key, label]}
          <button
            class={['rail__item', view === key ? 'rail__item--active' : ''].join(' ')}
            onclick={() => (view = key as ViewKey)}
            type="button"
          >
            <span class="rail__glyph" aria-hidden="true">{label}</span>
            <span class="rail__label">{key}</span>
          </button>
        {/each}
      </nav>

      <aside class="explorer surface" aria-label="资源面板">
        <div class="explorer__head">
          <p class="explorer__eyebrow">{view.toUpperCase()}</p>
          <p class="explorer__hint">选择对象后在右侧编辑与预览</p>
        </div>

        {#if view === 'artifacts'}
          <div class="explorer__section">
            <div class="section__title">
              <span>Artifacts</span>
              <strong>{fileArtifacts.length}</strong>
            </div>
            <ul class="list">
              {#each fileArtifacts as artifact (artifact.id)}
                <li>
                  <button
                    class={['list__item', selectedArtifact === artifact.id ? 'list__item--active' : ''].join(' ')}
                    onclick={() => selectArtifact(artifact.id)}
                    type="button"
                  >
                    <span class="list__dot" aria-hidden="true"></span>
                    <span class="list__name">{artifact.id}</span>
                    <span class="list__meta">{artifact.validate_as}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>

          <div class="explorer__section">
            <div class="section__title">
              <span>Profiles</span>
              <strong>{profileIds.length}</strong>
            </div>
            <div class="chips">
              {#each profileIds as profileId (profileId)}
                <button
                  class={['chip', selectedProfile === profileId ? 'chip--active' : ''].join(' ')}
                  onclick={() => selectProfile(profileId)}
                  type="button"
                >
                  {profileId}
                </button>
              {/each}
            </div>
          </div>
        {:else if view === 'env'}
          <div class="explorer__section">
            <div class="section__title">
              <span>Env Sets</span>
              <strong>{summary?.env_sets.length ?? 0}</strong>
            </div>
            <ul class="list">
              {#each summary?.env_sets ?? [] as envSet (envSet.id)}
                <li class="list__static">
                  <span class="list__name">{envSet.id}</span>
                  <span class="list__meta">{envSet.vars.length} vars</span>
                </li>
              {/each}
            </ul>
          </div>
        {:else}
          <p class="empty">
            当前视图还在迭代中。先把 artifacts 编辑/预览闭环打通，再把其余 PRD 面板逐步补齐。
          </p>
        {/if}
      </aside>

      <main class="canvas" aria-label="工作区画布">
        {#if view !== 'artifacts'}
          <div class="canvas__placeholder surface">
            <h2>{view}</h2>
            <p class="muted">该面板下一步按 PRD 补齐（CRUD、diff、impact、env 编辑等）。</p>
          </div>
        {:else}
          <div class="canvas__head">
            <div class="title">
              <strong>{selectedArtifact ?? '未选择 artifact'}</strong>
              <span class="muted">{selectedProfile ? `· ${selectedProfile}` : ''}</span>
            </div>

            <div class="canvas__actions">
              <md-outlined-button disabled={!dirty || busy.save} onclick={() => void saveArtifactSource()}>
                {busy.save ? '保存中…' : dirty ? '保存' : '已保存'}
              </md-outlined-button>
              <md-filled-tonal-button disabled={busy.preview || !selectedProfile} onclick={() => void refreshPreview()}>
                {busy.preview ? '渲染中…' : '渲染预览'}
              </md-filled-tonal-button>
            </div>
          </div>

          {#if errorMessage}
            <p class="notice notice--error">{errorMessage}</p>
          {/if}
          <p class="status-line" aria-live="polite">{statusLine}</p>

          <div class="split surface">
            <Splitpanes>
              <Pane minSize={28} size={52}>
                <div class="pane">
                  <div class="pane__title">Source</div>
                  <div class="pane__body">
                    {#if busy.source}
                      <p class="muted">读取中…</p>
                    {:else}
                      <CodeEditor
                        value={editorText}
                        onChange={(next) => {
                          editorText = next;
                          dirty = source?.content !== next;
                        }}
                      />
                    {/if}
                  </div>
                </div>
              </Pane>
              <Pane minSize={28}>
                <div class="pane">
                  <div class="pane__title">Preview</div>
                  <div class="pane__body">
                    <pre class="preview">{previewText || '（暂无预览）'}</pre>
                  </div>
                </div>
              </Pane>
            </Splitpanes>
          </div>
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  .frame {
    position: relative;
    min-height: 100vh;
  }

  .surface {
    background: color-mix(in oklch, var(--surface) 74%, white);
    border: 1px solid color-mix(in oklch, var(--line) 70%, white);
    border-radius: 26px;
    box-shadow: var(--shadow);
  }

  .muted {
    color: var(--ink-soft);
  }

  .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    letter-spacing: -0.02em;
  }

  .boot {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: clamp(24px, 4vw, 56px);
  }

  .boot__panel {
    max-width: 720px;
    padding: clamp(22px, 3.2vw, 42px);
  }

  .boot__eyebrow {
    margin: 0 0 6px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 12px;
    color: var(--ink-soft);
  }

  .boot h1 {
    margin: 0 0 10px;
    font-family: 'Manrope', sans-serif;
    font-size: clamp(30px, 4vw, 42px);
    letter-spacing: -0.04em;
  }

  .boot__lead {
    margin: 0 0 22px;
    color: var(--ink-soft);
    max-width: 54ch;
  }

  .boot__form {
    display: grid;
    gap: 16px;
  }

  .boot__toggle {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ink-soft);
    font-size: 14px;
  }

  .boot__toggle code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    color: var(--ink);
  }

  .boot__actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .boot__status {
    margin: 14px 0 0;
    color: var(--ink-soft);
  }

  .notice {
    margin: 14px 0 0;
    padding: 12px 14px;
    border-radius: 18px;
    font-size: 14px;
    line-height: 1.45;
  }

  .notice--error {
    background: color-mix(in oklch, var(--danger) 14%, white);
    border: 1px solid color-mix(in oklch, var(--danger) 38%, white);
    color: color-mix(in oklch, var(--danger) 62%, var(--ink));
  }

  .workbench {
    height: 100vh;
    padding: clamp(12px, 2.2vw, 22px);
    display: grid;
    grid-template-columns: 86px minmax(260px, 320px) minmax(0, 1fr);
    grid-template-rows: 64px 1fr;
    gap: 14px;
  }

  .topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 18px;
  }

  .topbar__brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .mark {
    width: 14px;
    height: 14px;
    border-radius: 5px;
    background: linear-gradient(135deg, var(--accent), var(--primary));
    box-shadow: 0 0 0 4px color-mix(in oklch, var(--primary) 10%, transparent);
  }

  .topbar__brand strong {
    display: block;
    font-family: 'Manrope', sans-serif;
    letter-spacing: -0.02em;
  }

  .topbar__brand .muted {
    display: block;
    font-size: 12px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  .topbar__workspace {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  .topbar__workspace .mono {
    white-space: nowrap;
  }

  .topbar__status {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    flex: 1;
    justify-content: center;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 999px;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border: 1px solid transparent;
    background: color-mix(in oklch, white 76%, transparent);
  }

  .pill--ok {
    border-color: color-mix(in oklch, var(--success) 38%, white);
    background: color-mix(in oklch, var(--success) 14%, white);
    color: color-mix(in oklch, var(--success) 68%, var(--ink));
  }

  .pill--warn {
    border-color: color-mix(in oklch, var(--accent) 48%, white);
    background: color-mix(in oklch, var(--accent) 18%, white);
    color: color-mix(in oklch, var(--accent) 72%, var(--ink));
  }

  .pill--neutral {
    border-color: color-mix(in oklch, var(--line) 74%, white);
    color: var(--ink-soft);
  }

  .topbar__actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .rail {
    padding: 10px 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: stretch;
  }

  .rail__item {
    border: 0;
    background: transparent;
    padding: 10px 8px;
    border-radius: 22px;
    display: grid;
    gap: 4px;
    justify-items: center;
    cursor: pointer;
    color: var(--ink-soft);
    transition: background 160ms ease, color 160ms ease;
  }

  .rail__item--active {
    background: color-mix(in oklch, var(--primary) 14%, white);
    color: color-mix(in oklch, var(--primary) 62%, var(--ink));
  }

  .rail__glyph {
    width: 40px;
    height: 40px;
    border-radius: 18px;
    display: grid;
    place-items: center;
    font-family: 'Manrope', sans-serif;
    font-size: 16px;
    letter-spacing: -0.04em;
    background: color-mix(in oklch, white 84%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 66%, white);
  }

  .rail__label {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .explorer {
    padding: 16px 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow: hidden;
  }

  .explorer__head {
    display: grid;
    gap: 4px;
  }

  .explorer__eyebrow {
    margin: 0;
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  .explorer__hint {
    margin: 0;
    font-size: 13px;
    color: var(--ink-soft);
  }

  .explorer__section {
    display: grid;
    gap: 10px;
    min-height: 0;
  }

  .section__title {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  .section__title strong {
    font-family: 'Manrope', sans-serif;
    color: var(--ink);
    letter-spacing: -0.02em;
  }

  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 8px;
    overflow: auto;
  }

  .list__item {
    width: 100%;
    border: 0;
    text-align: left;
    background: color-mix(in oklch, white 84%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 62%, white);
    border-radius: 18px;
    padding: 10px 12px;
    display: grid;
    grid-template-columns: 10px 1fr auto;
    gap: 10px;
    align-items: center;
    cursor: pointer;
    transition: transform 160ms ease, background 160ms ease, border-color 160ms ease;
  }

  .list__item:hover {
    transform: translateY(-1px);
    background: color-mix(in oklch, var(--primary) 10%, white);
    border-color: color-mix(in oklch, var(--primary) 20%, white);
  }

  .list__item--active {
    background: color-mix(in oklch, var(--primary) 14%, white);
    border-color: color-mix(in oklch, var(--primary) 30%, white);
  }

  .list__dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--primary) 62%, white);
  }

  .list__name {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }

  .list__meta {
    font-size: 12px;
    color: var(--ink-soft);
    text-transform: uppercase;
    letter-spacing: 0.14em;
  }

  .list__static {
    display: flex;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 18px;
    background: color-mix(in oklch, white 86%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 60%, white);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .chip {
    border: 1px solid color-mix(in oklch, var(--line) 70%, white);
    background: color-mix(in oklch, white 88%, transparent);
    border-radius: 999px;
    padding: 8px 12px;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    cursor: pointer;
    color: var(--ink-soft);
  }

  .chip--active {
    background: color-mix(in oklch, var(--primary) 16%, white);
    border-color: color-mix(in oklch, var(--primary) 30%, white);
    color: color-mix(in oklch, var(--primary) 62%, var(--ink));
  }

  .empty {
    margin: 0;
    padding: 12px 14px;
    border-radius: 18px;
    background: color-mix(in oklch, white 86%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 62%, white);
    color: var(--ink-soft);
    line-height: 1.5;
    font-size: 14px;
  }

  .canvas {
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 12px;
    min-width: 0;
  }

  .canvas__placeholder {
    padding: 24px;
    display: grid;
    gap: 8px;
    align-content: start;
  }

  .canvas__placeholder h2 {
    margin: 0;
    font-family: 'Manrope', sans-serif;
    letter-spacing: -0.03em;
  }

  .canvas__head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 14px;
    padding: 0 8px;
  }

  .canvas__head .title {
    font-size: 16px;
    letter-spacing: -0.02em;
  }

  .canvas__actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .status-line {
    margin: 0;
    padding: 0 8px;
    font-size: 13px;
    color: var(--ink-soft);
  }

  .split {
    min-height: 0;
    padding: 14px;
  }

  .pane {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    min-height: 0;
  }

  .pane__title {
    font-size: 12px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink-soft);
    padding: 0 6px;
  }

  .pane__body {
    min-height: 0;
  }

  .preview {
    height: 100%;
    margin: 0;
    padding: 14px;
    border-radius: 18px;
    background: color-mix(in oklch, white 86%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 68%, white);
    overflow: auto;
    white-space: pre-wrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    font-size: 13px;
    line-height: 1.55;
    color: var(--ink);
  }

  @media (max-width: 960px) {
    .workbench {
      grid-template-columns: 86px minmax(0, 1fr);
      grid-template-rows: 64px 1fr;
    }

    .explorer {
      display: none;
    }
  }

  @media (max-width: 680px) {
    .workbench {
      grid-template-columns: 1fr;
      grid-template-rows: 64px auto 1fr;
    }

    .rail {
      grid-column: 1;
      flex-direction: row;
      justify-content: space-between;
    }
  }

  :global(md-outlined-text-field) {
    width: 100%;
  }
  :global(md-outlined-button),
  :global(md-filled-tonal-button),
  :global(md-text-button) {
    --md-sys-typescale-label-large-font: 'Instrument Sans', sans-serif;
  }
  :global(.splitpanes__splitter) {
    background: transparent;
  }
  :global(.splitpanes__splitter:before) {
    background: color-mix(in oklch, var(--primary) 18%, transparent);
  }
</style>
