<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import CommandPalette, { type PaletteCommand } from '$lib/workbench/CommandPalette.svelte';
  import WorkbenchBottomPanel, {
    type BottomPanelTab
  } from '$lib/workbench/WorkbenchBottomPanel.svelte';
  import WorkspaceBoot from '$lib/workbench/WorkspaceBoot.svelte';
  import EditorTabs from '$lib/workbench/EditorTabs.svelte';
  import SplitView from '$lib/components/SplitView.svelte';
  import WorkbenchRail from '$lib/workbench/WorkbenchRail.svelte';
  import WorkbenchTopbar from '$lib/workbench/WorkbenchTopbar.svelte';
  import ArtifactsView from '$lib/workbench/views/ArtifactsView.svelte';
  import EnvView from '$lib/workbench/views/EnvView.svelte';
  import ImpactView from '$lib/workbench/views/ImpactView.svelte';
  import LinksView from '$lib/workbench/views/LinksView.svelte';
  import McpView from '$lib/workbench/views/McpView.svelte';
  import ScriptsView from '$lib/workbench/views/ScriptsView.svelte';
  import {
    ApiClientError,
    applyLinks,
    copyTextToClipboard,
    emitEnv,
    getImpactAnalysis,
    getLinkStatus,
    getWatchStatus,
    getWorkspaceGit,
    getWorkspaceState,
    getWorkspaceSummary,
    initWorkspace,
    pickWorkspace,
    planLinks,
    probeWorkspace,
    repairLinks,
    runScript,
    selectWorkspace
  } from '$lib/api/client';
  import type {
    EnvEmitResponse,
    EnvUsageRefResponse,
    ImpactAnalysisResponse,
    LinkOperationResponse,
    LinkStatusResponseItem,
    ScriptRunResponse,
    ShellKindResponse,
    WorkspaceGitSummaryResponse,
    WorkspaceProbeResponse,
    WorkspaceSelectResponse,
    WorkspaceStateResponse,
    WorkspaceSummaryResponse,
    WatchStatusResponse
  } from '$lib/types';
  import {
    basenameFromPath,
    formatRelativeTime,
    truncateMiddle
  } from '$lib/utils/format';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';
  import {
    applyResolvedTheme,
    createThemeMediaQuery,
    readThemePreference,
    resolveTheme,
    writeThemePreference,
    type ResolvedTheme,
    type ThemePreference
  } from '$lib/workbench/theme';

  type ViewKey = 'artifacts' | 'links' | 'env' | 'scripts' | 'mcp' | 'impact';
  type ImpactMode = 'artifact' | 'profile' | 'artifact_profile';
  type WorkbenchDocument = {
    id: string;
    kind: ViewKey;
    title: string;
    entityId?: string | null;
    dirty?: boolean;
  };

  const ARTIFACTS_DOC_ID = 'artifacts:workspace';
  const IMPACT_DOC_ID = 'impact:analysis';

  let view = $state<ViewKey>('artifacts');
  let paletteOpen = $state(false);
  let artifactRequestId = $state<string | null>(null);
  let manifestInsertRequest = $state<ManifestInsertKind | null>(null);
  let editorDocs = $state<WorkbenchDocument[]>([]);
  let activeDocId = $state<string | null>(null);
  let themePreference = $state<ThemePreference>('system');
  let resolvedTheme = $state<ResolvedTheme>('dark');
  let railExpanded = $state(false);
  let bottomPanelOpen = $state(false);
  let bottomPanelTab = $state<BottomPanelTab>('problems');

  let workspaceState = $state<WorkspaceStateResponse | null>(null);
  let workspaceInput = $state('');
  let workspaceProbe = $state<WorkspaceProbeResponse | null>(null);
  let initGit = $state(false);
  let pickerBusy = $state(false);

  let summary = $state<WorkspaceSummaryResponse | null>(null);
  let gitInfo = $state<WorkspaceGitSummaryResponse | null>(null);
  let watchStatus = $state<WatchStatusResponse | null>(null);

  let selectedArtifact = $state<string | null>(null);
  let selectedProfile = $state<string | null>(null);
  let selectedEnvSet = $state<string | null>(null);
  let selectedScript = $state<string | null>(null);
  let selectedTargetId = $state<string | null>(null);
  let selectedTargets = $state<string[]>([]);
  let selectedMcpServerId = $state<string | null>(null);

  let selectedShell = $state<ShellKindResponse>('bash');
  let envScript = $state<EnvEmitResponse | null>(null);

  let scriptStdin = $state('');
  let scriptRun = $state<ScriptRunResponse | null>(null);

  let linkStatus = $state<LinkStatusResponseItem[] | null>(null);
  let linkSearch = $state('');
  let linkUnhealthyOnly = $state(false);
  let linkForce = $state(false);
  let linkScope = $state<'selected' | 'all'>('selected');
  let linkOp = $state<LinkOperationResponse | null>(null);
  let linkOpTitle = $state<string | null>(null);

  let impactMode = $state<ImpactMode>('artifact_profile');
  let impact = $state<ImpactAnalysisResponse | null>(null);

  let busy = $state({
    boot: false,
    summary: false,
    git: false,
    watch: false,
    env_emit: false,
    script_run: false,
    links: false,
    link_op: false,
    impact: false
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
  const gitHeadline = $derived.by(() => {
    if (!gitInfo) {
      return 'Git 未接入';
    }
    const branch = gitInfo.branch ?? 'detached';
    return `${branch} @ ${gitInfo.head_short}${gitInfo.dirty ? ' • dirty' : ''}`;
  });

  const artifacts = $derived(summary?.artifacts ?? []);
  const profiles = $derived(summary?.profiles ?? []);
  const fileArtifacts = $derived(artifacts.filter((artifact) => artifact.kind === 'file'));
  const profileIds = $derived(profiles.map((profile) => profile.id));
  const envSets = $derived(summary?.env_sets ?? []);
  const scripts = $derived(summary?.scripts ?? []);
  const mcpServers = $derived(summary?.mcp_servers ?? []);
  const targets = $derived(summary?.targets ?? []);
  const activeEnvSet = $derived(envSets.find((envSet) => envSet.id === selectedEnvSet) ?? null);
  const activeScript = $derived(scripts.find((script) => script.id === selectedScript) ?? null);
  const activeMcpServer = $derived(
    mcpServers.find((server) => server.id === selectedMcpServerId) ?? null
  );
  const activeTarget = $derived(
    selectedTargetId ? targets.find((t) => t.id === selectedTargetId) ?? null : null
  );
  const activeLinkStatus = $derived(
    activeTarget
      ? (linkStatus ?? []).find((item) => item.target_path === activeTarget.target_path) ?? null
      : null
  );

  const watchPill = $derived.by(
    (): { tone: 'neutral' | 'warn' | 'ok'; label: string } => {
    if (!watchStatus) {
      return { tone: 'neutral', label: 'watcher 未连接' };
    }
    if (!watchStatus.healthy) {
      return { tone: 'warn', label: watchStatus.mode };
    }
    return { tone: 'ok', label: watchStatus.mode };
    }
  );

  const watchActivity = $derived.by(() => {
    if (!watchStatus?.last_event) {
      return '等待文件变化';
    }
    return `${watchStatus.last_event} · ${formatRelativeTime(watchStatus.last_event_at)}`;
  });

  const watchTraceEvents = $derived(watchStatus?.recent_events ?? []);
  const watchTraceCount = $derived(watchTraceEvents.length);
  const problemCount = $derived((errorMessage ? 1 : 0) + (summary?.issues.length ?? 0));

  function describeWatchRefreshed(): void {
    statusLine =
      watchTraceCount > 0
        ? `已刷新 watcher trace（${watchTraceCount} events）。`
        : '已刷新 watcher 状态。';
  }

  const viewLabels: Record<ViewKey, string> = {
    artifacts: 'Artifacts',
    links: 'Links',
    env: 'Env',
    scripts: 'Scripts',
    mcp: 'MCP',
    impact: 'Impact'
  };

  const editorTabModel = $derived.by(() =>
    editorDocs.map((doc) => ({
      id: doc.id,
      label: doc.title,
      dirty: Boolean(doc.dirty)
    }))
  );

  function buildDocTitle(kind: ViewKey, entityId?: string | null): string {
    if (!entityId) {
      return viewLabels[kind];
    }

    switch (kind) {
      case 'links':
        return `Target · ${entityId}`;
      case 'env':
        return `Env · ${entityId}`;
      case 'scripts':
        return `Script · ${entityId}`;
      case 'mcp':
        return `MCP · ${entityId}`;
      case 'impact':
        return 'Impact';
      case 'artifacts':
      default:
        return 'Artifacts';
    }
  }

  function syncSelectionFromDocument(doc: WorkbenchDocument | null): void {
    if (!doc) {
      return;
    }

    view = doc.kind;

    if (doc.kind === 'env' && doc.entityId) {
      selectedEnvSet = doc.entityId;
      return;
    }

    if (doc.kind === 'scripts' && doc.entityId) {
      selectedScript = doc.entityId;
      return;
    }

    if (doc.kind === 'links' && doc.entityId) {
      selectedTargetId = doc.entityId;
      if (!selectedTargets.includes(doc.entityId)) {
        selectedTargets = [doc.entityId];
      }
      return;
    }

    if (doc.kind === 'mcp' && doc.entityId) {
      selectedMcpServerId = doc.entityId;
      return;
    }

    if (doc.kind === 'artifacts' && artifactRequestId === null) {
      artifactRequestId = selectedArtifact ?? null;
    }
  }

  function upsertDocument(doc: WorkbenchDocument, activate = true): void {
    const idx = editorDocs.findIndex((item) => item.id === doc.id);
    if (idx === -1) {
      editorDocs = [...editorDocs, doc];
    } else {
      const next = editorDocs.slice();
      next[idx] = { ...next[idx], ...doc };
      editorDocs = next;
    }

    if (activate) {
      activeDocId = doc.id;
      syncSelectionFromDocument(doc);
    }
  }

  function ensureFallbackDocument(): void {
    if (editorDocs.length > 0 && activeDocId && editorDocs.some((doc) => doc.id === activeDocId)) {
      const doc = editorDocs.find((item) => item.id === activeDocId) ?? null;
      syncSelectionFromDocument(doc);
      return;
    }

    if (editorDocs.length > 0) {
      const doc = editorDocs.at(-1) ?? editorDocs[0] ?? null;
      activeDocId = doc?.id ?? null;
      syncSelectionFromDocument(doc);
      return;
    }

    const fallback: WorkbenchDocument = {
      id: ARTIFACTS_DOC_ID,
      kind: 'artifacts',
      title: 'Artifacts'
    };
    editorDocs = [fallback];
    activeDocId = fallback.id;
    syncSelectionFromDocument(fallback);
  }

  function activateDocument(id: string): void {
    const doc = editorDocs.find((item) => item.id === id) ?? null;
    if (!doc) {
      return;
    }
    activeDocId = id;
    syncSelectionFromDocument(doc);
  }

  function closeDocument(id: string): void {
    const closingIndex = editorDocs.findIndex((item) => item.id === id);
    if (closingIndex === -1) {
      return;
    }

    const nextDocs = editorDocs.filter((item) => item.id !== id);
    editorDocs = nextDocs;

    if (!nextDocs.length) {
      activeDocId = null;
      ensureFallbackDocument();
      return;
    }

    if (activeDocId === id) {
      const nextDoc = nextDocs[Math.max(0, closingIndex - 1)] ?? nextDocs[0] ?? null;
      activeDocId = nextDoc?.id ?? null;
      syncSelectionFromDocument(nextDoc);
    }
  }

  function reorderDocuments(nextOrder: string[]): void {
    const byId = new Map(editorDocs.map((doc) => [doc.id, doc]));
    const orderedDocs = nextOrder
      .map((id) => byId.get(id) ?? null)
      .filter((doc): doc is WorkbenchDocument => doc !== null);
    const seenIds = new Set(orderedDocs.map((doc) => doc.id));

    editorDocs = [
      ...orderedDocs,
      ...editorDocs.filter((doc) => !seenIds.has(doc.id))
    ];
  }

  function openArtifactsWorkspace(requestId?: string | null): void {
    if (requestId) {
      artifactRequestId = requestId;
    }

    upsertDocument({
      id: ARTIFACTS_DOC_ID,
      kind: 'artifacts',
      title: 'Artifacts'
    });
  }

  function openEnvDocument(id?: string | null): void {
    if (id) {
      selectEnvSet(id);
    }

    const entityId = id ?? selectedEnvSet ?? null;
    upsertDocument({
      id: entityId ? `env:${entityId}` : 'env:index',
      kind: 'env',
      title: buildDocTitle('env', entityId),
      entityId
    });
  }

  function openScriptDocument(id?: string | null): void {
    if (id) {
      selectScript(id);
    }

    const entityId = id ?? selectedScript ?? null;
    upsertDocument({
      id: entityId ? `scripts:${entityId}` : 'scripts:index',
      kind: 'scripts',
      title: buildDocTitle('scripts', entityId),
      entityId
    });
  }

  function openTargetDocument(id?: string | null): void {
    if (id) {
      selectTargetExclusive(id);
    }

    const entityId = id ?? selectedTargetId ?? null;
    upsertDocument({
      id: entityId ? `links:${entityId}` : 'links:index',
      kind: 'links',
      title: buildDocTitle('links', entityId),
      entityId
    });
  }

  function openMcpDocument(id?: string | null): void {
    if (id) {
      selectMcpServer(id);
    }

    const entityId = id ?? selectedMcpServerId ?? null;
    upsertDocument({
      id: entityId ? `mcp:${entityId}` : 'mcp:index',
      kind: 'mcp',
      title: buildDocTitle('mcp', entityId),
      entityId
    });
  }

  function openImpactDocument(): void {
    upsertDocument({
      id: IMPACT_DOC_ID,
      kind: 'impact',
      title: 'Impact',
      entityId: `${selectedArtifact ?? 'none'}@${selectedProfile ?? 'none'}`
    });
  }

  function openDefaultDocument(kind: ViewKey): void {
    if (kind === 'artifacts') {
      openArtifactsWorkspace();
      return;
    }
    if (kind === 'links') {
      openTargetDocument();
      return;
    }
    if (kind === 'env') {
      openEnvDocument();
      return;
    }
    if (kind === 'scripts') {
      openScriptDocument();
      return;
    }
    if (kind === 'mcp') {
      openMcpDocument();
      return;
    }
    openImpactDocument();
  }

  const statusFocus = $derived.by(() => {
    if (view === 'artifacts') {
      return selectedArtifact ? `artifact:${selectedArtifact}` : 'manifest';
    }
    if (view === 'links') {
      if (selectedTargets.length > 1) {
        return `${selectedTargets.length} targets`;
      }
      return selectedTargetId ? `target:${selectedTargetId}` : 'targets';
    }
    if (view === 'env') {
      return selectedEnvSet ? `env:${selectedEnvSet}` : 'env';
    }
    if (view === 'scripts') {
      return selectedScript ? `script:${selectedScript}` : 'scripts';
    }
    if (view === 'mcp') {
      return selectedMcpServerId ? `mcp:${selectedMcpServerId}` : 'mcp';
    }
    return selectedArtifact || selectedProfile
      ? `impact:${selectedArtifact ?? 'none'}@${selectedProfile ?? 'none'}`
      : 'impact';
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

  function returnToWorkspaceBoot(): void {
    workspaceProbe = workspaceState?.workspace ?? workspaceProbe;
    workspaceState = {
      workspace_root: workspaceRoot,
      manifest_present: false,
      workspace: workspaceProbe
    };
    bottomPanelOpen = false;
    gitInfo = null;
    editorDocs = [];
    activeDocId = null;
    summary = null;
    linkStatus = null;
    linkOp = null;
    linkOpTitle = null;
    impact = null;
    envScript = null;
    scriptRun = null;
    statusLine = '请选择一个 workspace。';
  }

  async function refreshWorkspaceState(): Promise<WorkspaceStateResponse | null> {
    try {
      const nextState = await getWorkspaceState();
      workspaceState = nextState;
      workspaceProbe = nextState.workspace ?? null;
      if (workspaceInput.trim().length === 0 && nextState.workspace_root) {
        workspaceInput = nextState.workspace_root;
      }
      return nextState;
    } catch (error) {
      errorMessage = describeError(error, '无法读取 workspace 状态。');
      return null;
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
      if (!selectedEnvSet) {
        selectedEnvSet = nextSummary.env_sets[0]?.id ?? null;
      }
      if (!selectedScript) {
        selectedScript = nextSummary.scripts[0]?.id ?? null;
      }
      if (!selectedTargetId) {
        selectedTargetId = nextSummary.targets[0]?.id ?? null;
      } else if (!nextSummary.targets.some((t) => t.id === selectedTargetId)) {
        selectedTargetId = nextSummary.targets[0]?.id ?? null;
      }

      const allowedTargets = new Set(nextSummary.targets.map((t) => t.id));
      selectedTargets = selectedTargets.filter((id) => allowedTargets.has(id));
      if (selectedTargets.length === 0 && selectedTargetId) {
        selectedTargets = [selectedTargetId];
      }
      if (selectedMcpServerId && !nextSummary.mcp_servers.some((server) => server.id === selectedMcpServerId)) {
        selectedMcpServerId = nextSummary.mcp_servers[0]?.id ?? null;
      } else if (!selectedMcpServerId) {
        selectedMcpServerId = nextSummary.mcp_servers[0]?.id ?? null;
      }

      const artifactIds = new Set(nextSummary.artifacts.map((artifact) => artifact.id));
      const envIds = new Set(nextSummary.env_sets.map((envSet) => envSet.id));
      const scriptIds = new Set(nextSummary.scripts.map((script) => script.id));
      const targetIds = new Set(nextSummary.targets.map((target) => target.id));
      const mcpIds = new Set(nextSummary.mcp_servers.map((server) => server.id));

      editorDocs = editorDocs
        .filter((doc) => {
          if (doc.kind === 'env' && doc.entityId) {
            return envIds.has(doc.entityId);
          }
          if (doc.kind === 'scripts' && doc.entityId) {
            return scriptIds.has(doc.entityId);
          }
          if (doc.kind === 'links' && doc.entityId) {
            return targetIds.has(doc.entityId);
          }
          if (doc.kind === 'mcp' && doc.entityId) {
            return mcpIds.has(doc.entityId);
          }
          if (doc.kind === 'artifacts' && doc.entityId) {
            return artifactIds.has(doc.entityId) || doc.id === ARTIFACTS_DOC_ID;
          }
          return true;
        })
        .map((doc) => ({
          ...doc,
          title: buildDocTitle(doc.kind, doc.entityId)
        }));

      if (activeDocId && !editorDocs.some((doc) => doc.id === activeDocId)) {
        activeDocId = editorDocs.at(-1)?.id ?? null;
      }
    } catch (error) {
      errorMessage = describeError(error, '无法读取 workspace 数据。');
      summary = null;
    } finally {
      busy.summary = false;
    }
  }

  async function refreshWorkspaceGit(): Promise<void> {
    busy.git = true;
    try {
      gitInfo = await getWorkspaceGit();
    } catch (error) {
      gitInfo = null;
      errorMessage ??= describeError(error, '无法读取 Git 状态。');
    } finally {
      busy.git = false;
    }
  }

  async function refreshWatchStatus(): Promise<boolean> {
    busy.watch = true;
    try {
      watchStatus = await getWatchStatus();
      return true;
    } catch (error) {
      watchStatus = null;
      errorMessage ??= describeError(error, '无法读取 watcher 状态。');
      return false;
    } finally {
      busy.watch = false;
    }
  }

  async function refreshTracePanel(): Promise<void> {
    if (await refreshWatchStatus()) {
      describeWatchRefreshed();
    }
  }

  async function bootstrapConfigured(): Promise<void> {
    await Promise.all([refreshSummary(), refreshWatchStatus(), refreshWorkspaceGit()]);
    linkStatus = null;
    impact = null;
    ensureFallbackDocument();
    statusLine = '已连接到 workspace。';
  }

  async function refreshLinkStatus(): Promise<void> {
    busy.links = true;
    errorMessage = null;
    try {
      const items = await getLinkStatus();
      linkStatus = items;
      statusLine = '已刷新 link status。';
    } catch (error) {
      linkStatus = null;
      errorMessage = describeError(error, '无法读取 link status。');
    } finally {
      busy.links = false;
    }
  }

  async function runLinkOperation(kind: 'plan' | 'apply' | 'repair'): Promise<void> {
    busy.link_op = true;
    errorMessage = null;
    try {
      const chosenTargets: string[] = [];
      if (linkScope === 'selected') {
        const ids = selectedTargets.filter(Boolean);
        if (ids.length === 0) {
          throw new Error('请先选择至少一个 target（提示：在列表里 Ctrl/Cmd 点击可多选）。');
        }
        chosenTargets.push(...ids);
      }

      const default_profile = selectedProfile ?? null;

      if (kind === 'plan') {
        linkOp = await planLinks({ targets: chosenTargets, default_profile });
        linkOpTitle = 'plan';
        statusLine = `plan 完成（${linkOp.items.length} items）。`;
        return;
      }

      if (kind === 'apply') {
        linkOp = await applyLinks({
          targets: chosenTargets,
          default_profile,
          force: linkForce
        });
        linkOpTitle = 'apply';
        statusLine = `apply 完成（${linkOp.items.length} items）。`;
        await refreshLinkStatus();
        return;
      }

      linkOp = await repairLinks({
        targets: chosenTargets,
        default_profile,
        force: linkForce
      });
      linkOpTitle = 'repair';
      statusLine = `repair 完成（${linkOp.items.length} items）。`;
      await refreshLinkStatus();
    } catch (error) {
      linkOp = null;
      linkOpTitle = null;
      errorMessage = describeError(error, `link ${kind} 失败。`);
    } finally {
      busy.link_op = false;
    }
  }

  async function refreshImpactAnalysis(): Promise<void> {
    const artifact = selectedArtifact;
    const profile = selectedProfile;

    const query: { artifact?: string | null; profile?: string | null } = {};
    if (impactMode === 'artifact') {
      query.artifact = artifact;
      query.profile = null;
    } else if (impactMode === 'profile') {
      query.artifact = null;
      query.profile = profile;
    } else {
      query.artifact = artifact;
      query.profile = profile;
    }

    if (!query.artifact && !query.profile) {
      impact = null;
      errorMessage = 'impact analysis 需要至少选择 artifact 或 profile。';
      return;
    }

    busy.impact = true;
    errorMessage = null;
    try {
      impact = await getImpactAnalysis(query);
      statusLine = '已刷新 impact analysis。';
    } catch (error) {
      impact = null;
      errorMessage = describeError(error, '无法生成 impact analysis。');
    } finally {
      busy.impact = false;
    }
  }

  async function handleProbeWorkspace(
    announceReady = true,
    manageBusy = true
  ): Promise<WorkspaceProbeResponse | null> {
    const requested = workspaceInput.trim();
    if (!requested) {
      errorMessage = '请输入 workspace 路径。';
      return null;
    }

    if (manageBusy) {
      busy.boot = true;
    }
    errorMessage = null;
    try {
      const probe = await probeWorkspace(requested);
      workspaceProbe = probe;
      workspaceInput = probe.resolved_workspace_root;
      if (announceReady) {
        if (probe.selectable && probe.manifest_present) {
          statusLine = '已确认路径可直接打开。';
        } else if (probe.initializable && !probe.exists) {
          statusLine = '目标路径不存在，可直接创建并初始化 workspace。';
        } else if (probe.initializable) {
          statusLine = '目标目录存在，但还没有 agentstow.toml，可继续初始化。';
        } else {
          statusLine = probe.reason ?? '当前路径不可用。';
        }
      }
      return probe;
    } catch (error) {
      errorMessage = describeError(error, '检查 workspace 路径失败。');
      return null;
    } finally {
      if (manageBusy) {
        busy.boot = false;
      }
    }
  }

  async function applySelectedWorkspaceResponse(resp: WorkspaceSelectResponse): Promise<void> {
    workspaceProbe = resp.workspace;
    workspaceInput = resp.workspace_root;
    workspaceState = {
      workspace_root: resp.workspace_root,
      manifest_present: resp.manifest_present,
      workspace: resp.workspace
    };

    if (!resp.manifest_present) {
      statusLine = '已选择目录，但还没有 agentstow.toml。你可以继续初始化 workspace。';
      return;
    }

    await bootstrapConfigured();
  }

  async function handleSelectWorkspace(): Promise<void> {
    busy.boot = true;
    errorMessage = null;
    try {
      const probe = await handleProbeWorkspace(false, false);
      if (!probe) {
        return;
      }
      if (!probe.selectable) {
        if (probe.initializable) {
          statusLine = probe.exists
            ? '目录存在但尚未初始化。你可以直接创建 workspace。'
            : '路径不存在。你可以直接创建并初始化 workspace。';
          errorMessage = null;
          return;
        }
        errorMessage = probe.reason ?? '选择的路径不可作为 workspace。';
        return;
      }
      const resp = await selectWorkspace(workspaceInput.trim());
      await applySelectedWorkspaceResponse(resp);
    } catch (error) {
      errorMessage = describeError(error, '选择 workspace 失败。');
    } finally {
      busy.boot = false;
    }
  }

  async function handlePickWorkspace(): Promise<void> {
    pickerBusy = true;
    errorMessage = null;
    try {
      const resp = await pickWorkspace();
      if (!resp) {
        statusLine = '已取消目录选择。';
        return;
      }
      await applySelectedWorkspaceResponse(resp);
    } catch (error) {
      errorMessage = describeError(error, '打开本地目录选择器失败。');
    } finally {
      pickerBusy = false;
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
      workspaceProbe = resp.workspace;
      workspaceInput = resp.workspace_root;
      workspaceState = {
        workspace_root: resp.workspace_root,
        manifest_present: true,
        workspace: resp.workspace
      };
      statusLine = resp.created ? '已初始化 workspace。' : 'workspace 已存在 manifest，已直接打开。';
      await bootstrapConfigured();
    } catch (error) {
      errorMessage = describeError(error, '初始化 workspace 失败。');
    } finally {
      busy.boot = false;
    }
  }

  async function copyToClipboard(text: string, label: string): Promise<void> {
    try {
      await copyTextToClipboard(text);
      if (typeof window !== 'undefined') {
        (window as Window & { __agentstowCopiedText__?: string }).__agentstowCopiedText__ = text;
      }
      statusLine = `已复制${label}到剪贴板。`;
    } catch (error) {
      statusLine = describeError(error, `复制${label}失败。当前环境未提供可用剪贴板。`);
    }
  }

  const shellChoices: ShellKindResponse[] = ['bash', 'zsh', 'fish', 'powershell', 'cmd'];

  async function handleEnvEmit(): Promise<void> {
    if (!selectedEnvSet) {
      return;
    }

    busy.env_emit = true;
    errorMessage = null;
    try {
      envScript = await emitEnv({
        env_set_id: selectedEnvSet,
        shell: selectedShell
      });
      statusLine = '已生成 env 激活脚本。';
    } catch (error) {
      envScript = null;
      errorMessage = describeError(error, '生成 env 激活脚本失败。');
    } finally {
      busy.env_emit = false;
    }
  }

  async function handleScriptRun(): Promise<void> {
    if (!selectedScript) {
      return;
    }

    busy.script_run = true;
    errorMessage = null;
    try {
      scriptRun = await runScript(selectedScript, {
        stdin: scriptStdin.trim().length ? scriptStdin : null
      });
      statusLine = `脚本已执行（exit=${scriptRun.exit_code}）。`;
    } catch (error) {
      scriptRun = null;
      errorMessage = describeError(error, '脚本执行失败。');
    } finally {
      busy.script_run = false;
    }
  }

  function selectArtifact(id: string): void {
    selectedArtifact = id;
    impact = null;
    statusLine = `已选择 artifact：${id}`;
  }

  function focusArtifact(id: string | null): void {
    selectedArtifact = id;
    impact = null;
  }

  function selectProfile(id: string): void {
    selectedProfile = id;
    impact = null;
    statusLine = `已选择 profile：${id}`;
  }

  function selectEnvSet(id: string): void {
    selectedEnvSet = id;
    envScript = null;
    statusLine = `已选择 env set：${id}`;
  }

  function selectScript(id: string): void {
    selectedScript = id;
    scriptRun = null;
    statusLine = `已选择 script：${id}`;
  }

  function setImpactMode(next: ImpactMode): void {
    impactMode = next;
    impact = null;
    if (view === 'impact') {
      void refreshImpactAnalysis();
    }
  }

  function selectTargetExclusive(id: string): void {
    selectedTargetId = id;
    selectedTargets = [id];
    linkOp = null;
    linkOpTitle = null;
    statusLine = `已选择 target：${id}`;
  }

  function toggleTargetSelection(id: string): void {
    selectedTargetId = id;
    linkOp = null;
    linkOpTitle = null;
    if (selectedTargets.includes(id)) {
      selectedTargets = selectedTargets.filter((t) => t !== id);
      statusLine = `已取消选择 target：${id}`;
      return;
    }
    selectedTargets = [...selectedTargets, id];
    statusLine = `已追加选择 target：${id}`;
  }

  function selectMcpServer(id: string): void {
    selectedMcpServerId = id;
    statusLine = `已选择 MCP server：${id}`;
  }

  function openTargetInLinks(targetId: string): void {
    openTargetDocument(targetId);
  }

  function openEnvUsageRef(ref: EnvUsageRefResponse): void {
    if (ref.owner_kind === 'env_set') {
      openEnvDocument(ref.owner_id);
      return;
    }
    if (ref.owner_kind === 'script') {
      openScriptDocument(ref.owner_id);
      return;
    }
    openMcpDocument(ref.owner_id);
  }

  function requestOpenArtifact(id: string): void {
    openArtifactsWorkspace(id);
  }

  function openProfileInArtifacts(id: string): void {
    selectProfile(id);
    openArtifactsWorkspace();
  }

  function openManifestEditor(): void {
    openArtifactsWorkspace('$manifest');
    statusLine = '已切换到 manifest 编辑器。';
  }

  function requestManifestInsert(kind: ManifestInsertKind): void {
    openArtifactsWorkspace('$manifest');
    manifestInsertRequest = kind;
  }

  function setThemePreference(next: ThemePreference): void {
    themePreference = next;
    writeThemePreference(next);

    const media = createThemeMediaQuery();
    resolvedTheme = resolveTheme(next, media?.matches ?? false);
    applyResolvedTheme(resolvedTheme);
    statusLine = `已切换主题：${next}（当前生效：${resolvedTheme}）。`;
  }

  async function toggleBottomPanel(tab?: BottomPanelTab): Promise<void> {
    if (!manifestPresent) {
      return;
    }

    if (!bottomPanelOpen) {
      bottomPanelOpen = true;
      bottomPanelTab = tab ?? bottomPanelTab;
    } else if (tab && bottomPanelTab !== tab) {
      bottomPanelTab = tab;
    } else {
      bottomPanelOpen = false;
      return;
    }

    if (bottomPanelTab === 'trace') {
      await refreshTracePanel();
    }
  }

  const themeChoices: Array<{ id: ThemePreference; label: string }> = [
    { id: 'system', label: 'Auto' },
    { id: 'light', label: 'Light' },
    { id: 'dark', label: 'Dark' }
  ];

  const paletteCommands = $derived.by((): PaletteCommand[] => {
    const cmds: PaletteCommand[] = [];

    const nav = (key: ViewKey) => {
      cmds.push({
        id: `nav:${key}`,
        group: 'Navigate',
        title: `Go to ${key}`,
        keywords: `view ${key}`,
        run: () => {
          openDefaultDocument(key);
        }
      });
    };

    nav('artifacts');
    nav('links');
    nav('env');
    nav('scripts');
    nav('mcp');
    nav('impact');

    cmds.push({
      id: 'action:refresh',
      group: 'Actions',
      title: 'Refresh workspace',
      keywords: 'reload refresh',
      disabled: !manifestPresent,
      run: async () => {
        await bootstrapConfigured();
      }
    });

    cmds.push({
      id: 'action:switch-workspace',
      group: 'Actions',
      title: 'Switch workspace',
      keywords: 'workspace switch boot',
      disabled: !manifestPresent,
      run: () => {
        returnToWorkspaceBoot();
      }
    });

    cmds.push({
      id: 'watch:trace',
      group: 'Watch',
      title:
        bottomPanelOpen && bottomPanelTab === 'trace' ? 'Hide watch trace' : 'Open watch trace',
      keywords: 'watch trace events statusbar',
      disabled: !manifestPresent,
      run: async () => {
        await toggleBottomPanel('trace');
      }
    });

    cmds.push({
      id: 'problems:toggle',
      group: 'Watch',
      title:
        bottomPanelOpen && bottomPanelTab === 'problems'
          ? 'Hide problems panel'
          : 'Open problems panel',
      keywords: 'problems issues errors panel statusbar',
      disabled: !manifestPresent,
      run: async () => {
        await toggleBottomPanel('problems');
      }
    });

    if (gitInfo) {
      const git = gitInfo;
      cmds.push({
        id: 'git:copy-head',
        group: 'Git',
        title: `Copy HEAD ${git.head_short}`,
        subtitle: git.branch ?? 'detached HEAD',
        keywords: `git head branch ${git.branch ?? ''} ${git.head_short}`,
        run: async () => {
          await copyToClipboard(git.head, 'Git HEAD');
        }
      });
      cmds.push({
        id: 'git:copy-root',
        group: 'Git',
        title: 'Copy repository root',
        subtitle: truncateMiddle(git.repo_root, 56),
        keywords: `git repo root ${git.repo_root}`,
        run: async () => {
          await copyToClipboard(git.repo_root, 'Git 仓库路径');
        }
      });
    }

    for (const theme of [
      { id: 'system' as const, label: 'Theme: Follow system' },
      { id: 'light' as const, label: 'Theme: Light' },
      { id: 'dark' as const, label: 'Theme: Dark' }
    ]) {
      cmds.push({
        id: `theme:${theme.id}`,
        group: 'Appearance',
        title: theme.label,
        keywords: `theme appearance ${theme.id}`,
        disabled: themePreference === theme.id,
        run: () => {
          setThemePreference(theme.id);
        }
      });
    }

    cmds.push({
      id: 'manifest:open',
      group: 'Manifest',
      title: 'Open workspace manifest',
      keywords: 'manifest workspace config toml',
      disabled: !manifestPresent,
      run: () => {
        openManifestEditor();
      }
    });

    for (const kind of [
      'profile',
      'artifact',
      'target',
      'env_set',
      'script',
      'mcp_server'
    ] as const) {
      cmds.push({
        id: `manifest:new:${kind}`,
        group: 'Create',
        title: `New ${kind.replace('_', ' ')}`,
        keywords: `new create ${kind} manifest`,
        disabled: !manifestPresent,
        run: () => {
          requestManifestInsert(kind);
        }
      });
    }

    cmds.push({
      id: 'links:status',
      group: 'Links',
      title: 'Links: Refresh status',
      disabled: !manifestPresent,
      keywords: 'links status refresh',
      run: async () => {
        openTargetDocument();
        await refreshLinkStatus();
      }
    });

    cmds.push({
      id: 'links:plan:selected',
      group: 'Links',
      title: 'Links: Plan (selected)',
      disabled: selectedTargets.length === 0,
      keywords: 'links plan selected',
      run: async () => {
        openTargetDocument();
        linkScope = 'selected';
        await runLinkOperation('plan');
      }
    });

    cmds.push({
      id: 'links:apply:selected',
      group: 'Links',
      title: 'Links: Apply (selected)',
      disabled: selectedTargets.length === 0,
      keywords: 'links apply selected',
      run: async () => {
        openTargetDocument();
        linkScope = 'selected';
        await runLinkOperation('apply');
      }
    });

    cmds.push({
      id: 'links:repair:selected',
      group: 'Links',
      title: 'Links: Repair (selected)',
      disabled: selectedTargets.length === 0,
      keywords: 'links repair selected',
      run: async () => {
        openTargetDocument();
        linkScope = 'selected';
        await runLinkOperation('repair');
      }
    });

    cmds.push({
      id: 'links:plan:all',
      group: 'Links',
      title: 'Links: Plan (all)',
      keywords: 'links plan all',
      disabled: !manifestPresent,
      run: async () => {
        openTargetDocument();
        linkScope = 'all';
        await runLinkOperation('plan');
      }
    });

    cmds.push({
      id: 'links:apply:all',
      group: 'Links',
      title: 'Links: Apply (all)',
      keywords: 'links apply all',
      disabled: !manifestPresent,
      run: async () => {
        openTargetDocument();
        linkScope = 'all';
        await runLinkOperation('apply');
      }
    });

    cmds.push({
      id: 'links:repair:all',
      group: 'Links',
      title: 'Links: Repair (all)',
      keywords: 'links repair all',
      disabled: !manifestPresent,
      run: async () => {
        openTargetDocument();
        linkScope = 'all';
        await runLinkOperation('repair');
      }
    });

    cmds.push({
      id: 'env:emit',
      group: 'Env',
      title: 'Env: Emit activation script',
      keywords: 'env emit shell',
      disabled: !selectedEnvSet,
      run: async () => {
        openEnvDocument();
        await handleEnvEmit();
      }
    });

    cmds.push({
      id: 'scripts:run',
      group: 'Scripts',
      title: 'Scripts: Run',
      keywords: 'scripts run execute',
      disabled: !selectedScript,
      run: async () => {
        openScriptDocument();
        await handleScriptRun();
      }
    });

    cmds.push({
      id: 'impact:run',
      group: 'Impact',
      title: 'Impact: Run analysis',
      keywords: 'impact analyze',
      disabled: !(selectedArtifact || selectedProfile),
      run: async () => {
        openImpactDocument();
        await refreshImpactAnalysis();
      }
    });

    for (const a of fileArtifacts) {
      cmds.push({
        id: `artifact:${a.id}`,
        group: 'Artifacts',
        title: `Open artifact: ${a.id}`,
        subtitle: truncateMiddle(a.source_path, 56),
        keywords: `${a.id} ${a.source_path} artifact`,
        run: () => {
          requestOpenArtifact(a.id);
        }
      });
    }

    for (const t of targets) {
      cmds.push({
        id: `target:${t.id}`,
        group: 'Targets',
        title: `Open target: ${t.id}`,
        subtitle: truncateMiddle(t.target_path, 56),
        keywords: `${t.id} ${t.target_path} ${t.artifact_id} ${t.profile ?? ''} target`,
        run: () => {
          openTargetDocument(t.id);
        }
      });
    }

    return cmds;
  });

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const isMod = event.metaKey || event.ctrlKey;
      if (!isMod) {
        return;
      }
      if (event.key.toLowerCase() !== 'k') {
        return;
      }

      event.preventDefault();
      paletteOpen = !paletteOpen;
    };

    window.addEventListener('keydown', onKeyDown);

    const media = createThemeMediaQuery();
    themePreference = readThemePreference();
    resolvedTheme = resolveTheme(themePreference, media?.matches ?? false);
    applyResolvedTheme(resolvedTheme);

    const onThemeChange = (event: MediaQueryListEvent) => {
      if (themePreference !== 'system') {
        return;
      }
      resolvedTheme = resolveTheme('system', event.matches);
      applyResolvedTheme(resolvedTheme);
    };

    if (media) {
      if (typeof media.addEventListener === 'function') {
        media.addEventListener('change', onThemeChange);
      } else {
        media.addListener(onThemeChange);
      }
    }

    void (async () => {
      const nextState = await refreshWorkspaceState();
      if (nextState?.manifest_present) {
        await bootstrapConfigured();
      } else if (nextState?.workspace_root) {
        statusLine = '当前目录没有 manifest。你可以初始化或切换到已有 workspace。';
      } else {
        statusLine = '请选择一个 workspace。';
      }
    })();

    return () => {
      window.removeEventListener('keydown', onKeyDown);
      if (media) {
        if (typeof media.removeEventListener === 'function') {
          media.removeEventListener('change', onThemeChange);
        } else {
          media.removeListener(onThemeChange);
        }
      }
    };
  });

  onDestroy(() => {
    // onMount cleanup covers it; this is a no-op but keeps intent explicit.
  });

  $effect(() => {
    if (!manifestPresent) {
      bottomPanelOpen = false;
      if (linkStatus !== null) {
        linkStatus = null;
      }
      if (impact !== null) {
        impact = null;
      }
      return;
    }

    if (view === 'links' && linkStatus === null && !busy.links) {
      void refreshLinkStatus();
      return;
    }

    if (view === 'impact' && impact === null && !busy.impact) {
      void refreshImpactAnalysis();
    }
  });

  $effect(() => {
    if (!manifestPresent) {
      return;
    }
    if (!errorMessage && (summary?.issues.length ?? 0) === 0) {
      return;
    }
    bottomPanelOpen = true;
    bottomPanelTab = 'problems';
  });
</script>

<div class="frame">
  {#if !manifestPresent}
    <WorkspaceBoot
      workspaceInput={workspaceInput}
      workspaceProbe={workspaceProbe}
      initGit={initGit}
      busy={busy.boot}
      pickerBusy={pickerBusy}
      errorMessage={errorMessage}
      statusLine={statusLine}
      onWorkspaceInput={(next) => (workspaceInput = next)}
      onInitGit={(next) => (initGit = next)}
      onProbeWorkspace={() => void handleProbeWorkspace()}
      onPickWorkspace={() => void handlePickWorkspace()}
      onOpenWorkspace={() => void handleSelectWorkspace()}
      onInitWorkspace={() => void handleInitWorkspace()}
    />
  {:else}
    <div class="workbench" style={`--rail-width:${railExpanded ? '172px' : '56px'}`}>
      <WorkbenchTopbar
        workspaceRoot={workspaceRoot}
        workspaceLabel={workspaceLabel}
        onOpenPalette={() => (paletteOpen = true)}
        onSwitchWorkspace={returnToWorkspaceBoot}
      />

      <WorkbenchRail
        view={view}
        expanded={railExpanded}
        onChange={openDefaultDocument}
        onToggleExpanded={() => (railExpanded = !railExpanded)}
      />

      <div class="workbench-tabs">
        <EditorTabs
          tabs={editorTabModel}
          active={activeDocId}
          ariaLabel="Workbench documents"
          emptyLabel="（未打开任何对象）"
          onChange={activateDocument}
          onClose={closeDocument}
          onReorder={reorderDocuments}
        />
      </div>

      <div class="workbench-view">
        {#snippet currentView()}
          {#if view === 'artifacts'}
            <ArtifactsView
              summary={summary}
              selectedProfile={selectedProfile}
              onSelectProfile={selectProfile}
              onFocusArtifact={focusArtifact}
              onOpenTarget={openTargetInLinks}
              onRefreshWorkspace={bootstrapConfigured}
              onSourceSaved={async () => {
                await Promise.all([refreshWorkspaceGit(), refreshWatchStatus()]);
              }}
              requestedArtifactId={artifactRequestId}
              onRequestHandled={(id) => {
                if (artifactRequestId === id) {
                  artifactRequestId = null;
                }
              }}
              requestedManifestInsert={manifestInsertRequest}
              onManifestInsertHandled={(kind) => {
                if (manifestInsertRequest === kind) {
                  manifestInsertRequest = null;
                }
              }}
              shortcutsEnabled={!paletteOpen}
              setStatusLine={(next) => (statusLine = next)}
              setErrorMessage={(next) => (errorMessage = next)}
            />
          {:else if view === 'links'}
            <LinksView
              targets={targets}
              linkStatus={linkStatus}
              selectedTargetId={selectedTargetId}
              selectedTargets={selectedTargets}
              linkSearch={linkSearch}
              linkUnhealthyOnly={linkUnhealthyOnly}
              linkForce={linkForce}
              linkScope={linkScope}
              linkOp={linkOp}
              linkOpTitle={linkOpTitle}
              activeTarget={activeTarget}
              activeLinkStatus={activeLinkStatus}
              selectedProfile={selectedProfile}
              busyLinks={busy.links}
              busyLinkOp={busy.link_op}
              errorMessage={errorMessage}
              statusLine={statusLine}
              onLinkSearch={(next) => (linkSearch = next)}
              onLinkUnhealthyOnly={(next) => (linkUnhealthyOnly = next)}
              onLinkForce={(next) => (linkForce = next)}
              onLinkScope={(next) => (linkScope = next)}
              onSelectTarget={openTargetDocument}
              onToggleTarget={toggleTargetSelection}
              onRefreshLinkStatus={refreshLinkStatus}
              onCopyToClipboard={copyToClipboard}
              onRunLinkOperation={runLinkOperation}
            />
          {:else if view === 'env'}
            <EnvView
              envSets={envSets}
              selectedEnvSet={selectedEnvSet}
              activeEnvSet={activeEnvSet}
              selectedShell={selectedShell}
              shellChoices={shellChoices}
              envScript={envScript}
              busyEnvEmit={busy.env_emit}
              errorMessage={errorMessage}
              statusLine={statusLine}
              onSelectEnvSet={openEnvDocument}
              onSelectShell={(shell) => (selectedShell = shell)}
              onEnvEmit={handleEnvEmit}
              onCopyToClipboard={copyToClipboard}
              onOpenUsageRef={openEnvUsageRef}
              onOpenManifestEditor={openManifestEditor}
              onCreateManifestObject={requestManifestInsert}
            />
          {:else if view === 'scripts'}
            <ScriptsView
              scripts={scripts}
              selectedScript={selectedScript}
              activeScript={activeScript}
              scriptStdin={scriptStdin}
              scriptRun={scriptRun}
              busyScriptRun={busy.script_run}
              errorMessage={errorMessage}
              statusLine={statusLine}
              onSelectScript={openScriptDocument}
              onScriptStdin={(next) => (scriptStdin = next)}
              onScriptRun={handleScriptRun}
              onCopyToClipboard={copyToClipboard}
              onOpenManifestEditor={openManifestEditor}
              onCreateManifestObject={requestManifestInsert}
            />
          {:else if view === 'impact'}
            <ImpactView
              impactMode={impactMode}
              impact={impact}
              selectedArtifact={selectedArtifact}
              selectedProfile={selectedProfile}
              busyImpact={busy.impact}
              errorMessage={errorMessage}
              statusLine={statusLine}
              onSetImpactMode={setImpactMode}
              onRefreshImpact={refreshImpactAnalysis}
              onOpenTarget={openTargetInLinks}
              onOpenArtifact={requestOpenArtifact}
              onOpenProfile={openProfileInArtifacts}
              onCopyToClipboard={copyToClipboard}
            />
          {:else if view === 'mcp'}
            <McpView
              mcpServers={mcpServers}
              selectedMcpServerId={selectedMcpServerId}
              activeMcpServer={activeMcpServer}
              errorMessage={errorMessage}
              statusLine={statusLine}
              setErrorMessage={(next) => (errorMessage = next)}
              onSelectMcpServer={openMcpDocument}
              onCopyToClipboard={copyToClipboard}
              onOpenManifestEditor={openManifestEditor}
              onCreateManifestObject={requestManifestInsert}
            />
          {/if}
        {/snippet}

        {#if bottomPanelOpen}
          <SplitView
            autoSaveId="workbench:main-bottom"
            direction="vertical"
            initialLeftPct={76}
            minLeftPx={280}
            minRightPx={180}
          >
            {#snippet left()}
              <div class="workbench-view__content">
                {@render currentView()}
              </div>
            {/snippet}

            {#snippet right()}
              <WorkbenchBottomPanel
                activeTab={bottomPanelTab}
                errorMessage={errorMessage}
                issues={summary?.issues ?? []}
                watchStatus={watchStatus}
                busyWatch={busy.watch}
                onSelectTab={(tab) => {
                  bottomPanelTab = tab;
                  if (tab === 'trace') {
                    void refreshTracePanel();
                  }
                }}
                onRefreshTrace={refreshTracePanel}
                onClose={() => (bottomPanelOpen = false)}
              />
            {/snippet}
          </SplitView>
        {:else}
          <div class="workbench-view__content">
            {@render currentView()}
          </div>
        {/if}
      </div>

      <CommandPalette
        open={paletteOpen}
        commands={paletteCommands}
        onClose={() => (paletteOpen = false)}
      />

      <footer class="statusbar" aria-live="polite">
        <div class="statusbar__group statusbar__group--context">
          <span
            class={[
              'statusbar__badge',
              problemCount > 0 ? 'statusbar__badge--error' : 'statusbar__badge--view'
            ].join(' ')}
          >
            {problemCount > 0 ? '问题' : viewLabels[view]}
          </span>
          <span class="statusbar__text" title={errorMessage ?? statusLine}>
            {truncateMiddle(errorMessage ?? statusLine, 120)}
          </span>
        </div>

        <div class="statusbar__group statusbar__group--actions">
          <button
            class="statusbar__button"
            disabled={!manifestPresent}
            type="button"
            onclick={() => void toggleBottomPanel('problems')}
          >
            {bottomPanelOpen && bottomPanelTab === 'problems'
              ? `problems 打开 (${problemCount})`
              : `problems ${problemCount}`}
          </button>
          <button
            class="statusbar__button"
            data-testid="watch-trace-toggle"
            disabled={!manifestPresent}
            type="button"
            onclick={() => void toggleBottomPanel('trace')}
          >
            {bottomPanelOpen && bottomPanelTab === 'trace'
              ? `trace 打开 (${watchTraceCount})`
              : `trace ${watchTraceCount}`}
          </button>
          <button
            class="statusbar__button"
            disabled={busy.summary}
            type="button"
            onclick={() => void bootstrapConfigured()}
          >
            {busy.summary ? '同步中…' : '同步'}
          </button>
        </div>

        <div class="statusbar__group statusbar__group--telemetry">
          <span class={['statusbar__item', `statusbar__item--${watchPill.tone}`].join(' ')}>
            watch <strong class="mono">{watchPill.label}</strong>
          </span>
          <span class="statusbar__item" title={watchActivity}>
            event <strong class="mono">{truncateMiddle(watchActivity, 30)}</strong>
          </span>
          {#if gitInfo}
            <span
              class={[
                'statusbar__item',
                'statusbar__item--git',
                gitInfo.dirty ? 'statusbar__item--dirty' : ''
              ].join(' ')}
              title={gitInfo.repo_root}
            >
              git <strong class="mono">{gitHeadline}</strong>
            </span>
          {/if}
          {#if selectedProfile}
            <span class="statusbar__item">
              profile <strong class="mono">{selectedProfile}</strong>
            </span>
          {/if}
          <div class="statusbar__segment" role="group" aria-label="Theme">
            {#each themeChoices as theme (theme.id)}
              <button
                class={[
                  'statusbar__segment-item',
                  themePreference === theme.id ? 'statusbar__segment-item--active' : ''
                ].join(' ')}
                type="button"
                aria-pressed={themePreference === theme.id}
                onclick={() => setThemePreference(theme.id)}
              >
                {theme.label}
              </button>
            {/each}
          </div>
          <span class="statusbar__item mono">{statusFocus}</span>
          <span class="statusbar__item mono">{workspaceLabel}</span>
        </div>
      </footer>
    </div>
  {/if}
</div>
