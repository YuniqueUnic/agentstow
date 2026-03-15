<script lang="ts">
  import type { WorkspaceGitSummaryResponse } from '$lib/types';
  import { truncateMiddle } from '$lib/utils/format';
  import type { ResolvedTheme, ThemePreference } from '$lib/workbench/theme';

  type WatchPill = {
    tone: 'neutral' | 'warn' | 'ok';
    label: string;
  };

  type Props = {
    workspaceRoot: string | null;
    workspaceLabel: string;
    gitInfo: WorkspaceGitSummaryResponse | null;
    watchPill: WatchPill;
    watchActivity: string;
    busySummary: boolean;
    themePreference: ThemePreference;
    resolvedTheme: ResolvedTheme;
    onOpenPalette: () => void;
    onSetTheme: (next: ThemePreference) => void;
    onSwitchWorkspace: () => void;
    onRefresh: () => Promise<void>;
  };

  let {
    workspaceRoot,
    workspaceLabel,
    gitInfo,
    watchPill,
    watchActivity,
    busySummary,
    themePreference,
    resolvedTheme,
    onOpenPalette,
    onSetTheme,
    onSwitchWorkspace,
    onRefresh
  }: Props = $props();

  const themeChoices: Array<{ id: ThemePreference; label: string; hint: string }> = [
    { id: 'system', label: 'Auto', hint: '跟随系统' },
    { id: 'light', label: 'Light', hint: '浅色' },
    { id: 'dark', label: 'Dark', hint: '深色' }
  ];
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
    <div class="topbar__status-block">
      <span class={['pill', `pill--${watchPill.tone}`].join(' ')}>
        {watchPill.label}
      </span>
      <span class="muted" title={watchActivity}>
        {truncateMiddle(watchActivity, 28)}
      </span>
    </div>
    {#if gitInfo}
      <div class="topbar__git" title={gitInfo.repo_root}>
        <span class={['pill', gitInfo.dirty ? 'pill--warn' : 'pill--neutral'].join(' ')}>
          {gitInfo.dirty ? 'dirty' : 'clean'}
        </span>
        <span class="mono topbar__git-ref">
          {truncateMiddle(`${gitInfo.branch ?? 'detached'} @ ${gitInfo.head_short}`, 28)}
        </span>
      </div>
    {/if}
  </div>

  <div class="topbar__actions">
    <div class="theme-switcher" role="group" aria-label="Theme">
      {#each themeChoices as theme (theme.id)}
        <button
          class={[
            'theme-switcher__item',
            themePreference === theme.id ? 'theme-switcher__item--active' : ''
          ].join(' ')}
          type="button"
          title={`${theme.hint}（当前生效：${resolvedTheme}）`}
          aria-pressed={themePreference === theme.id}
          onclick={() => onSetTheme(theme.id)}
        >
          {theme.label}
        </button>
      {/each}
    </div>
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
