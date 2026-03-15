<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { Tabs } from 'bits-ui';

  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import OutputViewer from '$lib/components/OutputViewer.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import {
    ApiClientError,
    getArtifactGitCompare,
    getArtifactGitHistory,
    getArtifactSource,
    getProfileDetail,
    getManifestSource,
    renderArtifact,
    rollbackArtifactToRevision,
    updateArtifactSource,
    updateManifestSource
  } from '$lib/api/client';
  import type {
    ArtifactGitCompareResponse,
    ArtifactGitHistoryResponse,
    ArtifactSourceResponse,
    ManifestSourceResponse,
    ProfileDetailResponse,
    ValidateAsResponse,
    WorkspaceSummaryResponse
  } from '$lib/types';
  import { basenameFromPath, truncateMiddle } from '$lib/utils/format';
  import { buildArtifactTree, relativeSourcePath } from '$lib/workbench/artifacts/artifact_tree';
  import ArtifactTreeNode from '$lib/workbench/artifacts/ArtifactTreeNode.svelte';
  import DiffViewer from '$lib/workbench/DiffViewer.svelte';
  import EditorTabs from '$lib/workbench/EditorTabs.svelte';
  import {
    buildManifestSnippet,
    manifestInsertLabel,
    type ManifestInsertKind
  } from '$lib/workbench/manifest_snippets';

  type Props = {
    summary: WorkspaceSummaryResponse | null;
    selectedProfile: string | null;
    onSelectProfile: (id: string) => void;
    onFocusArtifact?: (id: string | null) => void;
    onOpenTarget?: (id: string) => void;
    onRefreshWorkspace?: () => Promise<void>;
    onSourceSaved?: () => Promise<void>;
    requestedArtifactId?: string | null;
    onRequestHandled?: (id: string) => void;
    requestedManifestInsert?: ManifestInsertKind | null;
    onManifestInsertHandled?: (kind: ManifestInsertKind) => void;
    shortcutsEnabled?: boolean;
    statusLine: string;
    errorMessage: string | null;
    setStatusLine: (next: string) => void;
    setErrorMessage: (next: string | null) => void;
  };

  let {
    summary,
    selectedProfile,
    onSelectProfile,
    onFocusArtifact,
    onOpenTarget,
    onRefreshWorkspace,
    onSourceSaved,
    requestedArtifactId,
    onRequestHandled,
    requestedManifestInsert,
    onManifestInsertHandled,
    shortcutsEnabled,
    statusLine,
    errorMessage,
    setStatusLine,
    setErrorMessage
  }: Props = $props();

  type EditorState = {
    source: ArtifactSourceResponse | ManifestSourceResponse | null;
    editorText: string;
    previewText: string;
    dirty: boolean;
    busySource: boolean;
    busySave: boolean;
    busyPreview: boolean;
  };

  const MANIFEST_DOC_ID = '$manifest';

  let openTabs = $state<string[]>([]);
  let activeTab = $state<string | null>(null);
  let editors = $state<Record<string, EditorState>>({});
  let rightMode = $state<'preview' | 'diff'>('preview');
  let autoOpened = $state(false);
  let artifactSearch = $state('');
  let tabMenu = $state<{ id: string; x: number; y: number } | null>(null);
  let tabMenuEl = $state<HTMLDivElement | null>(null);
  let lastAutoPreviewKey: string | null = null;
  let profileDetail = $state<ProfileDetailResponse | null>(null);
  let profileDetailBusy = $state(false);
  let profileDetailError = $state<string | null>(null);
  let profileRequestToken = 0;
  let gitHistory = $state<ArtifactGitHistoryResponse | null>(null);
  let gitHistoryBusy = $state(false);
  let gitHistoryError = $state<string | null>(null);
  let gitCompare = $state<ArtifactGitCompareResponse | null>(null);
  let gitCompareBusy = $state(false);
  let gitCompareError = $state<string | null>(null);
  let selectedGitRevision = $state<string | null>(null);
  let gitHistoryToken = 0;
  let gitCompareToken = 0;
  let rollbackBusy = $state(false);

  const fileArtifacts = $derived((summary?.artifacts ?? []).filter((a) => a.kind === 'file'));
  const dirArtifacts = $derived((summary?.artifacts ?? []).filter((a) => a.kind === 'dir'));
  const workspaceRoot = $derived(summary?.workspace_root ?? null);
  const profiles = $derived(summary?.profiles ?? []);
  const profileIds = $derived(profiles.map((p) => p.id));
  const usageTargets = $derived.by(() => {
    if (!summary || !activeTab || activeTab === MANIFEST_DOC_ID) {
      return [];
    }
    return summary.targets.filter((t) => t.artifact_id === activeTab);
  });

  const filteredFileArtifacts = $derived.by(() => {
    const q = artifactSearch.trim().toLowerCase();
    if (!q) {
      return fileArtifacts;
    }

    return fileArtifacts.filter((artifact) => {
      const rel = relativeSourcePath(artifact.source_path, workspaceRoot).toLowerCase();
      return artifact.id.toLowerCase().includes(q) || rel.includes(q);
    });
  });

  const artifactTree = $derived.by(() => buildArtifactTree(filteredFileArtifacts, workspaceRoot));

  const activeEditor = $derived.by(() => {
    if (!activeTab) {
      return null;
    }
    return editors[activeTab] ?? null;
  });

  const tabsModel = $derived.by(() =>
    openTabs.map((id) => {
      const label = id === MANIFEST_DOC_ID ? 'agentstow.toml' : id;
      const dirty = editors[id]?.dirty ?? false;
      return { id, label, dirty };
    })
  );

  function isManifestTab(id: string | null): boolean {
    return id === MANIFEST_DOC_ID;
  }

  function sourcePathOf(
    source: ArtifactSourceResponse | ManifestSourceResponse | null | undefined
  ): string | null {
    return source?.source_path ?? null;
  }

  function savedContentOf(
    source: ArtifactSourceResponse | ManifestSourceResponse | null | undefined
  ): string {
    return source?.content ?? '';
  }

  function summarySourcePathOf(documentId: string | null): string | null {
    if (!documentId || documentId === MANIFEST_DOC_ID) {
      return workspaceRoot ? `${workspaceRoot}/agentstow.toml` : MANIFEST_DOC_ID;
    }

    return fileArtifacts.find((artifact) => artifact.id === documentId)?.source_path ?? null;
  }

  function validateAsOf(
    tabId: string | null,
    source: ArtifactSourceResponse | ManifestSourceResponse | null | undefined
  ): ValidateAsResponse {
    if (tabId === MANIFEST_DOC_ID) {
      return 'toml';
    }
    return source && 'validate_as' in source ? source.validate_as : 'none';
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

  function openDocument(id: string): void {
    if (!openTabs.includes(id)) {
      openTabs = [...openTabs, id];
    }
    activeTab = id;
    rightMode = 'preview';
    setStatusLine(id === MANIFEST_DOC_ID ? '已打开 workspace manifest。' : `已打开 artifact：${id}`);
  }

  function closeActiveArtifact(): void {
    if (!activeTab) {
      return;
    }
    const idx = openTabs.indexOf(activeTab);
    if (idx === -1) {
      activeTab = null;
      return;
    }
    const nextTabs = openTabs.filter((id) => id !== activeTab);
    openTabs = nextTabs;
    const nextActive = nextTabs[idx] ?? nextTabs[idx - 1] ?? null;
    activeTab = nextActive;
    setStatusLine(nextActive ? `切换到 ${nextActive}` : '已关闭所有 artifact。');
  }

  function closeArtifactTab(id: string): void {
    const idx = openTabs.indexOf(id);
    if (idx === -1) {
      return;
    }
    const nextTabs = openTabs.filter((t) => t !== id);
    openTabs = nextTabs;

    if (activeTab === id) {
      activeTab = nextTabs[idx] ?? nextTabs[idx - 1] ?? null;
    }
  }

  function closeOtherTabs(keepId: string): void {
    if (!openTabs.includes(keepId)) {
      return;
    }
    openTabs = [keepId];
    activeTab = keepId;
  }

  function reorderTabs(nextOrder: string[]): void {
    // Keep only currently opened tabs and preserve uniqueness.
    const allowed = new Set(openTabs);
    const next: string[] = [];
    for (const id of nextOrder) {
      if (allowed.has(id) && !next.includes(id)) {
        next.push(id);
      }
    }
    for (const id of openTabs) {
      if (!next.includes(id)) {
        next.push(id);
      }
    }
    openTabs = next;
  }

  async function copyToClipboard(text: string, label: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      setStatusLine(`已复制${label}到剪贴板。`);
    } catch {
      setStatusLine(`复制${label}失败（浏览器未授权）。`);
    }
  }

  function openTabMenu(id: string, x: number, y: number): void {
    const pad = 12;
    const w = 240;
    const h = 210;
    const clampedX = Math.min(x, window.innerWidth - w - pad);
    const clampedY = Math.min(y, window.innerHeight - h - pad);
    tabMenu = { id, x: Math.max(pad, clampedX), y: Math.max(pad, clampedY) };
  }

  function closeTabMenu(): void {
    tabMenu = null;
  }

  function describeDirArtifact(artifactId: string): void {
    setStatusLine(`dir artifact（${artifactId}）当前仅展示，不支持 source 编辑。`);
  }

  async function ensureEditorState(documentId: string): Promise<EditorState | null> {
    openDocument(documentId);
    await loadEditorSource(documentId);
    return editors[documentId] ?? null;
  }

  async function insertManifestSnippet(kind: ManifestInsertKind): Promise<void> {
    const st = await ensureEditorState(MANIFEST_DOC_ID);
    if (!st) {
      return;
    }

    const snippet = buildManifestSnippet(kind, summary);
    const base = st.editorText.trimEnd();
    st.editorText = `${base}${base ? '\n\n' : ''}${snippet}`;
    st.dirty = savedContentOf(st.source) !== st.editorText;
    activeTab = MANIFEST_DOC_ID;
    setErrorMessage(null);
    setStatusLine(`已插入 ${manifestInsertLabel(kind)} 模板，保存后生效。`);
  }

  async function loadEditorSource(documentId: string): Promise<void> {
    editors[documentId] ??= {
      source: null,
      editorText: '',
      previewText: '',
      dirty: false,
      busySource: false,
      busySave: false,
      busyPreview: false
    };

    const st = editors[documentId]!;
    if (st.busySource || st.source) {
      return;
    }

    st.busySource = true;
    setErrorMessage(null);
    try {
      const source =
        documentId === MANIFEST_DOC_ID
          ? await getManifestSource()
          : await getArtifactSource(documentId);
      st.source = source;
      st.editorText = source.content;
      st.dirty = false;
    } catch (error) {
      st.source = null;
      st.editorText = '';
      st.dirty = false;
      setErrorMessage(
        describeError(error, documentId === MANIFEST_DOC_ID ? '无法读取 manifest。' : '无法读取 artifact source。')
      );
    } finally {
      st.busySource = false;
    }
  }

  async function saveActiveArtifact(): Promise<void> {
    if (!activeTab) {
      return;
    }
    const st = editors[activeTab];
    if (!st) {
      return;
    }
    if (!st.dirty || st.busySave) {
      return;
    }

    st.busySave = true;
    setErrorMessage(null);
    try {
      const updated =
        activeTab === MANIFEST_DOC_ID
          ? await updateManifestSource({ content: st.editorText })
          : await updateArtifactSource(activeTab, st.editorText);
      st.source = updated;
      st.dirty = false;
      setStatusLine(activeTab === MANIFEST_DOC_ID ? '已保存 manifest。' : '已保存 source。');
      if (activeTab === MANIFEST_DOC_ID) {
        await onRefreshWorkspace?.();
      } else {
        await onSourceSaved?.();
      }
    } catch (error) {
      setErrorMessage(describeError(error, '保存失败。'));
    } finally {
      st.busySave = false;
    }
  }

  async function refreshPreview(): Promise<void> {
    if (!activeTab) {
      return;
    }
    const st = editors[activeTab];
    if (!st || st.busyPreview) {
      return;
    }

    st.busyPreview = true;
    setErrorMessage(null);
    try {
      if (activeTab === MANIFEST_DOC_ID) {
        st.previewText =
          'Manifest editor\n\n在这里直接编辑 workspace 的 profiles / artifacts / targets / env_sets / scripts / mcp_servers。\n保存后刷新左侧资源树，即可看到新增或变更的对象。';
        setStatusLine('已刷新 manifest 说明面板。');
      } else if (selectedProfile) {
        const resp = await renderArtifact(activeTab, selectedProfile);
        st.previewText = resp.text;
        setStatusLine('已刷新渲染预览。');
      } else {
        st.previewText = '请选择 profile 后再渲染预览。';
      }
    } catch (error) {
      st.previewText = '';
      setErrorMessage(describeError(error, '渲染预览失败。'));
    } finally {
      st.busyPreview = false;
    }
  }

  async function loadProfileDetail(profileId: string | null): Promise<void> {
    const token = ++profileRequestToken;

    if (!profileId) {
      profileDetail = null;
      profileDetailError = null;
      profileDetailBusy = false;
      return;
    }

    profileDetailBusy = true;
    profileDetailError = null;

    try {
      const detail = await getProfileDetail(profileId);
      if (token !== profileRequestToken) {
        return;
      }
      profileDetail = detail;
    } catch (error) {
      if (token !== profileRequestToken) {
        return;
      }
      profileDetail = null;
      profileDetailError = describeError(error, '无法读取 profile 变量。');
    } finally {
      if (token === profileRequestToken) {
        profileDetailBusy = false;
      }
    }
  }

  async function loadGitHistory(documentId: string | null): Promise<void> {
    const token = ++gitHistoryToken;

    if (!documentId || isManifestTab(documentId)) {
      gitHistory = null;
      gitHistoryError = null;
      gitHistoryBusy = false;
      gitCompare = null;
      gitCompareError = null;
      selectedGitRevision = null;
      return;
    }

    gitHistoryBusy = true;
    gitHistoryError = null;
    gitCompare = null;
    gitCompareError = null;
    selectedGitRevision = null;

    try {
      const history = await getArtifactGitHistory(documentId, 24);
      if (token !== gitHistoryToken || activeTab !== documentId) {
        return;
      }
      gitHistory = history;
    } catch (error) {
      if (token !== gitHistoryToken || activeTab !== documentId) {
        return;
      }
      gitHistory = null;
      gitHistoryError = describeError(error, '无法读取 Git history。');
    } finally {
      if (token === gitHistoryToken && activeTab === documentId) {
        gitHistoryBusy = false;
      }
    }
  }

  async function compareRevision(revision: string): Promise<void> {
    const documentId = activeTab;
    if (!documentId || isManifestTab(documentId) || gitCompareBusy) {
      return;
    }

    const token = ++gitCompareToken;
    gitCompareBusy = true;
    gitCompareError = null;

    try {
      const compare = await getArtifactGitCompare({
        artifact: documentId,
        base: revision,
        head: 'WORKTREE'
      });
      if (token !== gitCompareToken || activeTab !== documentId) {
        return;
      }
      gitCompare = compare;
      selectedGitRevision = revision;
      rightMode = 'diff';
      setStatusLine(`已加载 ${compare.base_label} -> ${compare.head_label} 的 Git 对比。`);
    } catch (error) {
      if (token !== gitCompareToken || activeTab !== documentId) {
        return;
      }
      gitCompare = null;
      gitCompareError = describeError(error, '无法读取 Git compare。');
    } finally {
      if (token === gitCompareToken && activeTab === documentId) {
        gitCompareBusy = false;
      }
    }
  }

  function clearGitCompare(): void {
    gitCompare = null;
    gitCompareError = null;
    selectedGitRevision = null;
    setStatusLine('已返回编辑差异视图。');
  }

  async function rollbackRevision(revision: string): Promise<void> {
    const documentId = activeTab;
    if (!documentId || isManifestTab(documentId) || rollbackBusy) {
      return;
    }

    const confirmed =
      typeof window === 'undefined'
        ? true
        : window.confirm(`将 ${documentId} 的 source 回退到 ${revision.slice(0, 7)}？这会覆盖当前已保存文件。`);
    if (!confirmed) {
      return;
    }

    rollbackBusy = true;
    selectedGitRevision = revision;
    setErrorMessage(null);
    try {
      const updated = await rollbackArtifactToRevision(documentId, { revision });
      const st = editors[documentId];
      if (st) {
        st.source = updated.source;
        st.editorText = updated.source.content;
        st.previewText = '';
        st.dirty = false;
      }
      gitCompare = null;
      gitCompareError = null;
      await loadGitHistory(documentId);
      selectedGitRevision = updated.commit.revision;
      if (activeTab === documentId) {
        await refreshPreview();
      }
      await onSourceSaved?.();
      setStatusLine(`已将 ${documentId} 回退到 ${updated.commit.short_revision} · ${updated.commit.summary}。`);
    } catch (error) {
      setErrorMessage(describeError(error, 'Git 回退失败。'));
    } finally {
      rollbackBusy = false;
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (shortcutsEnabled === false) {
      return;
    }
    const isMod = event.metaKey || event.ctrlKey;
    if (!isMod) {
      return;
    }

    if (event.key.toLowerCase() === 's') {
      event.preventDefault();
      void saveActiveArtifact();
      return;
    }

    if (event.key.toLowerCase() === 'p') {
      event.preventDefault();
      void refreshPreview();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', onKeyDown);
  });

  $effect(() => {
    if (!tabMenu) {
      return;
    }

    const onPointerDown = (event: PointerEvent) => {
      if (!tabMenuEl) {
        return;
      }
      const target = event.target;
      if (target instanceof Node && tabMenuEl.contains(target)) {
        return;
      }
      closeTabMenu();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeTabMenu();
      }
    };

    window.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('keydown', onKey);
    };
  });

  $effect(() => {
    if (autoOpened) {
      return;
    }
    if (fileArtifacts.length) {
      openDocument(fileArtifacts[0].id);
      autoOpened = true;
      return;
    }
    openDocument(MANIFEST_DOC_ID);
    autoOpened = true;
  });

  $effect(() => {
    if (!activeTab) {
      return;
    }
    void loadEditorSource(activeTab);
  });

  $effect(() => {
    if (!summary) {
      return;
    }

    const validIds = new Set(fileArtifacts.map((artifact) => artifact.id));
    const nextTabs = openTabs.filter((id) => id === MANIFEST_DOC_ID || validIds.has(id));
    if (nextTabs.length !== openTabs.length) {
      openTabs = nextTabs.length > 0 ? nextTabs : [MANIFEST_DOC_ID];
    }

    if (activeTab && activeTab !== MANIFEST_DOC_ID && !validIds.has(activeTab)) {
      activeTab = nextTabs[0] ?? MANIFEST_DOC_ID;
    }

    let needsReload = false;
    for (const documentId of nextTabs) {
      const st = editors[documentId];
      if (!st || st.dirty) {
        continue;
      }

      const expectedPath = summarySourcePathOf(documentId);
      const currentPath = sourcePathOf(st.source);
      if (expectedPath && currentPath && expectedPath !== currentPath) {
        st.source = null;
        st.editorText = '';
        st.previewText = '';
        st.dirty = false;
        if (activeTab === documentId) {
          needsReload = true;
        }
      }
    }

    if (needsReload && activeTab) {
      void loadEditorSource(activeTab);
    }
  });

  $effect(() => {
    void loadProfileDetail(selectedProfile ?? null);
  });

  $effect(() => {
    void loadGitHistory(activeTab);
  });

  $effect(() => {
    const req = requestedArtifactId ?? null;
    if (!req) {
      return;
    }
    if (activeTab === req) {
      onRequestHandled?.(req);
      return;
    }
    if (summary && !fileArtifacts.some((a) => a.id === req)) {
      setErrorMessage(`artifact 不存在：${req}`);
      onRequestHandled?.(req);
      return;
    }
    openDocument(req);
    onRequestHandled?.(req);
  });

  $effect(() => {
    onFocusArtifact?.(isManifestTab(activeTab) ? null : activeTab);
  });

  $effect(() => {
    const tabId = activeTab;
    if (!tabId) {
      lastAutoPreviewKey = null;
      return;
    }
    if (!isManifestTab(tabId) && !selectedProfile) {
      return;
    }

    const source = editors[tabId]?.source ?? null;
    if (!source) {
      return;
    }

    const nextKey = `${tabId}::${selectedProfile ?? ''}::${savedContentOf(source)}`;
    if (nextKey === lastAutoPreviewKey) {
      return;
    }

    lastAutoPreviewKey = nextKey;
    void refreshPreview();
  });

  $effect(() => {
    const kind = requestedManifestInsert ?? null;
    if (!kind) {
      return;
    }

    void insertManifestSnippet(kind);
    onManifestInsertHandled?.(kind);
  });

  const titleLabel = $derived.by(() => {
    if (!activeTab) {
      return '未选择 artifact';
    }
    if (activeTab === MANIFEST_DOC_ID) {
      return 'Workspace Manifest · agentstow.toml';
    }
    const st = editors[activeTab];
    const sourcePath = sourcePathOf(st?.source);
    const file = sourcePath ? basenameFromPath(sourcePath) : null;
    return file ? `${activeTab} · ${file}` : activeTab;
  });

  const activeValidateAs = $derived(validateAsOf(activeTab, activeEditor?.source));
  const previewMode = $derived(activeValidateAs === 'markdown' ? 'markdown' : 'plain');
  const diffOriginalText = $derived.by(() =>
    gitCompare ? gitCompare.base_content : savedContentOf(activeEditor?.source)
  );
  const diffModifiedText = $derived.by(() =>
    gitCompare ? gitCompare.head_content : activeEditor?.editorText ?? ''
  );
  const diffFromLabel = $derived.by(() =>
    gitCompare ? gitCompare.base_label : activeTab ? `${activeTab} (saved)` : 'saved'
  );
  const diffToLabel = $derived.by(() =>
    gitCompare ? gitCompare.head_label : activeTab ? `${activeTab} (edited)` : 'edited'
  );
  const activeArtifactSummary = $derived.by(() => {
    if (!activeTab || activeTab === MANIFEST_DOC_ID) {
      return null;
    }
    return fileArtifacts.find((artifact) => artifact.id === activeTab) ?? null;
  });
  const activeSourcePath = $derived(sourcePathOf(activeEditor?.source));
</script>

<SplitView autoSaveId="workbench:view:artifacts" initialLeftPct={22} minLeftPx={264} minRightPx={780}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">ARTIFACTS</p>
        <p class="explorer__hint">用左侧 source tree 打开文档，右侧按 editor/preview/inspector 协作。</p>
      </div>

      <div class="explorer__section">
        <div class="section__title">
          <span>Workspace Config</span>
          <strong>1</strong>
        </div>
        <div class="chips chips--tight" aria-label="Manifest quick create">
          <button class="chip" onclick={() => void insertManifestSnippet('profile')} type="button">
            新建 profile
          </button>
          <button class="chip" onclick={() => void insertManifestSnippet('artifact')} type="button">
            新建 artifact
          </button>
          <button class="chip" onclick={() => void insertManifestSnippet('target')} type="button">
            新建 target
          </button>
          <button class="chip" onclick={() => void insertManifestSnippet('env_set')} type="button">
            新建 env
          </button>
          <button class="chip" onclick={() => void insertManifestSnippet('script')} type="button">
            新建 script
          </button>
          <button class="chip" onclick={() => void insertManifestSnippet('mcp_server')} type="button">
            新建 MCP
          </button>
        </div>
        <button
          class={['list__item', activeTab === MANIFEST_DOC_ID ? 'list__item--active' : ''].join(' ')}
          onclick={() => openDocument(MANIFEST_DOC_ID)}
          type="button"
          title="agentstow.toml"
        >
          <span class="list__dot list__dot--accent" aria-hidden="true"></span>
          <span class="list__name">agentstow.toml</span>
          <span class="list__meta">workspace</span>
        </button>
      </div>

      <div class="explorer__section">
        <div class="section__title">
          <span>Sources</span>
          <strong>{filteredFileArtifacts.length}</strong>
        </div>

        <label class="field field--compact">
          <span class="field__label">搜索 artifacts</span>
          <input
            class="field__input mono"
            type="search"
            placeholder="id 或 source path…"
            value={artifactSearch}
            oninput={(event) => {
              const target = event.currentTarget as HTMLInputElement | null;
              artifactSearch = target?.value ?? '';
            }}
          />
        </label>

        {#if !summary}
          <div class="list__static">
            <span class="muted">读取中…</span>
            <span class="mono">/api/workspace-summary</span>
          </div>
        {:else if fileArtifacts.length === 0}
          <div class="list__static">
            <span class="muted">（暂无 file artifacts）</span>
            <span class="mono">artifacts</span>
          </div>
        {:else if filteredFileArtifacts.length === 0}
          <div class="list__static">
            <span class="muted">（无匹配结果）</span>
            <span class="mono">{artifactSearch.trim() || 'query'}</span>
          </div>
        {:else}
          <ul class="tree" aria-label="Artifacts explorer tree">
            {#each artifactTree as node (node.path)}
              <ArtifactTreeNode
                node={node}
                depth={0}
                activeArtifactId={activeTab}
                searchActive={artifactSearch.trim().length > 0}
                onOpenArtifact={openDocument}
              />
            {/each}
          </ul>
        {/if}
      </div>

      {#if dirArtifacts.length > 0}
        <div class="explorer__section">
          <div class="section__title">
            <span>Dir Artifacts</span>
            <strong>{dirArtifacts.length}</strong>
          </div>

          <ul class="list">
            {#each dirArtifacts as artifact (artifact.id)}
              <li>
                <button
                  class="list__item"
                  onclick={() => describeDirArtifact(artifact.id)}
                  type="button"
                  title={artifact.source_path}
                >
                  <span class="list__dot list__dot--accent" aria-hidden="true"></span>
                  <span class="list__name">{artifact.id}</span>
                  <span class="list__meta">
                    {basenameFromPath(relativeSourcePath(artifact.source_path, workspaceRoot))}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

  <div class="explorer__section">
    <div class="section__title">
      <span>Profiles</span>
      <strong>{profileIds.length}</strong>
        </div>
        <div class="chips">
          {#each profileIds as profileId (profileId)}
            <button
              class={['chip', selectedProfile === profileId ? 'chip--active' : ''].join(' ')}
              onclick={() => onSelectProfile(profileId)}
              type="button"
            >
              {profileId}
            </button>
      {/each}
    </div>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Profile Vars</span>
      <strong>{profileDetail?.merged_vars.length ?? 0}</strong>
    </div>

    {#if !selectedProfile}
      <div class="list__static">
        <span class="muted">（选择 profile 后查看变量与占位）</span>
        <span class="mono">vars</span>
      </div>
    {:else if profileDetailBusy}
      <div class="list__static">
        <span class="muted">读取中…</span>
        <span class="mono">{selectedProfile}</span>
      </div>
    {:else if profileDetailError}
      <p class="empty empty--flush">{profileDetailError}</p>
    {:else if !profileDetail || profileDetail.merged_vars.length === 0}
      <div class="list__static">
        <span class="muted">（该 profile 暂无变量）</span>
        <span class="mono">{selectedProfile}</span>
      </div>
    {:else}
      <div class="token-list token-list--stack">
        {#each profileDetail.merged_vars as variable (variable.key)}
          <button
            class="token token--interactive"
            type="button"
            title={variable.value_json}
            onclick={() => void copyToClipboard(`{{ ${variable.key} }}`, 'Tera 占位符')}
          >
            <span class="token__label">{variable.key}</span>
            <span class="token__meta mono">{`{{ ${variable.key} }}`}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</aside>
  {/snippet}

  {#snippet right()}
    <main class="canvas" aria-label="工作区画布">
      <div class="canvas__head">
        <div class="title">
          <strong>{titleLabel}</strong>
          <span class="muted">
            {activeTab === MANIFEST_DOC_ID ? '· workspace config' : selectedProfile ? `· ${selectedProfile}` : ''}
          </span>
        </div>

        <div class="canvas__actions">
          <button
            class="ui-button ui-button--ghost"
            disabled={!activeEditor?.dirty || activeEditor?.busySave}
            type="button"
            onclick={() => void saveActiveArtifact()}
          >
            {activeEditor?.busySave ? '保存中…' : activeEditor?.dirty ? '保存' : '已保存'}
          </button>
          <button
            class="ui-button ui-button--primary"
            disabled={!activeTab || (!isManifestTab(activeTab) && !selectedProfile) || activeEditor?.busyPreview}
            type="button"
            onclick={() => void refreshPreview()}
          >
            {activeEditor?.busyPreview ? '处理中…' : activeTab === MANIFEST_DOC_ID ? '说明 / 校验' : '渲染预览'}
          </button>
          <button
            class="ui-button ui-button--subtle"
            disabled={!activeTab}
            type="button"
            onclick={closeActiveArtifact}
          >
            关闭
          </button>
        </div>
      </div>

      {#if errorMessage}
        <p class="notice notice--error">{errorMessage}</p>
      {/if}
      <p class="status-line" aria-live="polite">{statusLine}</p>

      <div class="canvas__body">
        <SplitView
          autoSaveId="workbench:artifacts:inspector"
          initialLeftPct={72}
          minLeftPx={560}
          minRightPx={280}
        >
          {#snippet left()}
            <div class="split surface">
              <SplitView
                autoSaveId="workbench:artifacts:shell"
                initialLeftPct={52}
                minLeftPx={360}
                minRightPx={360}
              >
                {#snippet left()}
                  <div class="pane">
                    <div class="pane__title">
                      <div class="pane__title-row">
                        <span>Source</span>
                        <EditorTabs
                          tabs={tabsModel}
                          active={activeTab}
                          onChange={(next) => {
                            activeTab = next;
                          }}
                          onClose={(id) => closeArtifactTab(id)}
                          onReorder={(nextOrder) => reorderTabs(nextOrder)}
                          onOpenContextMenu={(id, x, y) => openTabMenu(id, x, y)}
                        />
                      </div>
                    </div>
                    <div class="pane__body">
                      {#if !activeTab}
                        <p class="muted">（请选择一个 artifact）</p>
                      {:else if activeEditor?.busySource}
                        <p class="muted">读取中…</p>
                      {:else}
                        {#key `${activeTab}:${activeSourcePath ?? 'unresolved'}`}
                          <CodeEditor
                            testId="artifact-source-editor"
                            value={activeEditor?.editorText ?? ''}
                            onChange={(next) => {
                              if (!activeTab) {
                                return;
                              }
                              const st = editors[activeTab];
                              if (!st) {
                                return;
                              }
                              st.editorText = next;
                              st.dirty = savedContentOf(st.source) !== next;
                            }}
                          />
                        {/key}
                      {/if}
                    </div>
                  </div>
                {/snippet}

                {#snippet right()}
                  <div class="pane">
                    <div class="pane__title">
                      <span>Preview</span>
                      {#if gitCompare}
                        <div class="chips chips--tight" aria-label="Git compare actions">
                          <span class="pill pill--warn mono">{gitCompare.base_revision.slice(0, 7)}</span>
                          <button class="chip" onclick={clearGitCompare} type="button">
                            返回编辑差异
                          </button>
                        </div>
                      {/if}
                    </div>
                    <div class="pane__body">
                      <Tabs.Root value={rightMode} onValueChange={(next) => (rightMode = next as typeof rightMode)}>
                        <Tabs.List class="tabs" aria-label="Preview mode">
                          <Tabs.Trigger class="tab" value="preview">Rendered</Tabs.Trigger>
                          <Tabs.Trigger class="tab" value="diff">Diff</Tabs.Trigger>
                        </Tabs.List>

                        <Tabs.Content class="tabs__panel" value="preview">
                          <OutputViewer text={activeEditor?.previewText || '（暂无预览）'} mode={previewMode} />
                        </Tabs.Content>

                        <Tabs.Content class="tabs__panel" value="diff">
                          {#if gitCompare && activeEditor?.dirty}
                            <p class="stack-note">当前 Git 对比基于已保存 worktree，未保存编辑不会反映在右侧内容中。</p>
                          {/if}
                          <DiffViewer
                            testId="artifact-diff-viewer"
                            original={diffOriginalText}
                            modified={diffModifiedText}
                            fromLabel={diffFromLabel}
                            toLabel={diffToLabel}
                          />
                        </Tabs.Content>
                      </Tabs.Root>
                    </div>
                  </div>
                {/snippet}
              </SplitView>
            </div>
          {/snippet}

          {#snippet right()}
            <section class="region secondary-sidebar" aria-label="Artifacts 检查器">
              <div class="region__header">
                <span>Inspector</span>
                <span class="mono">{activeTab === MANIFEST_DOC_ID ? 'manifest' : activeTab ?? 'idle'}</span>
              </div>

              <div class="region__body">
                {#if !activeTab}
                  <p class="empty empty--flush">（打开 artifact 或 manifest 后查看上下文与变量）</p>
                {:else if activeTab === MANIFEST_DOC_ID}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Workspace Summary</span>
                      <strong>{summary?.workspace_root ? 'ready' : 'idle'}</strong>
                    </div>
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Profiles</span>
                        <span class="inspector-row__value inspector-row__value--mono">{profiles.length}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Artifacts</span>
                        <span class="inspector-row__value inspector-row__value--mono">{summary?.artifacts.length ?? 0}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Targets</span>
                        <span class="inspector-row__value inspector-row__value--mono">{summary?.targets.length ?? 0}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Env Sets</span>
                        <span class="inspector-row__value inspector-row__value--mono">{summary?.env_sets.length ?? 0}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Scripts</span>
                        <span class="inspector-row__value inspector-row__value--mono">{summary?.scripts.length ?? 0}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">MCP</span>
                        <span class="inspector-row__value inspector-row__value--mono">{summary?.mcp_servers.length ?? 0}</span>
                      </div>
                    </div>
                  </div>
                {:else}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Document</span>
                      <strong>{activeArtifactSummary?.kind ?? 'file'}</strong>
                    </div>
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Artifact</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeTab}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Source</span>
                        <span
                          class="inspector-row__value inspector-row__value--mono"
                          data-testid="artifact-source-path"
                        >
                          {activeSourcePath ?? activeArtifactSummary?.source_path ?? '（未加载）'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Template</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeArtifactSummary?.template ? 'tera / jinja-like' : 'plain text'}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Validate</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeValidateAs}</span>
                      </div>
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Profile Context</span>
                      <strong>{selectedProfile ?? '未选择'}</strong>
                    </div>

                    {#if profileDetailBusy}
                      <p class="empty empty--flush">读取 profile 变量中…</p>
                    {:else if profileDetailError}
                      <p class="empty empty--flush">{profileDetailError}</p>
                    {:else if !selectedProfile}
                      <p class="empty empty--flush">（选择 profile 后显示渲染变量）</p>
                    {:else if !profileDetail || profileDetail.merged_vars.length === 0}
                      <p class="empty empty--flush">（当前 profile 没有 merged vars）</p>
                    {:else}
                      <div class="inspector-table">
                        {#each profileDetail.merged_vars as item (item.key)}
                          <div class="inspector-row">
                            <span class="inspector-row__label">{item.key}</span>
                            <span class="inspector-row__value inspector-row__value--mono">{item.value_json}</span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Used By Targets</span>
                      <strong>{usageTargets.length}</strong>
                    </div>

                    {#if usageTargets.length === 0}
                      <p class="empty empty--flush">（未被任何 target 引用）</p>
                    {:else}
                      <ul class="result-list">
                        {#each usageTargets as t (t.id)}
                          <li class="result-row">
                            <button
                              class="result-row__button"
                              type="button"
                              disabled={!onOpenTarget}
                              onclick={() => onOpenTarget?.(t.id)}
                              title={t.target_path}
                            >
                              <span class="pill pill--neutral">{t.method}</span>
                              <span class="result-row__title">{t.id}</span>
                              <span class="result-row__detail">{truncateMiddle(t.target_path, 64)}</span>
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Git History</span>
                      <strong>{gitHistory?.commits.length ?? 0}</strong>
                    </div>

                    {#if gitHistoryBusy}
                      <p class="empty empty--flush">读取 Git history 中…</p>
                    {:else if gitHistoryError}
                      <p class="empty empty--flush">{gitHistoryError}</p>
                    {:else if !gitHistory || gitHistory.commits.length === 0}
                      <p class="empty empty--flush">（当前 artifact 暂无可用 commit history）</p>
                    {:else}
                      <div class="subject-summary">
                        <div class="summary-row">
                          <span class="summary-row__label">Branch</span>
                          <span class="summary-row__value mono">{gitHistory.branch ?? 'detached'}</span>
                        </div>
                        <div class="summary-row">
                          <span class="summary-row__label">Head</span>
                          <span class="summary-row__value mono">
                            {gitHistory.head_short} · {gitHistory.head.slice(0, 12)}
                          </span>
                        </div>
                        <div class="summary-row">
                          <span class="summary-row__label">Workspace</span>
                          <span class="summary-row__value">
                            {gitHistory.dirty ? 'dirty' : 'clean'} · {gitHistory.repo_relative_path}
                          </span>
                        </div>
                      </div>

                      <ul class="result-list" aria-label="Artifact Git history" data-testid="artifact-git-history">
                        {#each gitHistory.commits as commit (commit.revision)}
                          <li class="result-row result-row--triple">
                            <span
                              class={[
                                'pill',
                                selectedGitRevision === commit.revision ? 'pill--warn' : 'pill--neutral'
                              ].join(' ')}
                            >
                              {commit.short_revision}
                            </span>
                            <div class="result-row__main">
                              <span class="result-row__title">{commit.summary}</span>
                              <span class="result-row__detail">
                                {commit.author_name} · {commit.authored_at}
                              </span>
                            </div>
                            <div class="chips chips--tight">
                              <button
                                class={['chip', selectedGitRevision === commit.revision ? 'chip--active' : ''].join(' ')}
                                disabled={gitCompareBusy}
                                type="button"
                                data-testid={`artifact-git-compare:${commit.short_revision}`}
                                onclick={() => void compareRevision(commit.revision)}
                              >
                                {selectedGitRevision === commit.revision ? '对比中' : '对比'}
                              </button>
                              <button
                                class="chip"
                                disabled={rollbackBusy}
                                type="button"
                                data-testid={`artifact-git-rollback:${commit.short_revision}`}
                                onclick={() => void rollbackRevision(commit.revision)}
                              >
                                {rollbackBusy && selectedGitRevision === commit.revision ? '回退中…' : '回退'}
                              </button>
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}

                    {#if gitCompareError}
                      <p class="empty empty--flush">{gitCompareError}</p>
                    {/if}
                  </div>
                {/if}
              </div>
            </section>
          {/snippet}
        </SplitView>
      </div>
    </main>
  {/snippet}
</SplitView>

{#if tabMenu}
  <div
    class="context-menu surface"
    bind:this={tabMenuEl}
    style={`left:${tabMenu.x}px; top:${tabMenu.y}px;`}
    role="menu"
    aria-label="tab menu"
  >
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        openDocument(tabMenu.id);
        closeTabMenu();
      }}
    >
      切换到该 tab
    </button>
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        closeArtifactTab(tabMenu.id);
        closeTabMenu();
      }}
    >
      关闭
    </button>
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        closeOtherTabs(tabMenu.id);
        closeTabMenu();
      }}
    >
      关闭其他
    </button>
    <div class="context-menu__sep" aria-hidden="true"></div>
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        void copyToClipboard(tabMenu.id === MANIFEST_DOC_ID ? 'agentstow.toml' : tabMenu.id, '文档标识');
        closeTabMenu();
      }}
    >
      复制文档标识
    </button>
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      disabled={!tabMenu || !sourcePathOf(editors[tabMenu.id]?.source)}
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        const p = sourcePathOf(editors[tabMenu.id]?.source) ?? '';
        void copyToClipboard(p, 'source path');
        closeTabMenu();
      }}
    >
      复制 source path
    </button>
  </div>
{/if}
