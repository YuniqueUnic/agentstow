<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';
  import type { McpServerSummaryResponse } from '$lib/types';

  type Props = {
    mcpServers: McpServerSummaryResponse[];
    selectedMcpServerId: string | null;
    activeMcpServer: McpServerSummaryResponse | null;
    errorMessage: string | null;
    statusLine: string;
    onSelectMcpServer: (id: string) => void;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
  };

  let {
    mcpServers,
    selectedMcpServerId,
    activeMcpServer,
    errorMessage,
    statusLine,
    onSelectMcpServer,
    onCopyToClipboard
  }: Props = $props();

  function activateOnKey(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }
    event.preventDefault();
    action();
  }
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">MCP</p>
    <p class="explorer__hint">选择 server 后查看 transport 与所需 env keys</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>MCP</span>
      <strong>{mcpServers.length}</strong>
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
      <span class="muted">{activeMcpServer ? `· ${activeMcpServer.transport_kind}` : ''}</span>
    </div>

    <div class="canvas__actions">
      <md-outlined-button
        disabled={!activeMcpServer}
        onclick={() => void onCopyToClipboard(activeMcpServer?.location ?? '', 'location')}
        onkeydown={(event) =>
          activateOnKey(event, () => void onCopyToClipboard(activeMcpServer?.location ?? '', 'location'))}
        role="button"
        tabindex="0"
      >
        复制 location
      </md-outlined-button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={44} minLeftPx={340} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Details</div>
          <div class="pane__body">
            {#if !activeMcpServer}
              <p class="muted">（请选择一个 MCP server）</p>
            {:else}
              <div class="meta">
                <div class="meta__row">
                  <span class="meta__label">Kind</span>
                  <span class="meta__value mono">{activeMcpServer.transport_kind}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Location</span>
                  <span class="meta__value mono">{activeMcpServer.location}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Env Keys</span>
                  <span class="meta__value mono">{activeMcpServer.env_keys.length}</span>
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Env Keys</div>
          <div class="pane__body">
            {#if !activeMcpServer}
              <p class="muted">（暂无数据）</p>
            {:else if activeMcpServer.env_keys.length === 0}
              <p class="muted">（该 MCP server 未声明 env keys）</p>
            {:else}
              <ul class="kv">
                {#each activeMcpServer.env_keys as key (key)}
                  <li class="kv__row">
                    <span class="kv__key">{key}</span>
                    <span class="kv__value">required</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>

