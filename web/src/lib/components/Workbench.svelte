<script lang="ts">
  import { onMount } from 'svelte';

  import WorkspaceBoot from '$lib/workbench/WorkspaceBoot.svelte';
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
    emitEnv,
    getImpactAnalysis,
    getLinkStatus,
    getWatchStatus,
    getWorkspaceState,
    getWorkspaceSummary,
    initWorkspace,
    planLinks,
    repairLinks,
    runScript,
    selectWorkspace
  } from '$lib/api/client';
  import type {
    EnvEmitResponse,
    ImpactAnalysisResponse,
    LinkApplyRequest,
    LinkOperationResponse,
    LinkPlanRequest,
    LinkRepairRequest,
    LinkStatusResponseItem,
    McpServerSummaryResponse,
    ScriptRunResponse,
    ShellKindResponse,
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
  type ImpactMode = 'artifact' | 'profile' | 'artifact_profile';

  let view = $state<ViewKey>('artifacts');

  let workspaceState = $state<WorkspaceStateResponse | null>(null);
  let workspaceInput = $state('');
  let initGit = $state(false);

  let summary = $state<WorkspaceSummaryResponse | null>(null);
  let watchStatus = $state<WatchStatusResponse | null>(null);

  let selectedArtifact = $state<string | null>(null);
  let selectedProfile = $state<string | null>(null);
  let selectedEnvSet = $state<string | null>(null);
  let selectedScript = $state<string | null>(null);
  let selectedLinkTargetPath = $state<string | null>(null);
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
  const activeLink = $derived(
    (linkStatus ?? []).find((item) => item.target_path === selectedLinkTargetPath) ?? null
  );
  const activeTargetIdForLink = $derived.by(() => {
    if (!activeLink) {
      return null;
    }
    return targets.find((target) => target.target_path === activeLink.target_path)?.id ?? null;
  });

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

  function returnToWorkspaceBoot(): void {
    workspaceState = {
      workspace_root: workspaceRoot,
      manifest_present: false
    };
    statusLine = '请选择一个 workspace。';
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
      if (!selectedEnvSet) {
        selectedEnvSet = nextSummary.env_sets[0]?.id ?? null;
      }
      if (!selectedScript) {
        selectedScript = nextSummary.scripts[0]?.id ?? null;
      }
      if (selectedMcpServerId && !nextSummary.mcp_servers.some((server) => server.id === selectedMcpServerId)) {
        selectedMcpServerId = nextSummary.mcp_servers[0]?.id ?? null;
      } else if (!selectedMcpServerId) {
        selectedMcpServerId = nextSummary.mcp_servers[0]?.id ?? null;
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
    linkStatus = null;
    selectedLinkTargetPath = null;
    impact = null;
    statusLine = '已连接到 workspace。';
  }

  async function refreshLinkStatus(): Promise<void> {
    busy.links = true;
    errorMessage = null;
    try {
      const items = await getLinkStatus();
      linkStatus = items;
      if (!selectedLinkTargetPath) {
        selectedLinkTargetPath = items[0]?.target_path ?? null;
      } else if (!items.some((item) => item.target_path === selectedLinkTargetPath)) {
        selectedLinkTargetPath = items[0]?.target_path ?? null;
      }
      statusLine = '已刷新 link status。';
    } catch (error) {
      linkStatus = null;
      selectedLinkTargetPath = null;
      errorMessage = describeError(error, '无法读取 link status。');
    } finally {
      busy.links = false;
    }
  }

  async function runLinkOperation(kind: 'plan' | 'apply' | 'repair'): Promise<void> {
    busy.link_op = true;
    errorMessage = null;
    try {
      const selectedTargets: string[] = [];
      if (linkScope === 'selected') {
        if (!activeTargetIdForLink) {
          throw new Error(
            '当前选择的 link 无法映射到 manifest target（可能是旧记录或未刷新 workspace summary）。'
          );
        }
        selectedTargets.push(activeTargetIdForLink);
      }

      const default_profile = selectedProfile ?? null;

      if (kind === 'plan') {
        linkOp = await planLinks({ targets: selectedTargets, default_profile });
        linkOpTitle = 'plan';
        statusLine = `plan 完成（${linkOp.items.length} items）。`;
        return;
      }

      if (kind === 'apply') {
        linkOp = await applyLinks({
          targets: selectedTargets,
          default_profile,
          force: linkForce
        });
        linkOpTitle = 'apply';
        statusLine = `apply 完成（${linkOp.items.length} items）。`;
        await refreshLinkStatus();
        return;
      }

      linkOp = await repairLinks({
        targets: selectedTargets,
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

  async function copyToClipboard(text: string, label: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      statusLine = `已复制${label}到剪贴板。`;
    } catch {
      statusLine = `复制${label}失败（浏览器未授权）。`;
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

  function selectLink(targetPath: string): void {
    selectedLinkTargetPath = targetPath;
    linkOp = null;
    linkOpTitle = null;
    statusLine = `已选择 link：${truncateMiddle(targetPath, 48)}`;
  }

  function selectMcpServer(id: string): void {
    selectedMcpServerId = id;
    statusLine = `已选择 MCP server：${id}`;
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
      linkStatus = null;
      impact = null;
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
</script>

<div class="frame">
  {#if !manifestPresent}
    <WorkspaceBoot
      workspaceInput={workspaceInput}
      initGit={initGit}
      busy={busy.boot}
      errorMessage={errorMessage}
      statusLine={statusLine}
      onWorkspaceInput={(next) => (workspaceInput = next)}
      onInitGit={(next) => (initGit = next)}
      onOpenWorkspace={() => void handleSelectWorkspace()}
      onInitWorkspace={() => void handleInitWorkspace()}
    />
  {:else}
    <div class="workbench">
      <WorkbenchTopbar
        workspaceRoot={workspaceRoot}
        workspaceLabel={workspaceLabel}
        watchPill={watchPill}
        watchActivity={watchActivity}
        busySummary={busy.summary}
        onSwitchWorkspace={returnToWorkspaceBoot}
        onRefresh={bootstrapConfigured}
      />

      <WorkbenchRail view={view} onChange={(next) => (view = next)} />

      {#if view === 'artifacts'}
        <ArtifactsView
          summary={summary}
          selectedProfile={selectedProfile}
          onSelectProfile={selectProfile}
          onFocusArtifact={focusArtifact}
          statusLine={statusLine}
          errorMessage={errorMessage}
          setStatusLine={(next) => (statusLine = next)}
          setErrorMessage={(next) => (errorMessage = next)}
        />
      {:else if view === 'links'}
        <LinksView
          linkStatus={linkStatus}
          selectedLinkTargetPath={selectedLinkTargetPath}
          linkSearch={linkSearch}
          linkUnhealthyOnly={linkUnhealthyOnly}
          linkForce={linkForce}
          linkScope={linkScope}
          linkOp={linkOp}
          linkOpTitle={linkOpTitle}
          activeLink={activeLink}
          activeTargetIdForLink={activeTargetIdForLink}
          selectedProfile={selectedProfile}
          busyLinks={busy.links}
          busyLinkOp={busy.link_op}
          errorMessage={errorMessage}
          statusLine={statusLine}
          onLinkSearch={(next) => (linkSearch = next)}
          onLinkUnhealthyOnly={(next) => (linkUnhealthyOnly = next)}
          onLinkForce={(next) => (linkForce = next)}
          onLinkScope={(next) => (linkScope = next)}
          onSelectLink={selectLink}
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
          onSelectEnvSet={selectEnvSet}
          onSelectShell={(shell) => (selectedShell = shell)}
          onEnvEmit={handleEnvEmit}
          onCopyToClipboard={copyToClipboard}
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
          onSelectScript={selectScript}
          onScriptStdin={(next) => (scriptStdin = next)}
          onScriptRun={handleScriptRun}
          onCopyToClipboard={copyToClipboard}
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
        />
      {:else if view === 'mcp'}
        <McpView
          mcpServers={mcpServers}
          selectedMcpServerId={selectedMcpServerId}
          activeMcpServer={activeMcpServer}
          errorMessage={errorMessage}
          statusLine={statusLine}
          onSelectMcpServer={selectMcpServer}
          onCopyToClipboard={copyToClipboard}
        />
      {/if}
    </div>
  {/if}
</div>
