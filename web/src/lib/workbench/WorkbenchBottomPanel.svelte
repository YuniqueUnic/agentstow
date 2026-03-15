<script lang="ts">
  import type { ValidationIssueResponse, WatchStatusResponse } from '$lib/types';
  import { formatRelativeTime, truncateMiddle } from '$lib/utils/format';

  export type BottomPanelTab = 'problems' | 'trace';

  type Props = {
    activeTab: BottomPanelTab;
    errorMessage: string | null;
    issues: ValidationIssueResponse[];
    watchStatus: WatchStatusResponse | null;
    busyWatch: boolean;
    onSelectTab: (tab: BottomPanelTab) => void;
    onRefreshTrace: () => void | Promise<void>;
    onClose: () => void;
  };

  let {
    activeTab,
    errorMessage,
    issues,
    watchStatus,
    busyWatch,
    onSelectTab,
    onRefreshTrace,
    onClose
  }: Props = $props();

  const watchTraceEvents = $derived(watchStatus?.recent_events ?? []);
  const problemCount = $derived((errorMessage ? 1 : 0) + issues.length);
  const traceCount = $derived(watchTraceEvents.length);

  function traceTone(level: 'change' | 'error'): 'ok' | 'warn' {
    return level === 'change' ? 'ok' : 'warn';
  }
</script>

<section
  class="bottom-panel"
  aria-label="Workbench bottom panel"
  data-testid="workbench-bottom-panel"
>
  <div class="bottom-panel__head">
    <div class="bottom-panel__tabs" role="tablist" aria-label="Bottom panel tabs">
      <button
        class={['bottom-panel__tab', activeTab === 'problems' ? 'bottom-panel__tab--active' : ''].join(' ')}
        type="button"
        role="tab"
        aria-selected={activeTab === 'problems'}
        onclick={() => onSelectTab('problems')}
      >
        Problems
        <span class="mono">{problemCount}</span>
      </button>
      <button
        class={['bottom-panel__tab', activeTab === 'trace' ? 'bottom-panel__tab--active' : ''].join(' ')}
        type="button"
        role="tab"
        aria-selected={activeTab === 'trace'}
        onclick={() => onSelectTab('trace')}
      >
        Trace
        <span class="mono">{traceCount}</span>
      </button>
    </div>

    <div class="bottom-panel__actions">
      {#if activeTab === 'trace'}
        <button class="ui-button ui-button--subtle" disabled={busyWatch} type="button" onclick={() => void onRefreshTrace()}>
          {busyWatch ? '刷新中…' : '刷新 trace'}
        </button>
      {/if}
      <button class="ui-button ui-button--subtle" type="button" onclick={onClose}>隐藏</button>
    </div>
  </div>

  {#if activeTab === 'problems'}
    <div class="bottom-panel__body bottom-panel__body--problems" role="tabpanel">
      {#if !errorMessage && issues.length === 0}
        <p class="empty">（当前没有 problems）</p>
      {:else}
        <ul class="result-list" aria-label="Problems list">
          {#if errorMessage}
            <li class="result-row">
              <div class="result-row__static">
                <span class="pill pill--warn">runtime</span>
                <span class="result-row__title">当前操作失败</span>
                <span class="result-row__detail">{errorMessage}</span>
              </div>
            </li>
          {/if}

          {#each issues as issue (`${issue.scope}:${issue.subject_id}:${issue.code}:${issue.message}`)}
            <li class="result-row">
              <div class="result-row__static">
                <span class={['pill', issue.severity === 'error' ? 'pill--warn' : 'pill--neutral'].join(' ')}>
                  {issue.severity}
                </span>
                <span class="result-row__title">{issue.scope} · {issue.subject_id}</span>
                <span class="result-row__detail">{issue.code} · {issue.message}</span>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else}
    <div
      class="bottom-panel__body bottom-panel__body--trace"
      role="tabpanel"
      data-testid="watch-trace-panel"
    >
      <section class="bottom-panel__meta" aria-label="Watcher roots">
        <div class="section__title">
          <span>Watch Roots</span>
          <strong>{watchStatus?.watch_roots.length ?? 0}</strong>
        </div>
        {#if !watchStatus}
          <p class="empty">（watcher 状态尚未加载）</p>
        {:else}
          <div class="subject-summary">
            <div class="summary-row">
              <span class="summary-row__label">Mode</span>
              <span class="summary-row__value mono">{watchStatus.mode}</span>
            </div>
            <div class="summary-row">
              <span class="summary-row__label">Revision</span>
              <span class="summary-row__value mono">{watchStatus.revision}</span>
            </div>
            <div class="summary-row">
              <span class="summary-row__label">Health</span>
              <span class="summary-row__value mono">{watchStatus.healthy ? 'healthy' : 'attention'}</span>
            </div>
            {#if watchStatus.poll_interval_ms}
              <div class="summary-row">
                <span class="summary-row__label">Poll</span>
                <span class="summary-row__value mono">{watchStatus.poll_interval_ms} ms</span>
              </div>
            {/if}
          </div>

          {#if watchStatus.last_error}
            <p class="notice notice--error">{watchStatus.last_error}</p>
          {/if}

          <ul class="trace-roots" aria-label="Watcher roots list">
            {#each watchStatus.watch_roots as root (root)}
              <li class="trace-roots__item mono" title={root}>{truncateMiddle(root, 88)}</li>
            {/each}
          </ul>
        {/if}
      </section>

      <section class="bottom-panel__events" aria-label="Recent watcher events">
        <div class="section__title">
          <span>Recent Events</span>
          <strong>{traceCount}</strong>
        </div>

        {#if !watchStatus}
          <p class="empty">（watcher 状态尚未加载）</p>
        {:else if watchTraceEvents.length === 0}
          <p class="empty">（暂无 watcher events，可在保存 source 后再展开查看）</p>
        {:else}
          <ul class="trace-list">
            {#each watchTraceEvents as event (`${event.at}:${event.summary}`)}
              <li class="trace-list__item">
                <span class={['pill', `pill--${traceTone(event.level)}`].join(' ')}>
                  {event.level}
                </span>
                <div class="trace-list__main">
                  <span class="trace-list__summary">{event.summary}</span>
                  <span class="trace-list__meta mono">
                    rev:{event.revision} · {formatRelativeTime(event.at)}
                  </span>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  {/if}
</section>
