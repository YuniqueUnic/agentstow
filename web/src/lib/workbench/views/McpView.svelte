<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';

  import type { McpServerSummaryResponse } from '$lib/types';
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
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">MCP</p>
    <p class="explorer__hint">MCP server 应作为工作台对象查看，而不是表单详情页。</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>MCP</span>
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
          <span class="mono">mcp</span>
        </li>
      {:else}
        {#each mcpServers as server (server.id)}
          <li>
            <button
              class={[
                'list__item',
                selectedMcpServerId === server.id ? 'list__item--active' : ''
              ].join(' ')}
              onclick={() => onSelectMcpServer(server.id)}
              type="button"
            >
              <span class="list__dot list__dot--accent" aria-hidden="true"></span>
              <span class="list__name">{server.id}</span>
              <span class="list__meta">{server.transport_kind}</span>
            </button>
          </li>
        {/each}
      {/if}
    </ul>
  </div>
</aside>

<main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>{activeMcpServer?.id ?? '未选择 MCP server'}</strong>
      <span class="muted">{activeMcpServer ? `· ${activeMcpServer.transport_kind}` : '· inspector document'}</span>
    </div>

    <div class="canvas__actions">
      <button class="ui-button ui-button--subtle" type="button" onclick={onOpenManifestEditor}>
        编辑 manifest
      </button>
      <button
        class="ui-button ui-button--ghost"
        disabled={!activeMcpServer}
        type="button"
        onclick={() => void onCopyToClipboard(activeMcpServer?.location ?? '', 'location')}
      >
        复制 location
      </button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="workspace-split surface">
    <SplitView autoSaveId="workbench:mcp:shell" initialLeftPct={66} minLeftPx={420} minRightPx={280}>
      {#snippet left()}
        <section class="region" aria-label="MCP server document">
          <div class="region__header">
            <span>Server Document</span>
            <span class="mono">{activeMcpServer?.transport_kind ?? 'idle'}</span>
          </div>

          <div class="region__body region__body--stack">
            {#if !activeMcpServer}
              <p class="empty">（选择 MCP server 后可查看 transport、location 与环境要求）</p>
            {:else}
              <section>
                <div class="region__header">
                  <span>Transport</span>
                  <button
                    class="ui-button ui-button--ghost"
                    type="button"
                    onclick={() => void onCopyToClipboard(activeMcpServer.id, 'server id')}
                  >
                    复制 id
                  </button>
                </div>
                <div class="panel__body panel__body--flush">
                  <div class="inspector-table">
                    <div class="inspector-row">
                      <span class="inspector-row__label">Id</span>
                      <span class="inspector-row__value inspector-row__value--mono">{activeMcpServer.id}</span>
                    </div>
                    <div class="inspector-row">
                      <span class="inspector-row__label">Kind</span>
                      <span class="inspector-row__value inspector-row__value--mono">{activeMcpServer.transport_kind}</span>
                    </div>
                    <div class="inspector-row">
                      <span class="inspector-row__label">Location</span>
                      <span class="inspector-row__value inspector-row__value--mono">{activeMcpServer.location}</span>
                    </div>
                  </div>
                </div>
              </section>

              <section>
                <div class="region__header">
                  <span>Runtime Requirements</span>
                  <span class="mono">{activeMcpServer.env_keys.length} env</span>
                </div>
                <div class="panel__body panel__body--flush">
                  <div class="inspector-table">
                    <div class="inspector-row">
                      <span class="inspector-row__label">Env Keys</span>
                      <span class="inspector-row__value inspector-row__value--mono">
                        {activeMcpServer.env_keys.length || '0'}
                      </span>
                    </div>
                    <div class="inspector-row">
                      <span class="inspector-row__label">Mode</span>
                      <span class="inspector-row__value">
                        当前先按 manifest 声明展示，后续可以继续在此接 transport args / headers 等细节。
                      </span>
                    </div>
                  </div>
                </div>
              </section>
            {/if}
          </div>
        </section>
      {/snippet}

      {#snippet right()}
        <section class="region secondary-sidebar" aria-label="MCP runtime sidebar">
          <div class="region__header">
            <span>Required Env Keys</span>
            <span class="mono">{activeMcpServer?.env_keys.length ?? 0}</span>
          </div>

          <div class="panel__body panel__body--flush">
            {#if !activeMcpServer}
              <p class="empty empty--flush">（选择 MCP server 后查看环境要求）</p>
            {:else if activeMcpServer.env_keys.length === 0}
              <p class="empty empty--flush">（该 MCP server 未声明 env keys）</p>
            {:else}
              <div class="token-list">
                {#each activeMcpServer.env_keys as key (key)}
                  <span class="token">{key}</span>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      {/snippet}
    </SplitView>
  </div>
</main>
