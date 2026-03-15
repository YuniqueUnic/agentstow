<script lang="ts">
  import { truncateMiddle } from '$lib/utils/format';

  type Props = {
    workspaceRoot: string | null;
    workspaceLabel: string;
    onOpenPalette: () => void;
    onSwitchWorkspace: () => void;
  };

  let {
    workspaceRoot,
    workspaceLabel,
    onOpenPalette,
    onSwitchWorkspace
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

  <div class="topbar__actions">
    <button class="command-launcher" type="button" onclick={onOpenPalette}>
      <span>Command Palette</span>
      <kbd class="topbar__shortcut">Cmd/Ctrl+K</kbd>
    </button>
    <button class="ui-button ui-button--ghost" type="button" onclick={onSwitchWorkspace}>
      Workspace
    </button>
  </div>
</header>
