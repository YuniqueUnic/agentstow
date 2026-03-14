<script lang="ts">
  import { Tabs } from 'bits-ui';

  type Tab = {
    id: string;
    label: string;
    dirty: boolean;
  };

  type Props = {
    tabs: Tab[];
    active: string | null;
    onChange?: (next: string) => void;
  };

  let { tabs, active, onChange }: Props = $props();

  const resolvedActive = $derived.by(() => {
    if (!tabs.length) {
      return null;
    }
    if (active && tabs.some((t) => t.id === active)) {
      return active;
    }
    return tabs[0].id;
  });
</script>

{#if !tabs.length}
  <div class="editor-tabs muted">（未打开任何 artifact）</div>
{:else if resolvedActive}
  <div class="editor-tabs" role="region" aria-label="打开的 artifacts">
    <Tabs.Root
      value={resolvedActive}
      onValueChange={(next) => onChange?.(next as string)}
    >
      <Tabs.List class="editor-tabs__list" aria-label="Open editors">
        {#each tabs as tab (tab.id)}
          <Tabs.Trigger class="editor-tab" value={tab.id} title={tab.label}>
            <span
              class={['dirty-dot', tab.dirty ? '' : 'dirty-dot--clean'].join(' ')}
              aria-hidden="true"
            ></span>
            <span class="editor-tab__name">{tab.label}</span>
          </Tabs.Trigger>
        {/each}
      </Tabs.List>
    </Tabs.Root>
  </div>
{/if}

