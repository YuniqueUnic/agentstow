<script lang="ts">
  import { truncateMiddle } from '$lib/utils/format';

  type WatchPill = {
    tone: 'neutral' | 'warn' | 'ok';
    label: string;
  };

  type Props = {
    workspaceRoot: string | null;
    workspaceLabel: string;
    watchPill: WatchPill;
    watchActivity: string;
    busySummary: boolean;
    onSwitchWorkspace: () => void;
    onRefresh: () => Promise<void>;
  };

  let {
    workspaceRoot,
    workspaceLabel,
    watchPill,
    watchActivity,
    busySummary,
    onSwitchWorkspace,
    onRefresh
  }: Props = $props();

  function activateOnKey(event: KeyboardEvent, action: () => void): void {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }

    event.preventDefault();
    action();
  }
</script>

<header class="topbar surface">
  <div class="topbar__brand">
    <span class="mark" aria-hidden="true"></span>
    <div>
      <strong>AgentStow</strong>
      <span class="muted">workbench</span>
    </div>
  </div>

  <div class="topbar__workspace" title={workspaceRoot ?? ''}>
    <span class="muted">Workspace</span>
    <span class="mono">{workspaceLabel}</span>
    <md-text-button
      onclick={onSwitchWorkspace}
      onkeydown={(event) => activateOnKey(event, onSwitchWorkspace)}
      role="button"
      tabindex="0"
    >
      切换
    </md-text-button>
  </div>

  <div class="topbar__status">
    <span class={['pill', `pill--${watchPill.tone}`].join(' ')}>
      {watchPill.label}
    </span>
    <span class="muted" title={watchActivity}>
      {truncateMiddle(watchActivity, 28)}
    </span>
  </div>

  <div class="topbar__actions">
    <md-outlined-button
      disabled={busySummary}
      onclick={() => void onRefresh()}
      onkeydown={(event) => activateOnKey(event, () => void onRefresh())}
      role="button"
      tabindex="0"
    >
      刷新
    </md-outlined-button>
  </div>
</header>

