<script lang="ts">
  type Tab = {
    id: string;
    label: string;
    dirty: boolean;
  };

  type Props = {
    tabs: Tab[];
    active: string | null;
    onChange?: (next: string) => void;
    onClose?: (id: string) => void;
    onReorder?: (nextOrder: string[]) => void;
    onOpenContextMenu?: (id: string, x: number, y: number) => void;
  };

  let { tabs, active, onChange, onClose, onReorder, onOpenContextMenu }: Props = $props();

  const resolvedActive = $derived.by(() => {
    if (!tabs.length) {
      return null;
    }
    if (active && tabs.some((t) => t.id === active)) {
      return active;
    }
    return tabs[0].id;
  });

  let draggedId = $state<string | null>(null);

  function moveBefore(list: string[], source: string, target: string): string[] {
    const from = list.indexOf(source);
    const to = list.indexOf(target);
    if (from === -1 || to === -1 || from === to) {
      return list;
    }

    const next = list.slice();
    next.splice(from, 1);
    const insertAt = from < to ? to - 1 : to;
    next.splice(insertAt, 0, source);
    return next;
  }
</script>

{#if !tabs.length}
  <div class="editor-tabs muted">（未打开任何 artifact）</div>
{:else if resolvedActive}
  <div class="editor-tabs" role="region" aria-label="打开的 artifacts">
    <div class="editor-tabs__list" role="tablist" aria-label="Open editors">
      {#each tabs as tab (tab.id)}
        <div
          class={[
            'editor-tab',
            tab.id === resolvedActive ? 'editor-tab--active' : ''
          ].join(' ')}
          role="presentation"
          draggable="true"
          ondragstart={(event) => {
            draggedId = tab.id;
            event.dataTransfer?.setData('text/plain', tab.id);
            event.dataTransfer?.setDragImage(event.currentTarget as Element, 18, 18);
          }}
          ondragend={() => {
            draggedId = null;
          }}
          ondragover={(event) => {
            event.preventDefault();
          }}
          ondrop={(event) => {
            event.preventDefault();
            const incoming = draggedId ?? event.dataTransfer?.getData('text/plain') ?? null;
            if (!incoming) {
              return;
            }
            const next = moveBefore(tabs.map((t) => t.id), incoming, tab.id);
            draggedId = null;
            onReorder?.(next);
          }}
          oncontextmenu={(event) => {
            event.preventDefault();
            onOpenContextMenu?.(tab.id, event.clientX, event.clientY);
          }}
        >
          <button
            class="editor-tab__main"
            type="button"
            role="tab"
            aria-selected={tab.id === resolvedActive}
            tabindex={tab.id === resolvedActive ? 0 : -1}
            onclick={() => onChange?.(tab.id)}
            title={tab.label}
          >
            <span
              class={['dirty-dot', tab.dirty ? '' : 'dirty-dot--clean'].join(' ')}
              aria-hidden="true"
            ></span>
            <span class="editor-tab__name">{tab.label}</span>
          </button>
          <button
            class="editor-tab__close"
            type="button"
            aria-label={`关闭 ${tab.label}`}
            title="关闭"
            onclick={() => onClose?.(tab.id)}
          >
            ×
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}
