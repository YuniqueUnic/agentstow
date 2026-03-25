<script lang="ts">
  import type {
    EnvEmitResponse,
    EnvUsageRefResponse,
    ImpactAnalysisResponse,
    LinkOperationResponse,
    LinkStatusResponseItem,
    ScriptRunResponse,
    ShellKindResponse,
    WorkspaceSummaryResponse
  } from '$lib/types';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

  import ArtifactsView from '$lib/workbench/views/ArtifactsView.svelte';
  import EnvView from '$lib/workbench/views/EnvView.svelte';
  import ImpactView from '$lib/workbench/views/ImpactView.svelte';
  import LinksView from '$lib/workbench/views/LinksView.svelte';
  import McpView from '$lib/workbench/views/McpView.svelte';
  import ScriptsView from '$lib/workbench/views/ScriptsView.svelte';

  type ViewKey = 'artifacts' | 'links' | 'env' | 'scripts' | 'mcp' | 'impact';
  type ImpactMode = 'artifact' | 'profile' | 'artifact_profile';
  type LinkScope = 'selected' | 'all';
  type LinkOperationKind = 'plan' | 'apply' | 'repair';

  type Props = {
    view: ViewKey;
    summary: WorkspaceSummaryResponse | null;
    selectedProfile: string | null;
    onSelectProfile: (id: string) => void;
    onFocusArtifact: (id: string | null) => void;
    onOpenTarget: (id: string) => void;
    onRefreshWorkspace: () => Promise<void>;
    onSourceSaved: () => Promise<void>;
    requestedArtifactId: string | null;
    onRequestHandled: (id: string) => void;
    requestedManifestInsert: ManifestInsertKind | null;
    onManifestInsertHandled: (kind: ManifestInsertKind) => void;
    shortcutsEnabled: boolean;
    setStatusLine: (next: string) => void;
    setErrorMessage: (next: string | null) => void;
    targets: WorkspaceSummaryResponse['targets'];
    linkStatus: LinkStatusResponseItem[] | null;
    selectedTargetId: string | null;
    selectedTargets: string[];
    linkSearch: string;
    linkUnhealthyOnly: boolean;
    linkForce: boolean;
    linkScope: LinkScope;
    linkOp: LinkOperationResponse | null;
    linkOpTitle: string | null;
    activeTarget: WorkspaceSummaryResponse['targets'][number] | null;
    activeLinkStatus: LinkStatusResponseItem | null;
    busyLinks: boolean;
    busyLinkOp: boolean;
    onLinkSearch: (next: string) => void;
    onLinkUnhealthyOnly: (next: boolean) => void;
    onLinkForce: (next: boolean) => void;
    onLinkScope: (next: LinkScope) => void;
    onSelectTarget: (id?: string | null) => void;
    onToggleTarget: (id: string) => void;
    onRefreshLinkStatus: (announce?: boolean) => Promise<void>;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onRunLinkOperation: (kind: LinkOperationKind) => Promise<void>;
    envSets: WorkspaceSummaryResponse['env_emit_sets'];
    selectedEnvSet: string | null;
    activeEnvSet: WorkspaceSummaryResponse['env_emit_sets'][number] | null;
    selectedShell: ShellKindResponse;
    shellChoices: ShellKindResponse[];
    envScript: EnvEmitResponse | null;
    busyEnvEmit: boolean;
    onSelectEnvSet: (id?: string | null) => void;
    onSelectShell: (shell: ShellKindResponse) => void;
    onEnvEmit: () => Promise<void>;
    onOpenUsageRef: (ref: EnvUsageRefResponse) => void;
    onOpenManifestEditor: () => void;
    onCreateManifestObject: (kind: ManifestInsertKind) => void;
    scripts: WorkspaceSummaryResponse['scripts'];
    selectedScript: string | null;
    activeScript: WorkspaceSummaryResponse['scripts'][number] | null;
    scriptStdin: string;
    scriptRun: ScriptRunResponse | null;
    busyScriptRun: boolean;
    onSelectScript: (id?: string | null) => void;
    onScriptStdin: (next: string) => void;
    onScriptRun: () => Promise<void>;
    impactMode: ImpactMode;
    impact: ImpactAnalysisResponse | null;
    selectedArtifact: string | null;
    busyImpact: boolean;
    onSetImpactMode: (next: ImpactMode) => void;
    onRefreshImpact: () => Promise<void>;
    onOpenArtifact: (id: string) => void;
    onOpenProfile: (id: string) => void;
    mcpServers: WorkspaceSummaryResponse['mcp_servers'];
    selectedMcpServerId: string | null;
    activeMcpServer: WorkspaceSummaryResponse['mcp_servers'][number] | null;
    onSelectMcpServer: (id?: string | null) => void;
    errorMessage: string | null;
    statusLine: string;
  };

  let {
    view,
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
    setStatusLine,
    setErrorMessage,
    targets,
    linkStatus,
    selectedTargetId,
    selectedTargets,
    linkSearch,
    linkUnhealthyOnly,
    linkForce,
    linkScope,
    linkOp,
    linkOpTitle,
    activeTarget,
    activeLinkStatus,
    busyLinks,
    busyLinkOp,
    onLinkSearch,
    onLinkUnhealthyOnly,
    onLinkForce,
    onLinkScope,
    onSelectTarget,
    onToggleTarget,
    onRefreshLinkStatus,
    onCopyToClipboard,
    onRunLinkOperation,
    envSets,
    selectedEnvSet,
    activeEnvSet,
    selectedShell,
    shellChoices,
    envScript,
    busyEnvEmit,
    onSelectEnvSet,
    onSelectShell,
    onEnvEmit,
    onOpenUsageRef,
    onOpenManifestEditor,
    onCreateManifestObject,
    scripts,
    selectedScript,
    activeScript,
    scriptStdin,
    scriptRun,
    busyScriptRun,
    onSelectScript,
    onScriptStdin,
    onScriptRun,
    impactMode,
    impact,
    selectedArtifact,
    busyImpact,
    onSetImpactMode,
    onRefreshImpact,
    onOpenArtifact,
    onOpenProfile,
    mcpServers,
    selectedMcpServerId,
    activeMcpServer,
    onSelectMcpServer,
    errorMessage,
    statusLine
  }: Props = $props();
</script>

{#if view === 'artifacts'}
  <ArtifactsView
    {summary}
    {selectedProfile}
    onSelectProfile={onSelectProfile}
    onFocusArtifact={onFocusArtifact}
    onOpenTarget={onOpenTarget}
    onRefreshWorkspace={onRefreshWorkspace}
    onSourceSaved={onSourceSaved}
    {requestedArtifactId}
    onRequestHandled={onRequestHandled}
    requestedManifestInsert={requestedManifestInsert}
    onManifestInsertHandled={onManifestInsertHandled}
    {shortcutsEnabled}
    {setStatusLine}
    {setErrorMessage}
  />
{:else if view === 'links'}
  <LinksView
    {targets}
    {linkStatus}
    {selectedTargetId}
    {selectedTargets}
    {linkSearch}
    {linkUnhealthyOnly}
    {linkForce}
    {linkScope}
    {linkOp}
    {linkOpTitle}
    {activeTarget}
    {activeLinkStatus}
    {selectedProfile}
    {busyLinks}
    busyLinkOp={busyLinkOp}
    {errorMessage}
    {statusLine}
    onLinkSearch={onLinkSearch}
    onLinkUnhealthyOnly={onLinkUnhealthyOnly}
    onLinkForce={onLinkForce}
    onLinkScope={onLinkScope}
    onSelectTarget={onSelectTarget}
    onToggleTarget={onToggleTarget}
    onRefreshLinkStatus={onRefreshLinkStatus}
    onCopyToClipboard={onCopyToClipboard}
    onRunLinkOperation={onRunLinkOperation}
  />
{:else if view === 'env'}
  <EnvView
    envSets={envSets}
    {selectedEnvSet}
    activeEnvSet={activeEnvSet}
    {selectedShell}
    {shellChoices}
    envScript={envScript}
    busyEnvEmit={busyEnvEmit}
    {errorMessage}
    {statusLine}
    onSelectEnvSet={onSelectEnvSet}
    onSelectShell={onSelectShell}
    onEnvEmit={onEnvEmit}
    onCopyToClipboard={onCopyToClipboard}
    onOpenUsageRef={onOpenUsageRef}
    onOpenManifestEditor={onOpenManifestEditor}
    onCreateManifestObject={onCreateManifestObject}
  />
{:else if view === 'scripts'}
  <ScriptsView
    {scripts}
    {selectedScript}
    activeScript={activeScript}
    {scriptStdin}
    scriptRun={scriptRun}
    busyScriptRun={busyScriptRun}
    {errorMessage}
    {statusLine}
    onSelectScript={onSelectScript}
    onScriptStdin={onScriptStdin}
    onScriptRun={onScriptRun}
    onCopyToClipboard={onCopyToClipboard}
    onOpenManifestEditor={onOpenManifestEditor}
    onCreateManifestObject={onCreateManifestObject}
  />
{:else if view === 'impact'}
  <ImpactView
    {impactMode}
    {impact}
    {selectedArtifact}
    {selectedProfile}
    busyImpact={busyImpact}
    {errorMessage}
    {statusLine}
    onSetImpactMode={onSetImpactMode}
    onRefreshImpact={onRefreshImpact}
    onOpenTarget={onOpenTarget}
    onOpenArtifact={onOpenArtifact}
    onOpenProfile={onOpenProfile}
  />
{:else}
  <McpView
    mcpServers={mcpServers}
    {selectedMcpServerId}
    activeMcpServer={activeMcpServer}
    {errorMessage}
    {statusLine}
    {setErrorMessage}
    onSelectMcpServer={onSelectMcpServer}
    onCopyToClipboard={onCopyToClipboard}
    onOpenManifestEditor={onOpenManifestEditor}
    onCreateManifestObject={onCreateManifestObject}
  />
{/if}
