<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { Tabs } from 'bits-ui';

  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import OutputViewer from '$lib/components/OutputViewer.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import {
    ApiClientError,
    getArtifactSource,
    renderArtifact,
    updateArtifactSource
  } from '$lib/api/client';
  import type { ArtifactSourceResponse, WorkspaceSummaryResponse } from '$lib/types';
  import { basenameFromPath, truncateMiddle } from '$lib/utils/format';
  import { buildArtifactTree, relativeSourcePath } from '$lib/workbench/artifacts/artifact_tree';
  import ArtifactTreeNode from '$lib/workbench/artifacts/ArtifactTreeNode.svelte';
  import DiffViewer from '$lib/workbench/DiffViewer.svelte';
  import EditorTabs from '$lib/workbench/EditorTabs.svelte';

  type Props = {
    summary: WorkspaceSummaryResponse | null;
    selectedProfile: string | null;
    onSelectProfile: (id: string) => void;
    onFocusArtifact?: (id: string | null) => void;
    onOpenTarget?: (id: string) => void;
    requestedArtifactId?: string | null;
    onRequestHandled?: (id: string) => void;
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
    requestedArtifactId,
    onRequestHandled,
    shortcutsEnabled,
    statusLine,
    errorMessage,
    setStatusLine,
    setErrorMessage
  }: Props = $props();

  type EditorState = {
    source: ArtifactSourceResponse | null;
    editorText: string;
    previewText: string;
    dirty: boolean;
    busySource: boolean;
    busySave: boolean;
    busyPreview: boolean;
  };

  let openTabs = $state<string[]>([]);
  let activeTab = $state<string | null>(null);
  let editors = $state<Record<string, EditorState>>({});
  let rightMode = $state<'preview' | 'diff'>('preview');
  let autoOpened = $state(false);
  let artifactSearch = $state('');
  let tabMenu = $state<{ id: string; x: number; y: number } | null>(null);
  let tabMenuEl = $state<HTMLDivElement | null>(null);

  const fileArtifacts = $derived((summary?.artifacts ?? []).filter((a) => a.kind === 'file'));
  const dirArtifacts = $derived((summary?.artifacts ?? []).filter((a) => a.kind === 'dir'));
  const workspaceRoot = $derived(summary?.workspace_root ?? null);
  const profiles = $derived(summary?.profiles ?? []);
  const profileIds = $derived(profiles.map((p) => p.id));
  const usageTargets = $derived.by(() => {
    if (!summary || !activeTab) {
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
      const label = id;
      const dirty = editors[id]?.dirty ?? false;
      return { id, label, dirty };
    })
  );

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

  function openArtifact(id: string): void {
    if (!openTabs.includes(id)) {
      openTabs = [...openTabs, id];
    }
    activeTab = id;
    rightMode = 'preview';
    setStatusLine(`已打开 artifact：${id}`);
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

  async function loadArtifactSource(artifactId: string): Promise<void> {
    editors[artifactId] ??= {
      source: null,
      editorText: '',
      previewText: '',
      dirty: false,
      busySource: false,
      busySave: false,
      busyPreview: false
    };

    const st = editors[artifactId]!;
    if (st.busySource || st.source) {
      return;
    }

    st.busySource = true;
    setErrorMessage(null);
    try {
      const source = await getArtifactSource(artifactId);
      st.source = source;
      st.editorText = source.content;
      st.dirty = false;
    } catch (error) {
      st.source = null;
      st.editorText = '';
      st.dirty = false;
      setErrorMessage(describeError(error, '无法读取 artifact source。'));
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
      const updated = await updateArtifactSource(activeTab, st.editorText);
      st.source = updated;
      st.dirty = false;
      setStatusLine('已保存 source。');
    } catch (error) {
      setErrorMessage(describeError(error, '保存失败。'));
    } finally {
      st.busySave = false;
    }
  }

  async function refreshPreview(): Promise<void> {
    if (!activeTab || !selectedProfile) {
      return;
    }
    const st = editors[activeTab];
    if (!st || st.busyPreview) {
      return;
    }

    st.busyPreview = true;
    setErrorMessage(null);
    try {
      const resp = await renderArtifact(activeTab, selectedProfile);
      st.previewText = resp.text;
      setStatusLine('已刷新渲染预览。');
    } catch (error) {
      st.previewText = '';
      setErrorMessage(describeError(error, '渲染预览失败。'));
    } finally {
      st.busyPreview = false;
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
    if (!fileArtifacts.length) {
      return;
    }
    openArtifact(fileArtifacts[0].id);
    autoOpened = true;
  });

  $effect(() => {
    if (!activeTab) {
      return;
    }
    void loadArtifactSource(activeTab);
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
    openArtifact(req);
    onRequestHandled?.(req);
  });

  $effect(() => {
    onFocusArtifact?.(activeTab);
  });

  $effect(() => {
    if (!activeTab || !selectedProfile) {
      return;
    }
    const st = editors[activeTab];
    if (!st?.source) {
      return;
    }
    void refreshPreview();
  });

  const titleLabel = $derived.by(() => {
    if (!activeTab) {
      return '未选择 artifact';
    }
    const st = editors[activeTab];
    const file = st?.source?.source_path ? basenameFromPath(st.source.source_path) : null;
    return file ? `${activeTab} · ${file}` : activeTab;
  });

  const activeValidateAs = $derived(activeEditor?.source?.validate_as ?? 'none');
  const previewMode = $derived(activeValidateAs === 'markdown' ? 'markdown' : 'plain');
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">ARTIFACTS</p>
    <p class="explorer__hint">打开 artifact 后在右侧编辑与预览</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Sources</span>
      <strong>{filteredFileArtifacts.length}</strong>
    </div>

    <md-outlined-text-field
      label="搜索 artifacts"
      placeholder="id 或 source path…"
      value={artifactSearch}
      oninput={(event) => {
        const target = event.currentTarget as { value?: string } | null;
        artifactSearch = typeof target?.value === 'string' ? target.value : '';
      }}
    ></md-outlined-text-field>

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
            onOpenArtifact={openArtifact}
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

  {#if activeTab}
    <div class="explorer__section">
      <div class="section__title">
        <span>Used By Targets</span>
        <strong>{usageTargets.length}</strong>
      </div>

      {#if usageTargets.length === 0}
        <div class="list__static">
          <span class="muted">（未被任何 target 引用）</span>
          <span class="mono">{activeTab}</span>
        </div>
      {:else}
        <ul class="list">
          {#each usageTargets as t (t.id)}
            <li>
              <button
                class="list__item"
                type="button"
                disabled={!onOpenTarget}
                onclick={() => onOpenTarget?.(t.id)}
                title={t.target_path}
              >
                <span class="list__dot list__dot--accent" aria-hidden="true"></span>
                <span class="list__name">{t.id}</span>
                <span class="list__meta">{truncateMiddle(t.target_path, 22)}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
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
</aside>

<main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>{titleLabel}</strong>
      <span class="muted">{selectedProfile ? `· ${selectedProfile}` : ''}</span>
    </div>

    <div class="canvas__actions">
      <md-outlined-button
        disabled={!activeEditor?.dirty || activeEditor?.busySave}
        onclick={() => void saveActiveArtifact()}
        onkeydown={(event) => activateOnKey(event, () => void saveActiveArtifact())}
        role="button"
        tabindex="0"
      >
        {activeEditor?.busySave ? '保存中…' : activeEditor?.dirty ? '保存' : '已保存'}
      </md-outlined-button>
      <md-filled-tonal-button
        disabled={!activeTab || !selectedProfile || activeEditor?.busyPreview}
        onclick={() => void refreshPreview()}
        onkeydown={(event) => activateOnKey(event, () => void refreshPreview())}
        role="button"
        tabindex="0"
      >
        {activeEditor?.busyPreview ? '渲染中…' : '渲染预览'}
      </md-filled-tonal-button>
      <md-text-button
        disabled={!activeTab}
        onclick={closeActiveArtifact}
        onkeydown={(event) => activateOnKey(event, closeActiveArtifact)}
        role="button"
        tabindex="0"
      >
        关闭
      </md-text-button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={52} minLeftPx={360} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">
            <div style="display:flex; align-items:center; justify-content:space-between; gap:12px;">
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
              <CodeEditor
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
                  st.dirty = st.source?.content !== next;
                }}
              />
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Preview</div>
          <div class="pane__body">
            <Tabs.Root value={rightMode} onValueChange={(next) => (rightMode = next as typeof rightMode)}>
              <Tabs.List class="tabs" aria-label="Preview mode">
                <Tabs.Trigger class="tab" value="preview">Rendered</Tabs.Trigger>
                <Tabs.Trigger class="tab" value="diff">Diff</Tabs.Trigger>
              </Tabs.List>

              <Tabs.Content class="tabs__panel" value="preview">
                <OutputViewer
                  text={activeEditor?.previewText || '（暂无预览）'}
                  mode={previewMode}
                />
              </Tabs.Content>

              <Tabs.Content class="tabs__panel" value="diff">
                <DiffViewer
                  original={activeEditor?.source?.content ?? ''}
                  modified={activeEditor?.editorText ?? ''}
                  fromLabel={activeTab ? `${activeTab} (saved)` : 'saved'}
                  toLabel={activeTab ? `${activeTab} (edited)` : 'edited'}
                />
              </Tabs.Content>
            </Tabs.Root>
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>

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
        openArtifact(tabMenu.id);
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
        void copyToClipboard(tabMenu.id, 'artifact id');
        closeTabMenu();
      }}
    >
      复制 artifact id
    </button>
    <button
      class="context-menu__item"
      type="button"
      role="menuitem"
      disabled={!tabMenu || !editors[tabMenu.id]?.source?.source_path}
      onclick={() => {
        if (!tabMenu) {
          return;
        }
        const p = editors[tabMenu.id]?.source?.source_path ?? '';
        void copyToClipboard(p, 'source path');
        closeTabMenu();
      }}
    >
      复制 source path
    </button>
  </div>
{/if}
