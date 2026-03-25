<script lang="ts">
  import { untrack } from 'svelte';

  import CodeEditor from '$lib/components/CodeEditor.svelte';
  import SplitView from '$lib/components/SplitView.svelte';

  import { ApiClientError, renderMcpServer, testMcpServer, validateMcpServer } from '$lib/api/client';
  import type {
    McpRenderResponse,
    McpServerSummaryResponse,
    McpTestResponse,
    McpValidateResponse
  } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

  type Props = {
    mcpServers: McpServerSummaryResponse[];
    selectedMcpServerId: string | null;
    activeMcpServer: McpServerSummaryResponse | null;
    errorMessage: string | null;
    statusLine: string;
    setErrorMessage: (next: string | null) => void;
    onSelectMcpServer: (id: string) => void;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onOpenManifestEditor: () => void;
    onCreateManifestObject: (kind: ManifestInsertKind) => void;
  };

  type RenderLoadOptions = {
    resetChecks?: boolean;
  };

  let {
    mcpServers,
    selectedMcpServerId,
    activeMcpServer,
    setErrorMessage,
    onSelectMcpServer,
    onCopyToClipboard,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();

  function buildServerMeta(server: McpServerSummaryResponse): string {
    const details = [`${server.env_bindings.length} env`];
    if (server.headers.length > 0) {
      details.push(`${server.headers.length} headers`);
    }
    if (server.args.length > 0) {
      details.push(`${server.args.length} args`);
    }
    return `${server.transport_kind} · ${details.join(' · ')}`;
  }

  function badgeClass(status: string): string {
    return ['pill', status === 'ok' ? 'pill--ok' : 'pill--warn'].join(' ');
  }

  let renderState = $state<McpRenderResponse | null>(null);
  let validateState = $state<McpValidateResponse | null>(null);
  let testState = $state<McpTestResponse | null>(null);
  let localError = $state<string | null>(null);
  let busyRender = $state(false);
  let busyValidate = $state(false);
  let busyTest = $state(false);
  let renderToken = 0;
  let validateToken = 0;
  let testToken = 0;

  function describeError(error: unknown, fallback: string): string {
    if (error instanceof ApiClientError) {
      return error.message;
    }
    if (error instanceof Error && error.message) {
      return error.message;
    }
    return fallback;
  }

  async function loadRenderState(
    serverId: string | null,
    options: RenderLoadOptions = {}
  ): Promise<void> {
    const token = ++renderToken;

    if (!serverId) {
      renderState = null;
      validateState = null;
      testState = null;
      localError = null;
      busyRender = false;
      return;
    }

    busyRender = true;
    localError = null;
    if (options.resetChecks ?? false) {
      validateState = null;
      testState = null;
    }

    try {
      const rendered = await renderMcpServer(serverId);
      if (token !== renderToken) {
        return;
      }
      renderState = rendered;
    } catch (error) {
      if (token !== renderToken) {
        return;
      }
      renderState = null;
      localError = describeError(error, '无法渲染 MCP 配置。');
    } finally {
      if (token === renderToken) {
        busyRender = false;
      }
    }
  }

  async function refreshRender(): Promise<void> {
    await loadRenderState(activeMcpServer?.id ?? null);
  }

  async function runValidate(): Promise<void> {
    const serverId = activeMcpServer?.id;
    if (!serverId || busyValidate) {
      return;
    }
    const token = ++validateToken;

    busyValidate = true;
    localError = null;
    try {
      const result = await validateMcpServer(serverId);
      if (token !== validateToken) {
        return;
      }
      validateState = result;
    } catch (error) {
      if (token !== validateToken) {
        return;
      }
      validateState = null;
      localError = describeError(error, 'MCP 校验失败。');
    } finally {
      if (token === validateToken) {
        busyValidate = false;
      }
    }
  }

  async function runTest(): Promise<void> {
    const serverId = activeMcpServer?.id;
    if (!serverId || busyTest) {
      return;
    }
    const token = ++testToken;

    busyTest = true;
    localError = null;
    try {
      const result = await testMcpServer(serverId);
      if (token !== testToken) {
        return;
      }
      testState = result;
    } catch (error) {
      if (token !== testToken) {
        return;
      }
      testState = null;
      localError = describeError(error, 'MCP dry-run 测试失败。');
    } finally {
      if (token === testToken) {
        busyTest = false;
      }
    }
  }

  $effect(() => {
    setErrorMessage(localError);
  });

  $effect(() => {
    const serverId = activeMcpServer?.id ?? null;
    untrack(() => {
      void loadRenderState(serverId, { resetChecks: true });
    });
  });

  const launcherPreview = $derived(renderState?.launcher_preview ?? '');
  const renderedConfigPreview = $derived(renderState?.config_json ?? '');
  const runtimeEnvBindings = $derived(renderState?.env_bindings ?? activeMcpServer?.env_bindings ?? []);
  const runtimeAvailableCount = $derived(
    runtimeEnvBindings.reduce((count, binding) => count + (binding.available ? 1 : 0), 0)
  );
  const validateIssueCount = $derived(validateState?.issues.length ?? 0);
  const testCheckCount = $derived(testState?.checks.length ?? 0);
  const testAttentionCount = $derived(
    testState?.checks.reduce((count, check) => count + (check.status === 'ok' ? 0 : 1), 0) ?? 0
  );
</script>

<SplitView autoSaveId="workbench:view:mcp" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">MCP</p>
        <p class="explorer__hint">这里展示的是 manifest 渲染结果、配置校验和 dry-run 检查，不会建立 live runtime 连接。</p>
      </div>

      <div class="explorer__section">
        <div class="section__title">
          <span>MCP Servers</span>
          <strong>{mcpServers.length}</strong>
        </div>
        <div class="chips chips--tight" aria-label="MCP actions">
          <button class="chip" onclick={() => onCreateManifestObject('mcp_server')} type="button">
            新建 MCP
          </button>
          <button class="chip" onclick={onOpenManifestEditor} type="button">
            编辑 manifest
          </button>
        </div>
        <ul class="list">
          {#if mcpServers.length === 0}
            <li class="list__static">
              <span class="muted">（未声明 MCP servers）</span>
              <span class="mono">mcp_servers</span>
            </li>
          {:else}
            {#each mcpServers as server (server.id)}
              <li>
                <button
                  class={['list__item', selectedMcpServerId === server.id ? 'list__item--active' : ''].join(' ')}
                  data-testid={`mcp-server-item:${server.id}`}
                  onclick={() => onSelectMcpServer(server.id)}
                  title={server.location}
                  type="button"
                >
                  <span class="list__dot list__dot--accent" aria-hidden="true"></span>
                  <span class="list__name">{server.id}</span>
                  <span class="list__meta">{buildServerMeta(server)}</span>
                </button>
              </li>
            {/each}
          {/if}
        </ul>
      </div>
    </aside>
  {/snippet}

  {#snippet right()}
    <main class="canvas" aria-label="工作区画布">
      <div class="canvas__head">
        <div class="title">
          <strong>{activeMcpServer?.id ?? '未选择 MCP server'}</strong>
          <span class="muted">
            {activeMcpServer
              ? `· ${activeMcpServer.transport_kind} · ${truncateMiddle(activeMcpServer.location, 68)}`
              : '· server document'}
          </span>
        </div>

        <div class="canvas__actions">
          <button class="ui-button ui-button--subtle" type="button" onclick={onOpenManifestEditor}>
            编辑 manifest
          </button>
          <button
            class="ui-button ui-button--ghost"
            data-testid="mcp-validate-run"
            disabled={!activeMcpServer || busyValidate}
            type="button"
            onclick={() => void runValidate()}
          >
            {busyValidate ? '校验中…' : '校验'}
          </button>
          <button
            class="ui-button ui-button--ghost"
            data-testid="mcp-render-run"
            disabled={!activeMcpServer || busyRender}
            type="button"
            onclick={() => void refreshRender()}
          >
            {busyRender ? '渲染中…' : '重新渲染'}
          </button>
          <button
            class="ui-button ui-button--ghost"
            data-testid="mcp-test-run"
            disabled={!activeMcpServer || busyTest}
            type="button"
            onclick={() => void runTest()}
          >
            {busyTest ? '测试中…' : 'dry-run 测试'}
          </button>
        </div>
      </div>
      <div class="workspace-split surface">
        <SplitView autoSaveId="workbench:mcp:document" initialLeftPct={70} minLeftPx={440} minRightPx={280}>
          {#snippet left()}
            <section class="region" aria-label="MCP config preview">
              <div class="region__header">
                <span>Config Contract</span>
                <span class="mono">{activeMcpServer?.transport_kind ?? 'idle'}</span>
              </div>

              <div class="region__body">
                {#if !activeMcpServer}
                  <p class="empty">（选择 MCP server 后可查看 transport、launcher preview 与渲染后的 config）</p>
                {:else}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Transport Contract</span>
                      <strong>{activeMcpServer.transport_kind}</strong>
                    </div>

                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Id</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeMcpServer.id}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Location</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeMcpServer.location}</span>
                      </div>
                      {#if activeMcpServer.transport_kind === 'stdio'}
                        <div class="inspector-row">
                          <span class="inspector-row__label">Command</span>
                          <span class="inspector-row__value inspector-row__value--mono">
                            {activeMcpServer.command ?? '—'}
                          </span>
                        </div>
                        <div class="inspector-row">
                          <span class="inspector-row__label">Args</span>
                          <span class="inspector-row__value inspector-row__value--mono">
                            {activeMcpServer.args.length > 0 ? activeMcpServer.args.join(' ') : '—'}
                          </span>
                        </div>
                      {:else}
                        <div class="inspector-row">
                          <span class="inspector-row__label">URL</span>
                          <span class="inspector-row__value inspector-row__value--mono">
                            {activeMcpServer.url ?? activeMcpServer.location}
                          </span>
                        </div>
                        <div class="inspector-row">
                          <span class="inspector-row__label">Headers</span>
                          <span class="inspector-row__value inspector-row__value--mono">
                            {activeMcpServer.headers.length}
                          </span>
                        </div>
                        {#each activeMcpServer.headers as header (header.key)}
                          <div class="inspector-row">
                            <span class="inspector-row__label">{header.key}</span>
                            <span class="inspector-row__value inspector-row__value--mono">{header.value}</span>
                          </div>
                        {/each}
                      {/if}
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Launcher</span>
                      <strong>{busyRender ? 'loading' : activeMcpServer.transport_kind === 'stdio' ? 'spawn' : 'http'}</strong>
                    </div>

                    <div class="terminal">
                      {#key launcherPreview || `launcher:${busyRender ? 'busy' : 'idle'}`}
                        <CodeEditor
                          value={launcherPreview || (busyRender ? '正在重新渲染 launcher preview…' : '（暂无 launcher preview）')}
                          readonly={true}
                          documentLanguage="shell"
                          testId="mcp-launcher-preview"
                        />
                      {/key}
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Rendered Config Preview</span>
                      <strong>{busyRender ? 'loading' : 'json'}</strong>
                    </div>

                    <div class="terminal">
                      {#key renderedConfigPreview || `config:${busyRender ? 'busy' : 'idle'}`}
                        <CodeEditor
                          value={renderedConfigPreview || (busyRender ? '正在重新渲染 MCP config…' : '（暂无 rendered config preview）')}
                          readonly={true}
                          documentLanguage="json"
                          testId="mcp-rendered-config"
                        />
                      {/key}
                    </div>
                  </div>
                {/if}
              </div>
            </section>
          {/snippet}

          {#snippet right()}
            <section class="region secondary-sidebar" aria-label="MCP config sidebar">
              <div class="region__header">
                <span>Config &amp; Dry-run</span>
                <span class="mono">{runtimeEnvBindings.length}</span>
              </div>

              <div class="region__body">
                {#if !activeMcpServer}
                  <p class="empty empty--flush">（选择 MCP server 后查看 dry-run 检查、env 依赖和可复制输出）</p>
                {:else}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Preview Contract</span>
                      <strong>{activeMcpServer.transport_kind}</strong>
                    </div>
                    <div class="subject-summary">
                      <div class="summary-row">
                        <span class="summary-row__label">Ready</span>
                        <span class="summary-row__value mono">
                          {runtimeAvailableCount}/{runtimeEnvBindings.length}
                        </span>
                      </div>
                      <div class="summary-row">
                        <span class="summary-row__label">Headers</span>
                        <span class="summary-row__value mono">{activeMcpServer.headers.length}</span>
                      </div>
                      <div class="summary-row">
                        <span class="summary-row__label">Args</span>
                        <span class="summary-row__value mono">{activeMcpServer.args.length}</span>
                      </div>
                      <div class="summary-row">
                      <span class="summary-row__label">Validate</span>
                      <span class="summary-row__value mono">
                          {validateState ? (validateIssueCount === 0 && validateState.ok ? 'ok' : `${validateIssueCount} issue`) : 'idle'}
                      </span>
                    </div>
                      <div class="summary-row">
                        <span class="summary-row__label">Dry-run</span>
                        <span class="summary-row__value mono">
                          {testState ? (testAttentionCount === 0 && testState.ok ? 'ok' : `${testAttentionCount} attention`) : 'idle'}
                        </span>
                      </div>
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Validate</span>
                      <strong>{validateState ? (validateState.ok && validateIssueCount === 0 ? 'ok' : 'attention') : 'idle'}</strong>
                    </div>

                    {#if busyValidate}
                      <p class="empty empty--flush">（正在执行 validate，请稍候…）</p>
                    {:else if !validateState}
                      <p class="empty empty--flush">（尚未执行 validate，可先点击顶部“校验”）</p>
                    {:else if validateState.issues.length === 0}
                      <p class="empty empty--flush">（校验通过，未发现问题）</p>
                    {:else}
                      <ul class="result-list" aria-label="MCP validate issues" data-testid="mcp-validate-issues">
                        {#each validateState.issues as issue (`${issue.code}:${issue.subject_id}:${issue.message}`)}
                          <li class="result-row">
                            <span class={badgeClass(issue.severity === 'info' ? 'ok' : 'warn')}>
                              {issue.severity}
                            </span>
                            <div class="result-row__main">
                              <span class="result-row__title">{issue.message}</span>
                              <span class="result-row__detail mono">
                                {issue.scope} · {issue.subject_id} · {issue.code}
                              </span>
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Test</span>
                      <strong>{testState ? (testState.ok && testAttentionCount === 0 ? 'ok' : 'attention') : 'idle'}</strong>
                    </div>

                    {#if busyTest}
                      <p class="empty empty--flush">（正在执行 dry-run 测试，请稍候…）</p>
                    {:else if !testState}
                      <p class="empty empty--flush">（尚未执行 dry-run 测试）</p>
                    {:else if testCheckCount === 0}
                      <p class="empty empty--flush">（dry-run 未返回 checks）</p>
                    {:else}
                      <ul class="result-list" aria-label="MCP test checks" data-testid="mcp-test-checks">
                        {#each testState.checks as check (`${check.code}:${check.message}`)}
                          <li class="result-row">
                            <span class={badgeClass(check.status)}>{check.status}</span>
                            <div class="result-row__main">
                              <span class="result-row__title">{check.message}</span>
                              <span class="result-row__detail mono">{check.code}</span>
                              {#if check.detail}
                                <span class="result-row__detail mono">{check.detail}</span>
                              {/if}
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Env Bindings</span>
                      <strong>{runtimeEnvBindings.length}</strong>
                    </div>

                    {#if runtimeEnvBindings.length === 0}
                      <p class="empty empty--flush">（该 MCP server 未声明 env bindings）</p>
                    {:else}
                      <ul class="result-list" aria-label="MCP env bindings" data-testid="mcp-env-bindings">
                        {#each runtimeEnvBindings as binding (binding.key)}
                          <li class="result-row">
                            <span class={['pill', binding.available ? 'pill--ok' : 'pill--warn'].join(' ')}>
                              {binding.available ? 'ready' : 'missing'}
                            </span>
                            <div class="result-row__main">
                              <span class="result-row__title">{binding.key}</span>
                              <span class="result-row__detail mono">
                                {binding.binding_kind} · {binding.binding} · {binding.rendered_placeholder}
                              </span>
                              {#if binding.source_env_var}
                                <span class="result-row__detail mono">
                                  source env: {binding.source_env_var}
                                </span>
                              {/if}
                              {#if binding.diagnostic}
                                <span class="result-row__detail">{binding.diagnostic}</span>
                              {/if}
                              {#if binding.referrers.length > 0}
                                <span class="result-row__detail mono">
                                  refs: {binding.referrers.map((ref) => `${ref.owner_kind}:${ref.owner_id}`).join(', ')}
                                </span>
                              {/if}
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Copy Outputs</span>
                      <strong>copy</strong>
                    </div>
                    <div class="chips chips--tight">
                      <button
                        class="chip"
                        disabled={!launcherPreview}
                        type="button"
                        onclick={() => void onCopyToClipboard(launcherPreview, 'launcher command')}
                      >
                        复制 launcher
                      </button>
                      <button
                        class="chip"
                        disabled={activeMcpServer.transport_kind !== 'http'}
                        type="button"
                        onclick={() => void onCopyToClipboard(activeMcpServer.url ?? '', 'MCP URL')}
                      >
                        复制 URL
                      </button>
                      <button
                        class="chip"
                        disabled={!renderedConfigPreview}
                        type="button"
                        onclick={() => void onCopyToClipboard(renderedConfigPreview, 'MCP JSON')}
                      >
                        复制 JSON
                      </button>
                    </div>
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
