<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';
  import type { LinkOperationResponse, LinkStatusResponseItem } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';

  type Props = {
    linkStatus: LinkStatusResponseItem[] | null;
    selectedLinkTargetPath: string | null;
    linkSearch: string;
    linkUnhealthyOnly: boolean;
    linkForce: boolean;
    linkScope: 'selected' | 'all';
    linkOp: LinkOperationResponse | null;
    linkOpTitle: string | null;
    activeLink: LinkStatusResponseItem | null;
    activeTargetIdForLink: string | null;
    selectedProfile: string | null;
    busyLinks: boolean;
    busyLinkOp: boolean;
    errorMessage: string | null;
    statusLine: string;
    onLinkSearch: (next: string) => void;
    onLinkUnhealthyOnly: (next: boolean) => void;
    onLinkForce: (next: boolean) => void;
    onLinkScope: (next: 'selected' | 'all') => void;
    onSelectLink: (targetPath: string) => void;
    onRefreshLinkStatus: () => Promise<void>;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onRunLinkOperation: (kind: 'plan' | 'apply' | 'repair') => Promise<void>;
  };

  let {
    linkStatus,
    selectedLinkTargetPath,
    linkSearch,
    linkUnhealthyOnly,
    linkForce,
    linkScope,
    linkOp,
    linkOpTitle,
    activeLink,
    activeTargetIdForLink,
    selectedProfile,
    busyLinks,
    busyLinkOp,
    errorMessage,
    statusLine,
    onLinkSearch,
    onLinkUnhealthyOnly,
    onLinkForce,
    onLinkScope,
    onSelectLink,
    onRefreshLinkStatus,
    onCopyToClipboard,
    onRunLinkOperation
  }: Props = $props();

  const filteredLinks = $derived.by(() => {
    const items = linkStatus ?? [];
    const q = linkSearch.trim().toLowerCase();
    return items.filter((item) => {
      if (linkUnhealthyOnly && item.ok) {
        return false;
      }
      if (!q) {
        return true;
      }
      return (
        item.artifact_id.toLowerCase().includes(q) ||
        item.profile.toLowerCase().includes(q) ||
        item.target_path.toLowerCase().includes(q) ||
        item.method.toLowerCase().includes(q) ||
        item.message.toLowerCase().includes(q)
      );
    });
  });

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
    <p class="explorer__eyebrow">LINKS</p>
    <p class="explorer__hint">选择实例后在右侧查看详情与操作</p>
  </div>

  <div class="explorer__section">
    <div class="section__title">
      <span>Links</span>
      <strong>{filteredLinks.length}</strong>
    </div>

    <md-outlined-text-field
      label="搜索 links"
      placeholder="artifact/profile/path…"
      value={linkSearch}
      oninput={(event) => {
        const target = event.currentTarget as { value?: string } | null;
        onLinkSearch(typeof target?.value === 'string' ? target.value : '');
      }}
    ></md-outlined-text-field>

    <div class="toggle" role="group" aria-label="Links 过滤">
      <md-checkbox
        checked={linkUnhealthyOnly}
        onchange={(event: Event) => {
          const target = event.target as unknown as { checked?: unknown } | null;
          onLinkUnhealthyOnly(Boolean(target?.checked));
        }}
        aria-label="仅显示不健康项"
      ></md-checkbox>
      <span>仅显示不健康项</span>
    </div>

    <ul class="list">
      {#if busyLinks && !linkStatus}
        <li class="list__static">
          <span class="muted">读取中…</span>
          <span class="mono">/api/link-status</span>
        </li>
      {:else if filteredLinks.length === 0}
        <li class="list__static">
          <span class="muted">（暂无 link 记录）</span>
          <span class="mono">link</span>
        </li>
      {:else}
        {#each filteredLinks as item (item.target_path)}
          <li>
            <button
              class={[
                'list__item',
                selectedLinkTargetPath === item.target_path ? 'list__item--active' : ''
              ].join(' ')}
              onclick={() => onSelectLink(item.target_path)}
              type="button"
              title={item.target_path}
            >
              <span
                class={['list__dot', item.ok ? 'list__dot--ok' : 'list__dot--bad'].join(' ')}
                aria-hidden="true"
              ></span>
              <span class="list__name">{truncateMiddle(item.target_path, 28)}</span>
              <span class="list__meta">{item.artifact_id}@{item.profile}</span>
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
      <strong>Links</strong>
      <span class="muted">{linkStatus ? `· ${linkStatus.length} instances` : ''}</span>
    </div>

    <div class="canvas__actions">
      <md-outlined-button
        disabled={busyLinks}
        onclick={() => void onRefreshLinkStatus()}
        onkeydown={(event) => activateOnKey(event, () => void onRefreshLinkStatus())}
        role="button"
        tabindex="0"
      >
        {busyLinks ? '刷新中…' : '刷新 status'}
      </md-outlined-button>
      <md-filled-tonal-button
        disabled={!activeLink}
        onclick={() => void onCopyToClipboard(activeLink?.target_path ?? '', '目标路径')}
        onkeydown={(event) =>
          activateOnKey(event, () =>
            void onCopyToClipboard(activeLink?.target_path ?? '', '目标路径')
          )}
        role="button"
        tabindex="0"
      >
        复制路径
      </md-filled-tonal-button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={46} minLeftPx={360} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Instances</div>
          <div class="pane__body">
            <ul class="list">
              {#if !linkStatus}
                <li class="list__static">
                  <span class="muted">（尚未加载 link status）</span>
                  <span class="mono">/api/link-status</span>
                </li>
              {:else if linkStatus.length === 0}
                <li class="list__static">
                  <span class="muted">（暂无 link 实例）</span>
                  <span class="mono">link</span>
                </li>
              {:else}
                {#each linkStatus as item (item.target_path)}
                  <li>
                    <button
                      class={[
                        'list__item',
                        selectedLinkTargetPath === item.target_path ? 'list__item--active' : ''
                      ].join(' ')}
                      onclick={() => onSelectLink(item.target_path)}
                      type="button"
                      title={item.target_path}
                    >
                      <span
                        class={[
                          'list__dot',
                          item.ok ? 'list__dot--ok' : 'list__dot--bad'
                        ].join(' ')}
                        aria-hidden="true"
                      ></span>
                      <span class="list__name">{item.artifact_id}@{item.profile}</span>
                      <span class="list__meta">{item.method}</span>
                    </button>
                  </li>
                {/each}
              {/if}
            </ul>
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Details</div>
          <div class="pane__body">
            {#if !activeLink}
              <p class="muted">（请选择一个 link 实例）</p>
            {:else}
              <div class="meta">
                <div class="meta__row">
                  <span class="meta__label">Status</span>
                  <span class="meta__value">
                    <span class={['pill', activeLink.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                      {activeLink.ok ? 'healthy' : 'broken'}
                    </span>
                  </span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Target</span>
                  <span class="meta__value mono">{activeLink.target_path}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Method</span>
                  <span class="meta__value mono">{activeLink.method}</span>
                </div>
              </div>

              <div class="output output--secondary">
                <div class="output__title">message</div>
                <pre class="preview preview--wrap">{activeLink.message}</pre>
              </div>

              <div class="output">
                <div class="output__title">operations</div>

                <div class="op">
                  <div class="op__row">
                    <span class="muted small">scope</span>
                    <div class="chips chips--tight" aria-label="Links 操作范围">
                      <button
                        class={['chip', linkScope === 'selected' ? 'chip--active' : ''].join(' ')}
                        onclick={() => onLinkScope('selected')}
                        type="button"
                      >
                        selected
                      </button>
                      <button
                        class={['chip', linkScope === 'all' ? 'chip--active' : ''].join(' ')}
                        onclick={() => onLinkScope('all')}
                        type="button"
                      >
                        all
                      </button>
                    </div>
                  </div>

                  <div class="toggle" role="group" aria-label="Links 操作选项">
                    <md-checkbox
                      checked={linkForce}
                      onchange={(event: Event) => {
                        const target = event.target as unknown as { checked?: unknown } | null;
                        onLinkForce(Boolean(target?.checked));
                      }}
                      aria-label="force"
                    ></md-checkbox>
                    <span>force（覆盖冲突 target）</span>
                  </div>

                  <div class="op__actions">
                    <md-outlined-button
                      disabled={busyLinkOp}
                      onclick={() => void onRunLinkOperation('plan')}
                      onkeydown={(event) =>
                        activateOnKey(event, () => void onRunLinkOperation('plan'))}
                      role="button"
                      tabindex="0"
                    >
                      {busyLinkOp && linkOpTitle === 'plan' ? '处理中…' : 'Plan'}
                    </md-outlined-button>
                    <md-filled-tonal-button
                      disabled={busyLinkOp}
                      onclick={() => void onRunLinkOperation('apply')}
                      onkeydown={(event) =>
                        activateOnKey(event, () => void onRunLinkOperation('apply'))}
                      role="button"
                      tabindex="0"
                    >
                      {busyLinkOp && linkOpTitle === 'apply' ? '处理中…' : 'Apply'}
                    </md-filled-tonal-button>
                    <md-filled-tonal-button
                      disabled={busyLinkOp}
                      onclick={() => void onRunLinkOperation('repair')}
                      onkeydown={(event) =>
                        activateOnKey(event, () => void onRunLinkOperation('repair'))}
                      role="button"
                      tabindex="0"
                    >
                      {busyLinkOp && linkOpTitle === 'repair' ? '处理中…' : 'Repair'}
                    </md-filled-tonal-button>
                  </div>

                  {#if linkScope === 'selected'}
                    <p class="muted small">
                      target={activeTargetIdForLink ?? '（未知）'} · profile={selectedProfile ?? '（空）'}
                    </p>
                  {:else}
                    <p class="muted small">
                      对所有 manifest targets 执行 · default_profile={selectedProfile ?? '（空）'}
                    </p>
                  {/if}
                </div>

                {#if linkOp}
                  <div class="op__result">
                    <p class="muted small">{linkOpTitle} · {linkOp.items.length} items</p>
                    <ul class="list">
                      {#each linkOp.items as item (item.item.target_path)}
                        <li class="list__static" title={item.item.target_path}>
                          <span class="pill pill--neutral">{item.action}</span>
                          <span class="mono">{item.item.target}</span>
                          <span class="muted">{truncateMiddle(item.item.target_path, 34)}</span>
                        </li>
                      {/each}
                    </ul>
                  </div>
                {:else}
                  <p class="muted small">（尚未运行 link 操作）</p>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>

