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
    onOpenPalette: () => void;
    onSwitchWorkspace: () => void;
    onRefresh: () => Promise<void>;
  };

  let {
    workspaceRoot,
    workspaceLabel,
    watchPill,
    watchActivity,
    busySummary,
    onOpenPalette,
    onSwitchWorkspace,
    onRefresh
  }: Props = $props();
</script>

<header class="topbar">
  <div class="topbar__brand">
    <span class="mark" aria-hidden="true"></span>
    <div class="topbar__brand-copy">
      <strong>AgentStow</strong>
      <span class="muted">local-first workbench</span>
    </div>
  </div>

  <div class="topbar__workspace" title={workspaceRoot ?? ''}>
    <span class="topbar__crumb">workspace</span>
    <span class="mono">{truncateMiddle(workspaceRoot ?? workspaceLabel, 72)}</span>
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
    <button class="command-launcher" type="button" onclick={onOpenPalette}>
      <span>Command</span>
      <kbd class="topbar__shortcut">Cmd/Ctrl+K</kbd>
    </button>
    <button class="ui-button ui-button--ghost" type="button" onclick={onSwitchWorkspace}>
      切换 workspace
    </button>
    <button
      class="ui-button ui-button--primary"
      disabled={busySummary}
      type="button"
      onclick={() => void onRefresh()}
    >
      {busySummary ? '刷新中…' : '同步视图'}
    </button>
  </div>
</header>
