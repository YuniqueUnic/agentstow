<script lang="ts">
  import { onMount } from 'svelte';

  import {
    ApiClientError,
    getLinkStatus,
    getLinks,
    getManifest,
    getWatchStatus,
    renderArtifact
  } from '$lib/api/client';
  import type {
    LinkRecordResponse,
    LinkStatusResponseItem,
    ManifestResponse,
    WatchStatusResponse
  } from '$lib/types';
  import {
    basenameFromPath,
    formatRelativeTime,
    truncateMiddle
  } from '$lib/utils/format';

  type LoadingState = {
    manifest: boolean;
    links: boolean;
    linkStatus: boolean;
    watchStatus: boolean;
    render: boolean;
    copy: boolean;
  };

  type ErrorState = {
    manifest: string | null;
    links: string | null;
    linkStatus: string | null;
    watchStatus: string | null;
    render: string | null;
  };

  let manifest = $state<ManifestResponse | null>(null);
  let links = $state<LinkRecordResponse[]>([]);
  let linkStatus = $state<LinkStatusResponseItem[]>([]);
  let watchStatus = $state<WatchStatusResponse | null>(null);
  let renderedText = $state('');
  let filterText = $state('');
  let selectedArtifact = $state<string | null>(null);
  let selectedProfile = $state<string | null>(null);
  let statusMessage = $state('等待连接到 AgentStow 工作区…');
  let lastSeenWatchRevision = $state<number | null>(null);

  let loading = $state<LoadingState>({
    manifest: false,
    links: false,
    linkStatus: false,
    watchStatus: false,
    render: false,
    copy: false
  });

  let errors = $state<ErrorState>({
    manifest: null,
    links: null,
    linkStatus: null,
    watchStatus: null,
    render: null
  });

  const artifacts = $derived(manifest?.artifacts ?? []);
  const profiles = $derived(manifest?.profiles ?? []);
  const targetCount = $derived(manifest?.targets.length ?? 0);
  const healthyCount = $derived(linkStatus.filter((item) => item.ok).length);
  const unhealthyCount = $derived(linkStatus.filter((item) => !item.ok).length);
  const selectedDescriptor = $derived(
    selectedArtifact && selectedProfile
      ? `${selectedArtifact} · ${selectedProfile}`
      : '请选择 artifact 与 profile'
  );
  const workspaceLabel = $derived(
    manifest ? basenameFromPath(manifest.workspace_root) : 'AgentStow workspace'
  );
  const canRender = $derived(
    Boolean(selectedArtifact && selectedProfile) && !loading.render && !loading.manifest
  );
  const watchModeLabel = $derived(
    watchStatus
      ? watchStatus.mode === 'native'
        ? 'native watcher'
        : watchStatus.mode === 'poll'
          ? 'poll fallback'
          : 'manual refresh'
      : 'watcher 未连接'
  );
  const autoRefreshLabel = $derived(
    watchStatus?.healthy && watchStatus.mode !== 'manual' ? '自动刷新已启用' : '仅支持手动刷新'
  );
  const watchActivityLabel = $derived(
    watchStatus?.last_event
      ? `${watchStatus.last_event} · ${formatRelativeTime(watchStatus.last_event_at)}`
      : '等待文件变化'
  );
  const filteredStatus = $derived.by(() => filterCollection(linkStatus, filterText));
  const filteredLinks = $derived.by(() =>
    [...filterCollection(links, filterText)].sort((left, right) =>
      right.updated_at.localeCompare(left.updated_at)
    )
  );
  let refreshInFlight = false;
  let watchPollInFlight = false;

  function filterCollection<T extends Record<string, unknown>>(items: T[], query: string): T[] {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return items;
    }

    return items.filter((item) =>
      Object.values(item).some((value) =>
        String(value ?? '')
          .toLowerCase()
          .includes(normalized)
      )
    );
  }

  function describeError(error: unknown, fallback: string): string {
    if (error instanceof ApiClientError) {
      return error.message;
    }

    if (error instanceof Error && error.message) {
      return error.message;
    }

    return fallback;
  }

  function syncSelections(nextManifest: ManifestResponse): void {
    if (!nextManifest.artifacts.includes(selectedArtifact ?? '')) {
      selectedArtifact = nextManifest.artifacts[0] ?? null;
    }

    if (!nextManifest.profiles.includes(selectedProfile ?? '')) {
      selectedProfile = nextManifest.profiles[0] ?? null;
    }
  }

  function readTextValue(event: Event): string {
    const target = event.currentTarget as { value?: string } | null;
    return typeof target?.value === 'string' ? target.value : '';
  }

  function chooseArtifact(artifact: string): void {
    selectedArtifact = artifact;
    statusMessage = `已选择 artifact：${artifact}`;
  }

  function chooseProfile(profile: string): void {
    selectedProfile = profile;
    statusMessage = `已选择 profile：${profile}`;
  }

  function applyLinkSelection(item: { artifact_id: string; profile: string }): void {
    selectedArtifact = item.artifact_id;
    selectedProfile = item.profile;
    statusMessage = `已载入 ${item.artifact_id} · ${item.profile} 到渲染面板。`;
  }

  function triggerOnEnterOrSpace(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }

    event.preventDefault();
    action();
  }

  async function loadManifest(): Promise<boolean> {
    loading.manifest = true;
    errors.manifest = null;

    try {
      const nextManifest = await getManifest();
      manifest = nextManifest;
      syncSelections(nextManifest);
      return true;
    } catch (error) {
      errors.manifest = describeError(error, '无法读取 manifest。');
      return false;
    } finally {
      loading.manifest = false;
    }
  }

  async function loadLinks(): Promise<boolean> {
    loading.links = true;
    errors.links = null;

    try {
      links = await getLinks();
      return true;
    } catch (error) {
      errors.links = describeError(error, '无法读取 links 记录。');
      links = [];
      return false;
    } finally {
      loading.links = false;
    }
  }

  async function loadLinkStatus(): Promise<boolean> {
    loading.linkStatus = true;
    errors.linkStatus = null;

    try {
      linkStatus = await getLinkStatus();
      return true;
    } catch (error) {
      errors.linkStatus = describeError(error, '无法读取 link 健康状态。');
      linkStatus = [];
      return false;
    } finally {
      loading.linkStatus = false;
    }
  }

  async function loadWatchStatus(options: { quiet?: boolean } = {}): Promise<boolean> {
    loading.watchStatus = true;
    errors.watchStatus = null;

    try {
      watchStatus = await getWatchStatus();
      return true;
    } catch (error) {
      errors.watchStatus = describeError(error, '无法读取 watcher 状态。');
      if (!options.quiet) {
        statusMessage = errors.watchStatus;
      }
      return false;
    } finally {
      loading.watchStatus = false;
    }
  }

  async function refreshAll(options: { quiet?: boolean } = {}): Promise<boolean> {
    if (refreshInFlight) {
      return false;
    }

    refreshInFlight = true;
    if (!options.quiet) {
      statusMessage = '正在同步 manifest、links、watcher 与健康状态…';
    }

    try {
      const [manifestOk, linksOk, statusOk, watchOk] = await Promise.all([
        loadManifest(),
        loadLinks(),
        loadLinkStatus(),
        loadWatchStatus({ quiet: true })
      ]);

      const ok = manifestOk && linksOk && statusOk && watchOk;
      if (!options.quiet) {
        statusMessage = ok ? '工作区数据已刷新。' : '部分数据刷新失败，请查看各面板提示。';
      }
      return ok;
    } finally {
      refreshInFlight = false;
    }
  }

  async function runRender(): Promise<void> {
    if (!selectedArtifact || !selectedProfile) {
      statusMessage = '请先选择 artifact 与 profile。';
      return;
    }

    loading.render = true;
    errors.render = null;
    statusMessage = `正在渲染 ${selectedArtifact} · ${selectedProfile}…`;

    try {
      const response = await renderArtifact(selectedArtifact, selectedProfile);
      renderedText = response.text;
      statusMessage = `渲染完成：${selectedArtifact} · ${selectedProfile}`;
    } catch (error) {
      errors.render = describeError(error, '渲染失败。');
      statusMessage = '渲染失败，请检查输入与服务端错误信息。';
    } finally {
      loading.render = false;
    }
  }

  async function copyPreview(): Promise<void> {
    if (!renderedText) {
      statusMessage = '当前没有可复制的渲染结果。';
      return;
    }

    loading.copy = true;

    try {
      await navigator.clipboard.writeText(renderedText);
      statusMessage = '渲染结果已复制到剪贴板。';
    } catch (error) {
      statusMessage = describeError(error, '复制失败，请检查浏览器权限。');
    } finally {
      loading.copy = false;
    }
  }

  onMount(() => {
    let disposed = false;
    let watchTimer: number | null = null;

    async function bootstrap(): Promise<void> {
      const ok = await refreshAll();
      if (!disposed && ok) {
        lastSeenWatchRevision = watchStatus?.revision ?? null;
      }
    }

    async function pollWatchStatus(): Promise<void> {
      if (disposed || watchPollInFlight) {
        return;
      }

      watchPollInFlight = true;
      try {
        const ok = await loadWatchStatus({ quiet: true });
        if (!ok || !watchStatus) {
          return;
        }

        const nextRevision = watchStatus.revision;
        const autoRefreshReady = watchStatus.healthy && watchStatus.mode !== 'manual';
        if (
          autoRefreshReady &&
          lastSeenWatchRevision !== null &&
          nextRevision > lastSeenWatchRevision
        ) {
          statusMessage = `检测到工作区变更（rev ${nextRevision}），正在自动刷新…`;
          const refreshed = await refreshAll({ quiet: true });
          statusMessage = refreshed ? '检测到工作区变更，已自动刷新。' : '检测到工作区变更，但自动刷新失败。';
        }

        lastSeenWatchRevision = watchStatus.revision;
      } finally {
        watchPollInFlight = false;
      }
    }

    void bootstrap();
    watchTimer = window.setInterval(() => {
      void pollWatchStatus();
    }, 2_000);

    return () => {
      disposed = true;
      if (watchTimer !== null) {
        window.clearInterval(watchTimer);
      }
    };
  });
</script>

<svelte:head>
  <title>AgentStow Workbench</title>
  <meta
    name="description"
    content="AgentStow 独立前端工作台，用于查看 manifest、link 状态与渲染预览。"
  />
</svelte:head>

<div class="app-shell">
  <header class="hero surface">
    <div class="hero__copy">
      <p class="eyebrow">Git-native source-of-truth workbench</p>
      <h1>AgentStow</h1>
      <p class="hero__lead">
        将 manifest、render 预览与 link 健康检查集中到一个可维护的独立前端里，供
        Rust server 直接托管构建产物。
      </p>
    </div>

    <div class="hero__stats" aria-label="工作区摘要">
      <div class="stat">
        <span>Workspace</span>
        <strong title={manifest?.workspace_root ?? ''}>{workspaceLabel}</strong>
      </div>
      <div class="stat">
        <span>Artifacts</span>
        <strong>{artifacts.length}</strong>
      </div>
      <div class="stat">
        <span>Healthy Links</span>
        <strong>{healthyCount}</strong>
      </div>
      <div class="stat stat--warn">
        <span>Needs Attention</span>
        <strong>{unhealthyCount}</strong>
      </div>
    </div>

    <div class="hero__controls">
      <div class="filter-field">
        <label for="workspace-filter">过滤 artifacts / links / path</label>
        <md-outlined-text-field
          id="workspace-filter"
          aria-label="过滤 artifacts 与 links"
          label="过滤视图"
          placeholder="输入 artifact、profile 或 path"
          value={filterText}
          supporting-text="即时筛选右侧 links 与状态面板"
          oninput={(event) => (filterText = readTextValue(event))}
        ></md-outlined-text-field>
      </div>

      <div class="action-row">
        <md-outlined-button
          onclick={() => void refreshAll()}
          onkeydown={(event) => triggerOnEnterOrSpace(event, () => void refreshAll())}
          role="button"
          tabindex="0"
          disabled={loading.manifest || loading.links || loading.linkStatus || loading.watchStatus}
        >
          刷新数据
        </md-outlined-button>
        <md-filled-tonal-button
          onclick={() => void runRender()}
          onkeydown={(event) => triggerOnEnterOrSpace(event, () => void runRender())}
          role="button"
          tabindex="0"
          disabled={!canRender}
        >
          {loading.render ? '渲染中…' : '渲染预览'}
        </md-filled-tonal-button>
      </div>

      <div class="watch-strip" aria-label="文件监听状态">
        <span
          class={[
            'pill',
            watchStatus?.healthy ? 'pill--ok' : watchStatus?.mode === 'poll' ? 'pill--warn' : 'pill--neutral'
          ]}
        >
          {watchModeLabel}
        </span>
        <span>{autoRefreshLabel}</span>
        <span title={watchActivityLabel}>{truncateMiddle(watchActivityLabel, 20)}</span>
        {#if watchStatus?.watch_roots.length}
          <span title={watchStatus.watch_roots.join('\n')}>
            roots {watchStatus.watch_roots.length}
          </span>
        {/if}
      </div>

      {#if errors.watchStatus}
        <p class="notice notice--error">{errors.watchStatus}</p>
      {/if}
      <p class="status-line" aria-live="polite">{statusMessage}</p>
    </div>
  </header>

  <main class="workspace">
    <section class="surface panel">
      <div class="panel__head">
        <div>
          <p class="panel__eyebrow">Manifest</p>
          <h2>选择上下文</h2>
        </div>
        {#if loading.manifest}
          <md-circular-progress indeterminate aria-label="正在加载 manifest"></md-circular-progress>
        {/if}
      </div>

      {#if errors.manifest}
        <p class="notice notice--error">{errors.manifest}</p>
      {/if}

      {#if manifest}
        <p class="workspace-path" title={manifest.workspace_root}>
          {truncateMiddle(manifest.workspace_root, 26)}
        </p>

        <div class="token-group">
          <div class="token-group__title">
            <span>Artifacts</span>
            <strong>{artifacts.length}</strong>
          </div>
          <md-chip-set aria-label="Artifacts">
            {#each artifacts as artifact (artifact)}
              <md-filter-chip
                label={artifact}
                selected={selectedArtifact === artifact}
                onclick={() => chooseArtifact(artifact)}
                onkeydown={(event) => triggerOnEnterOrSpace(event, () => chooseArtifact(artifact))}
                role="button"
                tabindex="0"
              ></md-filter-chip>
            {/each}
          </md-chip-set>
        </div>

        <div class="token-group">
          <div class="token-group__title">
            <span>Profiles</span>
            <strong>{profiles.length}</strong>
          </div>
          <md-chip-set aria-label="Profiles">
            {#each profiles as profile (profile)}
              <md-filter-chip
                label={profile}
                selected={selectedProfile === profile}
                onclick={() => chooseProfile(profile)}
                onkeydown={(event) => triggerOnEnterOrSpace(event, () => chooseProfile(profile))}
                role="button"
                tabindex="0"
              ></md-filter-chip>
            {/each}
          </md-chip-set>
        </div>

        <div class="manifest-footnote">
          <div>
            <span>当前选择</span>
            <strong>{selectedDescriptor}</strong>
          </div>
          <div>
            <span>Targets</span>
            <strong>{targetCount}</strong>
          </div>
        </div>
      {:else}
        <div class="empty-state">
          <p>还没有拿到 manifest。确认 `agentstow serve` 正在运行，并检查 `/api/manifest`。</p>
        </div>
      {/if}
    </section>

    <section class="surface panel panel--preview">
      <div class="panel__head">
        <div>
          <p class="panel__eyebrow">Render</p>
          <h2>预览输出</h2>
        </div>
        {#if loading.render}
          <md-circular-progress indeterminate aria-label="正在渲染"></md-circular-progress>
        {/if}
      </div>

      <p class="preview-subtitle">{selectedDescriptor}</p>

      {#if errors.render}
        <p class="notice notice--error">{errors.render}</p>
      {/if}

      {#if renderedText}
        <div class="preview-toolbar">
          <md-text-button
            onclick={() => void copyPreview()}
            onkeydown={(event) => triggerOnEnterOrSpace(event, () => void copyPreview())}
            role="button"
            tabindex="0"
            disabled={loading.copy}
          >
            {loading.copy ? '复制中…' : '复制结果'}
          </md-text-button>
          <md-outlined-button
            onclick={() => (renderedText = '')}
            onkeydown={(event) => triggerOnEnterOrSpace(event, () => (renderedText = ''))}
            role="button"
            tabindex="0"
          >
            清空预览
          </md-outlined-button>
        </div>

        <pre class="preview-output">{renderedText}</pre>
      {:else}
        <div class="empty-state empty-state--preview">
          <p>选择 artifact 与 profile 后点击“渲染预览”，这里会显示 `/api/render` 返回的文本。</p>
        </div>
      {/if}
    </section>

    <section class="surface panel panel--status">
      <div class="panel__head">
        <div>
          <p class="panel__eyebrow">Links</p>
          <h2>健康状态与安装记录</h2>
        </div>
        {#if loading.linkStatus || loading.links}
          <md-circular-progress indeterminate aria-label="正在加载 link 数据"></md-circular-progress>
        {/if}
      </div>

      {#if errors.linkStatus}
        <p class="notice notice--error">{errors.linkStatus}</p>
      {/if}

      <div class="status-summary">
        <div>
          <span>Healthy</span>
          <strong>{healthyCount}</strong>
        </div>
        <div>
          <span>Unhealthy</span>
          <strong>{unhealthyCount}</strong>
        </div>
        <div>
          <span>Records</span>
          <strong>{links.length}</strong>
        </div>
      </div>

      <div class="list-block">
        <div class="list-block__head">
          <h3>Link Status</h3>
          <span>{filteredStatus.length}</span>
        </div>

        {#if filteredStatus.length}
          <ul class="stack-list">
            {#each filteredStatus as item (`${item.target_path}:${item.artifact_id}:${item.profile}`)}
              <li class={['stack-row', item.ok ? 'stack-row--ok' : 'stack-row--warn']}>
                <button class="stack-row__button" type="button" onclick={() => applyLinkSelection(item)}>
                  <span class="stack-row__topline">
                    <strong>{item.artifact_id}</strong>
                    <span>{item.profile}</span>
                    <span class={['pill', item.ok ? 'pill--ok' : 'pill--warn']}>
                      {item.message}
                    </span>
                  </span>
                  <span class="stack-row__path">{item.target_path}</span>
                  <span class="stack-row__meta">{item.method}</span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty-state empty-state--compact">
            <p>当前筛选下没有 link status 记录。</p>
          </div>
        {/if}
      </div>

      <md-divider></md-divider>

      <div class="list-block">
        <div class="list-block__head">
          <h3>Installed Links</h3>
          <span>{filteredLinks.length}</span>
        </div>

        {#if errors.links}
          <p class="notice notice--error">{errors.links}</p>
        {/if}

        {#if filteredLinks.length}
          <ul class="stack-list stack-list--links">
            {#each filteredLinks as item (`${item.target_path}:${item.updated_at}`)}
              <li class="stack-row stack-row--neutral">
                <button class="stack-row__button" type="button" onclick={() => applyLinkSelection(item)}>
                  <span class="stack-row__topline">
                    <strong>{item.artifact_id}</strong>
                    <span>{item.profile}</span>
                    <span class="pill pill--neutral">{item.method}</span>
                  </span>
                  <span class="stack-row__path">{item.target_path}</span>
                  <span class="stack-row__meta">
                    更新于 {formatRelativeTime(item.updated_at)}
                    {#if item.rendered_path}
                      · cache {truncateMiddle(item.rendered_path, 14)}
                    {/if}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty-state empty-state--compact">
            <p>当前筛选下没有安装记录。</p>
          </div>
        {/if}
      </div>
    </section>
  </main>
</div>

<style>
  .app-shell {
    position: relative;
    padding: clamp(1.25rem, 3vw, 2.75rem);
    max-width: 1600px;
    margin: 0 auto;
  }

  .surface {
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.86),
      rgba(255, 255, 255, 0.74)
    );
    border: 1px solid color-mix(in oklch, var(--line) 70%, white);
    border-radius: 32px;
    box-shadow: var(--shadow);
    backdrop-filter: blur(18px);
  }

  .hero,
  .panel {
    animation: rise 500ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .hero {
    display: grid;
    gap: 1.5rem;
    grid-template-columns: minmax(0, 1.1fr) minmax(18rem, 24rem) minmax(16rem, 23rem);
    padding: clamp(1.5rem, 3vw, 2.5rem);
    margin-bottom: 1.5rem;
  }

  .hero__copy {
    display: grid;
    gap: 0.75rem;
  }

  .eyebrow,
  .panel__eyebrow {
    margin: 0;
    font-size: 0.78rem;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  h1,
  h2,
  h3 {
    margin: 0;
    font-family: 'Manrope', sans-serif;
    line-height: 1.02;
    letter-spacing: -0.03em;
    color: var(--ink);
  }

  h1 {
    font-size: clamp(2.5rem, 6vw, 4.8rem);
  }

  h2 {
    font-size: clamp(1.35rem, 2vw, 1.8rem);
  }

  h3 {
    font-size: 1rem;
  }

  p {
    margin: 0;
  }

  .hero__lead {
    max-width: 44rem;
    color: var(--ink-soft);
    font-size: 1rem;
  }

  .hero__stats {
    display: grid;
    gap: 0.9rem;
    align-content: start;
  }

  .stat {
    padding: 1rem 1.1rem;
    border-radius: 24px;
    background: color-mix(in oklch, white 72%, var(--canvas-tint));
    border: 1px solid color-mix(in oklch, var(--line) 85%, white);
  }

  .stat span {
    display: block;
    color: var(--ink-soft);
    font-size: 0.8rem;
    margin-bottom: 0.4rem;
  }

  .stat strong {
    display: block;
    font-size: 1.15rem;
  }

  .stat--warn {
    background: color-mix(in oklch, var(--accent) 12%, white);
  }

  .hero__controls {
    display: grid;
    gap: 1rem;
    align-content: start;
  }

  .filter-field {
    display: grid;
    gap: 0.6rem;
  }

  .filter-field label {
    font-size: 0.86rem;
    color: var(--ink-soft);
  }

  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .watch-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem 0.9rem;
    font-size: 0.84rem;
    color: var(--ink-soft);
    align-items: center;
  }

  .status-line {
    min-height: 1.5rem;
    font-size: 0.92rem;
    color: var(--ink-soft);
  }


  .workspace {
    display: grid;
    gap: 1.5rem;
    grid-template-columns: minmax(18rem, 24rem) minmax(0, 1.2fr) minmax(20rem, 26rem);
  }

  .panel {
    display: grid;
    gap: 1.2rem;
    padding: 1.35rem;
    min-width: 0;
  }

  .panel__head,
  .list-block__head,
  .preview-toolbar,
  .manifest-footnote,
  .status-summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .panel__head {
    min-height: 2.5rem;
  }

  .workspace-path,
  .preview-subtitle {
    font-size: 0.95rem;
    color: var(--ink-soft);
  }

  .token-group {
    display: grid;
    gap: 0.85rem;
  }

  .token-group__title {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: 0.88rem;
    color: var(--ink-soft);
  }

  .manifest-footnote,
  .status-summary {
    padding-top: 0.2rem;
    border-top: 1px solid color-mix(in oklch, var(--line) 78%, white);
  }

  .manifest-footnote div,
  .status-summary div {
    display: grid;
    gap: 0.22rem;
  }

  .manifest-footnote span,
  .status-summary span {
    color: var(--ink-soft);
    font-size: 0.78rem;
  }

  .preview-toolbar {
    justify-content: flex-end;
  }

  .preview-output {
    margin: 0;
    padding: 1.25rem;
    min-height: 24rem;
    border-radius: 24px;
    border: 1px solid color-mix(in oklch, var(--line) 72%, white);
    background:
      linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.88),
        rgba(255, 255, 255, 0.76)
      );
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.95rem;
    line-height: 1.6;
    color: var(--ink);
  }

  .notice {
    padding: 0.9rem 1rem;
    border-radius: 20px;
    font-size: 0.92rem;
  }

  .notice--error {
    border: 1px solid color-mix(in oklch, var(--danger) 35%, white);
    background: color-mix(in oklch, var(--danger) 9%, white);
    color: color-mix(in oklch, var(--danger) 70%, black);
  }

  .list-block {
    display: grid;
    gap: 0.9rem;
    min-height: 0;
  }

  .list-block__head span {
    color: var(--ink-soft);
    font-size: 0.84rem;
  }

  .stack-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.8rem;
    min-width: 0;
  }

  .stack-list--links {
    max-height: 24rem;
    overflow: auto;
    padding-right: 0.2rem;
  }

  .stack-row {
    border-radius: 24px;
    border: 1px solid color-mix(in oklch, var(--line) 72%, white);
    background: color-mix(in oklch, white 76%, var(--canvas-tint));
    overflow: hidden;
  }

  .stack-row--ok {
    border-color: color-mix(in oklch, var(--success) 36%, white);
  }

  .stack-row--warn {
    border-color: color-mix(in oklch, var(--accent) 45%, white);
  }

  .stack-row__button {
    width: 100%;
    border: 0;
    background: transparent;
    padding: 1rem 1.05rem;
    text-align: left;
    color: inherit;
    display: grid;
    gap: 0.48rem;
    cursor: pointer;
  }

  .stack-row__button:hover {
    background: rgba(255, 255, 255, 0.38);
  }

  .stack-row__button:focus-visible {
    outline: 3px solid color-mix(in oklch, var(--primary) 58%, white);
    outline-offset: -3px;
  }

  .stack-row__topline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.75rem;
    min-width: 0;
  }

  .stack-row__topline strong {
    font-family: 'Manrope', sans-serif;
    font-size: 0.98rem;
  }

  .stack-row__path,
  .stack-row__meta {
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .stack-row__path {
    color: var(--ink);
    font-size: 0.92rem;
  }

  .stack-row__meta {
    color: var(--ink-soft);
    font-size: 0.8rem;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 1.7rem;
    padding: 0.2rem 0.72rem;
    border-radius: 999px;
    font-size: 0.74rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .pill--ok {
    background: color-mix(in oklch, var(--success) 18%, white);
    color: color-mix(in oklch, var(--success) 70%, black);
  }

  .pill--warn {
    background: color-mix(in oklch, var(--accent) 24%, white);
    color: color-mix(in oklch, var(--accent) 64%, black);
  }

  .pill--neutral {
    background: color-mix(in oklch, var(--line) 28%, white);
    color: var(--ink-soft);
  }

  .empty-state {
    display: grid;
    place-items: center;
    min-height: 9rem;
    padding: 1.1rem;
    text-align: center;
    border-radius: 24px;
    background: color-mix(in oklch, white 70%, var(--canvas-tint));
    border: 1px dashed color-mix(in oklch, var(--line) 80%, white);
    color: var(--ink-soft);
  }

  .empty-state--preview {
    min-height: 24rem;
  }

  .empty-state--compact {
    min-height: 6rem;
  }

  :global(md-chip-set) {
    display: flex;
    flex-wrap: wrap;
    gap: 0.68rem;
  }

  :global(md-outlined-text-field) {
    width: 100%;
  }

  :global(md-divider) {
    --md-divider-color: color-mix(in oklch, var(--line) 78%, white);
  }

  @media (max-width: 1180px) {
    .hero,
    .workspace {
      grid-template-columns: 1fr;
    }

    .hero__stats {
      grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
    }

    .panel--status {
      order: 3;
    }
  }

  @media (max-width: 720px) {
    .app-shell {
      padding: 1rem;
    }

    .surface {
      border-radius: 26px;
    }

    .hero,
    .panel {
      padding: 1rem;
    }

    .action-row,
    .panel__head,
    .list-block__head,
    .preview-toolbar,
    .manifest-footnote,
    .status-summary {
      align-items: flex-start;
      flex-direction: column;
    }

    .preview-output {
      min-height: 18rem;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }

    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
