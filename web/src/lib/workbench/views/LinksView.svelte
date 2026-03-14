<script lang="ts">
  import { Tabs } from 'bits-ui';
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { get } from 'svelte/store';

  import SplitView from '$lib/components/SplitView.svelte';
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
  let panelTab = $state<'result' | 'focused' | 'status'>('result');

  const statusByPath = $derived.by(() => {
    const map = new Map<string, LinkStatusResponseItem>();
    for (const item of linkStatus ?? []) {
      map.set(item.target_path, item);
    }
    return map;
  });

  const filteredTargets = $derived.by(() => {
    const q = linkSearch.trim().toLowerCase();
    return targets.filter((target) => {
      const status = statusByPath.get(target.target_path) ?? null;

      if (linkUnhealthyOnly && (!status || status.ok)) {
        return false;
      }

      if (!q) {
        return true;
      }

      return (
        target.id.toLowerCase().includes(q) ||
        target.artifact_id.toLowerCase().includes(q) ||
        (target.profile ?? '').toLowerCase().includes(q) ||
        target.target_path.toLowerCase().includes(q) ||
        target.method.toLowerCase().includes(q)
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
    <p class="explorer__hint">以 targets 为主列表，多选与修复操作都在同一工作台完成。</p>
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
      <span>仅显示不健康项</span>
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
            {@const target = filteredTargets[row.index]}
            {@const status = statusByPath.get(target.target_path) ?? null}
            {@const isActive = selectedTargetId === target.id}
            {@const isSelected = selectedTargets.includes(target.id)}
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
                    onToggleTarget(target.id);
                    return;
                  }
                  onSelectTarget(target.id);
                }}
                type="button"
                title={target.target_path}
              >
                <span
                  class={[
                    'list__dot',
                    !status ? 'list__dot--accent' : status.ok ? 'list__dot--ok' : 'list__dot--bad'
                  ].join(' ')}
                  aria-hidden="true"
                ></span>
                <span class="list__name">{target.id}</span>
                <span class="list__meta">{target.artifact_id}@{target.profile ?? selectedProfile ?? 'default'}</span>
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
        {activeTarget ? `· ${truncateMiddle(activeTarget.target_path, 34)}` : '· target workspace'}
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

  <div class="workspace-split surface">
    <SplitView autoSaveId="workbench:links:shell" initialLeftPct={68} minLeftPx={480} minRightPx={300}>
      {#snippet left()}
        <SplitView
          autoSaveId="workbench:links:stack"
          direction="vertical"
          initialLeftPct={56}
          minLeftPx={240}
          minRightPx={180}
        >
          {#snippet left()}
            <section class="region" aria-label="target document">
              <div class="region__header">
                <span>Target Document</span>
                <span class="mono">{activeTarget?.id ?? 'idle'}</span>
              </div>

              <div class="panel__body panel__body--flush">
                {#if !activeTarget}
                  <p class="empty empty--flush">（请选择一个 target）</p>
                {:else}
                  <div class="subject-summary">
                    <div class="summary-row">
                      <span class="summary-row__label">Artifact</span>
                      <span class="summary-row__value mono">{activeTarget.artifact_id}</span>
                    </div>
                    <div class="summary-row">
                      <span class="summary-row__label">Profile</span>
                      <span class="summary-row__value mono">
                        {activeTarget.profile ?? selectedProfile ?? '（未指定）'}
                      </span>
                    </div>
                    <div class="summary-row">
                      <span class="summary-row__label">Method</span>
                      <span class="summary-row__value mono">{activeTarget.method}</span>
                    </div>
                    <div class="summary-row">
                      <span class="summary-row__label">Health</span>
                      <span class="summary-row__value">
                        {#if !activeLinkStatus}
                          尚未记录
                        {:else}
                          <span class={['pill', activeLinkStatus.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                            {activeLinkStatus.ok ? 'healthy' : 'broken'}
                          </span>
                        {/if}
                      </span>
                    </div>
                  </div>

                  <div class="inspector-table">
                    <div class="inspector-row">
                      <span class="inspector-row__label">Target Path</span>
                      <span class="inspector-row__value inspector-row__value--mono">{activeTarget.target_path}</span>
                    </div>
                    <div class="inspector-row">
                      <span class="inspector-row__label">Selection</span>
                      <span class="inspector-row__value inspector-row__value--mono">
                        {selectedTargets.length} selected
                      </span>
                    </div>
                    <div class="inspector-row">
                      <span class="inspector-row__label">Status</span>
                      <span class="inspector-row__value">
                        {activeLinkStatus?.message ?? '尚未 apply 或未记录状态'}
                      </span>
                    </div>
                  </div>
                {/if}
              </div>
            </section>
          {/snippet}

          {#snippet right()}
            <section class="panel bottom-panel" aria-label="Links 结果面板">
              <Tabs.Root value={panelTab} onValueChange={(next) => (panelTab = next as typeof panelTab)}>
                <div class="region__header">
                  <Tabs.List class="tabs" aria-label="Links panel tabs">
                    <Tabs.Trigger class="tab" value="result">Results</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="focused">Focused</Tabs.Trigger>
                    <Tabs.Trigger class="tab" value="status">Status</Tabs.Trigger>
                  </Tabs.List>
                  <span class="mono">{linkOpTitle ?? 'idle'}</span>
                </div>

                <Tabs.Content class="panel__body" value="result">
                  {#if !linkOp}
                    <p class="empty empty--flush">（尚未运行 link 操作）</p>
                  {:else}
                    <ul class="result-list">
                      {#each linkOp.items as item (item.item.target_path)}
                        <li class="result-row result-row--triple" title={item.item.target_path}>
                          <span class="pill pill--neutral">{item.action}</span>
                          <div class="result-row__main">
                            <span class="result-row__title">{item.item.target}</span>
                            <span class="result-row__detail">
                              {item.message ?? truncateMiddle(item.item.target_path, 88)}
                            </span>
                          </div>
                          <span class="mono muted">{item.item.method}</span>
                        </li>
                      {/each}
                    </ul>
                  {/if}
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="focused">
                  {#if !activeOpItem}
                    <p class="empty empty--flush">（选择已执行过的 target 后查看 focused result）</p>
                  {:else}
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Action</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeOpItem.action}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Desired</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {desiredSummary(activeOpItem.item.desired)}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Target</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeOpItem.item.target_path}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Message</span>
                        <span class="inspector-row__value">
                          {activeOpItem.message ?? '（无额外信息）'}
                        </span>
                      </div>
                    </div>
                  {/if}
                </Tabs.Content>

                <Tabs.Content class="panel__body" value="status">
                  {#if !activeTarget}
                    <p class="empty empty--flush">（先选择 target，再查看健康度）</p>
                  {:else if !activeLinkStatus}
                    <p class="empty empty--flush">（该 target 还没有 link status 记录）</p>
                  {:else}
                    <div class="inspector-table">
                      <div class="inspector-row">
                        <span class="inspector-row__label">Health</span>
                        <span class="inspector-row__value">
                          <span class={['pill', activeLinkStatus.ok ? 'pill--ok' : 'pill--warn'].join(' ')}>
                            {activeLinkStatus.ok ? 'healthy' : 'broken'}
                          </span>
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Artifact</span>
                        <span class="inspector-row__value inspector-row__value--mono">
                          {activeLinkStatus.artifact_id}@{activeLinkStatus.profile}
                        </span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Method</span>
                        <span class="inspector-row__value inspector-row__value--mono">{activeLinkStatus.method}</span>
                      </div>
                      <div class="inspector-row">
                        <span class="inspector-row__label">Message</span>
                        <span class="inspector-row__value">{activeLinkStatus.message}</span>
                      </div>
                    </div>
                  {/if}
                </Tabs.Content>
              </Tabs.Root>
            </section>
          {/snippet}
        </SplitView>
      {/snippet}

      {#snippet right()}
        <section class="region secondary-sidebar" aria-label="links 操作侧栏">
          <div class="region__header">
            <span>Operations</span>
            <span class="mono">{linkScope}</span>
          </div>

          <div class="region__body">
            <div class="subject-summary">
              <div class="summary-row">
                <span class="summary-row__label">Selected</span>
                <span class="summary-row__value mono">{selectedTargets.length}</span>
              </div>
              <div class="summary-row">
                <span class="summary-row__label">Profile</span>
                <span class="summary-row__value mono">{selectedProfile ?? '（空）'}</span>
              </div>
              <div class="summary-row">
                <span class="summary-row__label">Force</span>
                <span class="summary-row__value mono">{linkForce ? 'on' : 'off'}</span>
              </div>
            </div>

            <div class="region__toolbar" aria-label="Links scope">
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

            <label class="toggle" aria-label="Links force">
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

            <p class="stack-note">
              `selected` 只作用于当前选择的 targets，`all` 则面向整个 manifest。结果会写入左侧底部 panel。
            </p>

            {#if selectedTargets.length > 0}
              <div class="token-list" aria-label="selected targets">
                {#each selectedTargets as id (id)}
                  <span class="token">{id}</span>
                {/each}
              </div>
            {/if}
          </div>
        </section>
      {/snippet}
    </SplitView>
  </div>
</main>
