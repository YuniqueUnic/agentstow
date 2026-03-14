<script lang="ts">
  import SplitView from '$lib/components/SplitView.svelte';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { get } from 'svelte/store';
  import type {
    LinkDesiredInstallResponse,
    LinkOperationItemResponse,
    LinkOperationResponse,
    LinkStatusResponseItem,
    TargetSummaryResponse
  } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';

  type Props = {
    targets: TargetSummaryResponse[];
    linkStatus: LinkStatusResponseItem[] | null;
    selectedTargetId: string | null;
    selectedTargets: string[];
    linkSearch: string;
    linkUnhealthyOnly: boolean;
    linkForce: boolean;
    linkScope: 'selected' | 'all';
    linkOp: LinkOperationResponse | null;
    linkOpTitle: string | null;
    activeTarget: TargetSummaryResponse | null;
    activeLinkStatus: LinkStatusResponseItem | null;
    selectedProfile: string | null;
    busyLinks: boolean;
    busyLinkOp: boolean;
    errorMessage: string | null;
    statusLine: string;
    onLinkSearch: (next: string) => void;
    onLinkUnhealthyOnly: (next: boolean) => void;
    onLinkForce: (next: boolean) => void;
    onLinkScope: (next: 'selected' | 'all') => void;
    onSelectTarget: (id: string) => void;
    onToggleTarget: (id: string) => void;
    onRefreshLinkStatus: () => Promise<void>;
    onCopyToClipboard: (text: string, label: string) => Promise<void>;
    onRunLinkOperation: (kind: 'plan' | 'apply' | 'repair') => Promise<void>;
  };

  let {
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
    selectedProfile,
    busyLinks,
    busyLinkOp,
    errorMessage,
    statusLine,
    onLinkSearch,
    onLinkUnhealthyOnly,
    onLinkForce,
    onLinkScope,
    onSelectTarget,
    onToggleTarget,
    onRefreshLinkStatus,
    onCopyToClipboard,
    onRunLinkOperation
  }: Props = $props();

  let targetListEl = $state<HTMLDivElement | null>(null);

  const statusByPath = $derived.by(() => {
    const map = new Map<string, LinkStatusResponseItem>();
    for (const item of linkStatus ?? []) {
      map.set(item.target_path, item);
    }
    return map;
  });

  const filteredTargets = $derived.by(() => {
    const q = linkSearch.trim().toLowerCase();
    return targets.filter((t) => {
      const status = statusByPath.get(t.target_path) ?? null;

      if (linkUnhealthyOnly) {
        if (!status || status.ok) {
          return false;
        }
      }

      if (!q) {
        return true;
      }

      return (
        t.id.toLowerCase().includes(q) ||
        t.artifact_id.toLowerCase().includes(q) ||
        (t.profile ?? '').toLowerCase().includes(q) ||
        t.target_path.toLowerCase().includes(q) ||
        t.method.toLowerCase().includes(q)
      );
    });
  });

  const activeOpItem = $derived.by((): LinkOperationItemResponse | null => {
    if (!activeTarget || !linkOp) {
      return null;
    }
    return linkOp.items.find((item) => item.item.target === activeTarget.id) ?? null;
  });

  function desiredSummary(desired: LinkDesiredInstallResponse): string {
    if (desired.kind === 'copy') {
      return `copy · ${desired.bytes_len} bytes · ${desired.blake3.slice(0, 10)}…`;
    }
    return `${desired.kind} · ${desired.source_path}`;
  }

  const targetVirtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => targetListEl,
    estimateSize: () => 52,
    overscan: 10
  });

  $effect(() => {
    const virtualizer = get(targetVirtualizer);
    const nextCount = filteredTargets.length;
    if (virtualizer.options.count === nextCount) {
      return;
    }
    virtualizer.setOptions({ count: nextCount });
  });
</script>

<aside class="explorer surface" aria-label="资源面板">
  <div class="explorer__head">
    <p class="explorer__eyebrow">LINKS</p>
    <p class="explorer__hint">以 targets 为主列表。提示：Ctrl/Cmd 点击可多选。</p>
  </div>

  <div class="explorer__section explorer__section--fill">
    <div class="section__title">
      <span>Targets</span>
      <strong>{filteredTargets.length}</strong>
    </div>

    <label class="field field--compact">
      <span class="field__label">搜索 targets</span>
      <input
        class="field__input mono"
        type="search"
        placeholder="id/artifact/profile/path…"
        value={linkSearch}
        oninput={(event) => {
          const target = event.currentTarget as HTMLInputElement | null;
          onLinkSearch(target?.value ?? '');
        }}
      />
    </label>

    <label class="toggle" aria-label="Targets 过滤">
      <input
        class="toggle__control"
        type="checkbox"
        checked={linkUnhealthyOnly}
        onchange={(event) => {
          const target = event.currentTarget as HTMLInputElement | null;
          onLinkUnhealthyOnly(Boolean(target?.checked));
        }}
      />
      <span>仅显示不健康项（已应用且 broken）</span>
    </label>

    {#if targets.length === 0}
      <div class="list__static">
        <span class="muted">（manifest 未声明 targets）</span>
        <span class="mono">targets</span>
      </div>
    {:else if filteredTargets.length === 0}
      <div class="list__static">
        <span class="muted">（无匹配结果）</span>
        <span class="mono">{linkSearch.trim() || 'query'}</span>
      </div>
    {:else}
      <div class="virtual-list" bind:this={targetListEl} role="list" aria-label="Declared targets list">
        <div class="virtual-list__inner" style={`height:${$targetVirtualizer.getTotalSize()}px;`}>
          {#each $targetVirtualizer.getVirtualItems() as row (row.key)}
            {@const t = filteredTargets[row.index]}
            {@const status = statusByPath.get(t.target_path) ?? null}
            {@const isActive = selectedTargetId === t.id}
            {@const isSelected = selectedTargets.includes(t.id)}
              <div class="virtual-list__row" style={`transform:translateY(${row.start}px);`}>
                <button
                  class={[
                    'list__item',
                    isActive ? 'list__item--active' : '',
                    !isActive && isSelected ? 'list__item--selected' : ''
                  ].join(' ')}
                  onclick={(event) => {
                    const e = event as MouseEvent;
                    if (e.metaKey || e.ctrlKey) {
                      onToggleTarget(t.id);
                      return;
                    }
                    onSelectTarget(t.id);
                  }}
                  type="button"
                  title={t.target_path}
                >
                <span
                  class={[
                    'list__dot',
                    !status ? 'list__dot--accent' : status.ok ? 'list__dot--ok' : 'list__dot--bad'
                  ].join(' ')}
                  aria-hidden="true"
                ></span>
                <span class="list__name">{t.id}</span>
                <span class="list__meta">{t.artifact_id}@{t.profile ?? selectedProfile ?? 'default'}</span>
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
</aside>

<main class="canvas" aria-label="工作区画布">
  <div class="canvas__head">
    <div class="title">
      <strong>{activeTarget?.id ?? '未选择 target'}</strong>
      <span class="muted">
        {activeTarget ? `· ${truncateMiddle(activeTarget.target_path, 34)}` : ''}
      </span>
    </div>

    <div class="canvas__actions">
      <button
        class="ui-button ui-button--ghost"
        disabled={busyLinks}
        type="button"
        onclick={() => void onRefreshLinkStatus()}
      >
        {busyLinks ? '刷新中…' : '刷新 status'}
      </button>
      <button
        class="ui-button ui-button--primary"
        disabled={!activeTarget}
        type="button"
        onclick={() => void onCopyToClipboard(activeTarget?.target_path ?? '', '目标路径')}
      >
        复制路径
      </button>
    </div>
  </div>

  {#if errorMessage}
    <p class="notice notice--error">{errorMessage}</p>
  {/if}
  <p class="status-line" aria-live="polite">{statusLine}</p>

  <div class="split surface">
    <SplitView initialLeftPct={48} minLeftPx={360} minRightPx={360}>
      {#snippet left()}
        <div class="pane">
          <div class="pane__title">Target</div>
          <div class="pane__body">
            {#if !activeTarget}
              <p class="muted">（请选择一个 target）</p>
            {:else}
              <div class="meta">
                <div class="meta__row">
                  <span class="meta__label">Artifact</span>
                  <span class="meta__value mono">{activeTarget.artifact_id}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Profile</span>
                  <span class="meta__value mono">
                    {activeTarget.profile ?? selectedProfile ?? '（未指定）'}
                  </span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Method</span>
                  <span class="meta__value mono">{activeTarget.method}</span>
                </div>
                <div class="meta__row">
                  <span class="meta__label">Target Path</span>
                  <span class="meta__value mono">{activeTarget.target_path}</span>
                </div>
              </div>

              <div class="output output--secondary">
                <div class="output__title">link status</div>
                {#if !activeLinkStatus}
                  <p class="muted small">（尚未 apply 或未记录状态）</p>
                {:else}
                  <div class="meta">
                    <div class="meta__row">
                      <span class="meta__label">Health</span>
                      <span class="meta__value">
                        <span class={['pill', activeLinkStatus.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                          {activeLinkStatus.ok ? 'healthy' : 'broken'}
                        </span>
                      </span>
                    </div>
                    <div class="meta__row">
                      <span class="meta__label">Message</span>
                      <span class="meta__value mono">{activeLinkStatus.message}</span>
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet right()}
        <div class="pane">
          <div class="pane__title">Operations</div>
          <div class="pane__body">
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

              <label class="toggle" aria-label="Links 操作选项">
                <input
                  class="toggle__control"
                  type="checkbox"
                  checked={linkForce}
                  onchange={(event) => {
                    const target = event.currentTarget as HTMLInputElement | null;
                    onLinkForce(Boolean(target?.checked));
                  }}
                />
                <span>force（覆盖冲突 target）</span>
              </label>

              <div class="op__actions">
                <button
                  class="ui-button ui-button--ghost"
                  disabled={busyLinkOp}
                  type="button"
                  onclick={() => void onRunLinkOperation('plan')}
                >
                  {busyLinkOp && linkOpTitle === 'plan' ? '处理中…' : 'Plan'}
                </button>
                <button
                  class="ui-button ui-button--primary"
                  disabled={busyLinkOp}
                  type="button"
                  onclick={() => void onRunLinkOperation('apply')}
                >
                  {busyLinkOp && linkOpTitle === 'apply' ? '处理中…' : 'Apply'}
                </button>
                <button
                  class="ui-button ui-button--primary ui-button--danger"
                  disabled={busyLinkOp}
                  type="button"
                  onclick={() => void onRunLinkOperation('repair')}
                >
                  {busyLinkOp && linkOpTitle === 'repair' ? '处理中…' : 'Repair'}
                </button>
              </div>

              {#if linkScope === 'selected'}
                <p class="muted small">
                  已选择 {selectedTargets.length} targets · default_profile={selectedProfile ?? '（空）'}
                </p>
              {:else}
                <p class="muted small">
                  plan/apply：对所有 manifest targets 执行 · repair：扫描并修复不健康项 · default_profile={selectedProfile ?? '（空）'}
                </p>
              {/if}
            </div>

            <div class="output">
              <div class="output__title">result</div>

              {#if !linkOp}
                <p class="muted small">（尚未运行 link 操作）</p>
              {:else}
                <p class="muted small">{linkOpTitle} · {linkOp.items.length} items</p>
                <ul class="list">
                  {#each linkOp.items as item (item.item.target_path)}
                    <li class="list__static" title={item.item.target_path}>
                      <span class="pill pill--neutral">{item.action}</span>
                      <span class="mono">{item.item.target}</span>
                      <span class="muted">{item.message ?? truncateMiddle(item.item.target_path, 30)}</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>

            {#if activeOpItem}
              <div class="output output--secondary">
                <div class="output__title">focused item</div>
                <div class="meta">
                  <div class="meta__row">
                    <span class="meta__label">Action</span>
                    <span class="meta__value mono">{activeOpItem.action}</span>
                  </div>
                  <div class="meta__row">
                    <span class="meta__label">Desired</span>
                    <span class="meta__value mono">{desiredSummary(activeOpItem.item.desired)}</span>
                  </div>
                  {#if activeOpItem.message}
                    <div class="meta__row">
                      <span class="meta__label">Message</span>
                      <span class="meta__value mono">{activeOpItem.message}</span>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/snippet}
    </SplitView>
  </div>
</main>
