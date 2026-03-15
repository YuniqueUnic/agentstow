<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';

  import type { McpServerSummaryResponse } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';
  import type { ManifestInsertKind } from '$lib/workbench/manifest_snippets';

  type Props = {
    mcpServers: McpServerSummaryResponse[];
    selectedMcpServerId: string | null;
    activeMcpServer: McpServerSummaryResponse | null;
    errorMessage: string | null;
    statusLine: string;
    onSelectMcpServer: (id: string) => void;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onOpenManifestEditor: () => void;
    onCreateManifestObject: (kind: ManifestInsertKind) => void;
  };

  let {
    mcpServers,
    selectedMcpServerId,
    activeMcpServer,
    errorMessage,
    statusLine,
    onSelectMcpServer,
    onCopyToClipboard,
    onOpenManifestEditor,
    onCreateManifestObject
  }: Props = $props();

  function quoteShellArg(part: string): string {
    return /[\s"'\\]/.test(part) ? JSON.stringify(part) : part;
  }

  function summarizeBinding(binding: string): string {
    if (binding.startsWith('env:')) {
      return `\${${binding.slice(4)}}`;
    }
    if (binding === 'literal') {
      return '<literal>';
    }
    return `<${binding}>`;
  }

  function buildLauncherPreview(server: McpServerSummaryResponse | null): string {
    if (!server) {
      return '';
    }

    if (server.transport_kind === 'stdio') {
      const parts = [server.command ?? server.location, ...server.args].filter(
        (part): part is string => Boolean(part)
      );
      return parts.map((part) => quoteShellArg(part)).join(' ');
    }

    return [
      `GET ${server.url ?? server.location}`,
      server.headers.map((header) => `${header.key}: ${header.value}`).join('\n')
    ]
      .filter(Boolean)
      .join('\n');
  }

  function buildConfigPreview(server: McpServerSummaryResponse | null): string {
    if (!server) {
      return '';
    }

    const env = Object.fromEntries(
      server.env_bindings.map((binding) => [binding.key, summarizeBinding(binding.binding)])
    );

    const payload =
      server.transport_kind === 'stdio'
        ? {
            command: server.command ?? server.location,
            args: server.args,
            env
          }
        : {
            type: 'http',
            url: server.url ?? server.location,
            headers: Object.fromEntries(server.headers.map((header) => [header.key, header.value])),
            env
          };

    return JSON.stringify(
      {
        mcpServers: {
          [server.id]: payload
        }
      },
      null,
      2
    );
  }

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

  const launcherPreview = $derived.by(() => buildLauncherPreview(activeMcpServer));
  const renderedConfigPreview = $derived.by(() => buildConfigPreview(activeMcpServer));
</script>

<SplitView autoSaveId="workbench:view:mcp" initialLeftPct={22} minLeftPx={256} minRightPx={760}>
  {#snippet left()}
    <aside class="explorer surface" aria-label="资源面板">
      <div class="explorer__head">
        <p class="explorer__eyebrow">MCP</p>
        <p class="explorer__hint">MCP server 需要像请求文档一样被检查，而不是停留在摘要 inspector。</p>
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
            disabled={!activeMcpServer}
            type="button"
            onclick={() => void onCopyToClipboard(activeMcpServer?.id ?? '', 'MCP id')}
          >
            复制 id
          </button>
        </div>
      </div>

      {#if errorMessage}
        <p class="notice notice--error">{errorMessage}</p>
      {/if}
      <p class="status-line" aria-live="polite">{statusLine}</p>

      <div class="workspace-split surface">
        <SplitView autoSaveId="workbench:mcp:document" initialLeftPct={70} minLeftPx={440} minRightPx={280}>
          {#snippet left()}
            <section class="region" aria-label="MCP server document">
              <div class="region__header">
                <span>Server Document</span>
                <span class="mono">{activeMcpServer?.transport_kind ?? 'idle'}</span>
              </div>

              <div class="region__body">
                {#if !activeMcpServer}
                  <p class="empty">（选择 MCP server 后可查看 transport、launcher 与 config preview）</p>
                {:else}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Transport</span>
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
                      <strong>{activeMcpServer.transport_kind === 'stdio' ? 'spawn' : 'http'}</strong>
                    </div>

                    <div class="terminal">
                      <pre class="terminal__screen">{launcherPreview}</pre>
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Rendered Config Preview</span>
                      <strong>json</strong>
                    </div>

                    <div class="terminal">
                      <pre class="terminal__screen">{renderedConfigPreview}</pre>
                    </div>
                  </div>
                {/if}
              </div>
            </section>
          {/snippet}

          {#snippet right()}
            <section class="region secondary-sidebar" aria-label="MCP runtime sidebar">
              <div class="region__header">
                <span>Env Bindings</span>
                <span class="mono">{activeMcpServer?.env_bindings.length ?? 0}</span>
              </div>

              <div class="panel__body panel__body--flush">
                {#if !activeMcpServer}
                  <p class="empty empty--flush">（选择 MCP server 后查看环境绑定和快捷操作）</p>
                {:else}
                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Runtime</span>
                      <strong>{activeMcpServer.transport_kind}</strong>
                    </div>
                    <div class="subject-summary">
                      <div class="summary-row">
                        <span class="summary-row__label">Env</span>
                        <span class="summary-row__value mono">{activeMcpServer.env_bindings.length}</span>
                      </div>
                      <div class="summary-row">
                        <span class="summary-row__label">Headers</span>
                        <span class="summary-row__value mono">{activeMcpServer.headers.length}</span>
                      </div>
                      <div class="summary-row">
                        <span class="summary-row__label">Args</span>
                        <span class="summary-row__value mono">{activeMcpServer.args.length}</span>
                      </div>
                    </div>
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Env Bindings</span>
                      <strong>{activeMcpServer.env_bindings.length}</strong>
                    </div>

                    {#if activeMcpServer.env_bindings.length === 0}
                      <p class="empty empty--flush">（该 MCP server 未声明 env bindings）</p>
                    {:else}
                      <div class="inspector-table">
                        {#each activeMcpServer.env_bindings as binding (binding.key)}
                          <div class="inspector-row">
                            <span class="inspector-row__label">{binding.key}</span>
                            <span class="inspector-row__value inspector-row__value--mono">
                              {binding.binding}
                            </span>
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>

                  <div class="inspector-section">
                    <div class="section__title">
                      <span>Quick Actions</span>
                      <strong>copy</strong>
                    </div>
                    <div class="chips chips--tight">
                      <button class="chip" type="button" onclick={() => void onCopyToClipboard(activeMcpServer.id, 'MCP id')}>
                        复制 id
                      </button>
                      <button
                        class="chip"
                        disabled={activeMcpServer.transport_kind !== 'stdio'}
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
