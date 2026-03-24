<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  import { Pane, PaneGroup, PaneResizer } from 'paneforge';

  type Direction = 'horizontal' | 'vertical';

  type Props = {
    left: Snippet;
    right: Snippet;
    direction?: Direction;
    initialLeftPct?: number;
    minLeftPx?: number;
    minRightPx?: number;
    disabled?: boolean;
    autoSaveId?: string | null;
    keyboardResizeBy?: number | null;
    onLayoutChange?: ((layout: number[]) => void) | null;
  };

  let {
    left,
    right,
    direction = 'horizontal',
    initialLeftPct = 52,
    minLeftPx = 320,
    minRightPx = 320,
    disabled = false,
    autoSaveId = null,
    keyboardResizeBy = null,
    onLayoutChange = null
  }: Props = $props();

  let host: HTMLDivElement | null = null;
  let dragging = $state(false);
  let hostWidthPx = $state(0);
  let hostHeightPx = $state(0);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  const activeDirection = $derived.by<Direction>(() => {
    if (direction !== 'horizontal') {
      return direction;
    }

    const collapseThresholdPx = minLeftPx + minRightPx + 72;
    if (hostWidthPx > 0 && hostWidthPx <= collapseThresholdPx) {
      return 'vertical';
    }

    return direction;
  });

  const effectiveMinLeftPx = $derived.by(() =>
    activeDirection === 'vertical' && direction === 'horizontal' ? Math.min(minLeftPx, 220) : minLeftPx
  );
  const effectiveMinRightPx = $derived.by(() =>
    activeDirection === 'vertical' && direction === 'horizontal' ? Math.min(minRightPx, 260) : minRightPx
  );
  const groupExtentPx = $derived(activeDirection === 'horizontal' ? hostWidthPx : hostHeightPx);

  function pxToPct(px: number): number {
    if (groupExtentPx <= 0) {
      return 0;
    }
    return (px / groupExtentPx) * 100;
  }

  const minFirstPct = $derived.by(() => clamp(pxToPct(effectiveMinLeftPx), 0, 100));
  const minSecondPct = $derived.by(() => clamp(pxToPct(effectiveMinRightPx), 0, 100));
  const maxFirstPct = $derived.by(() => Math.max(minFirstPct, 100 - minSecondPct));
  const defaultFirstPct = $derived.by(() =>
    clamp(initialLeftPct, minFirstPct, maxFirstPct)
  );

  onMount(() => {
    if (!host) {
      return;
    }

    const updateExtent = () => {
      if (!host) {
        return;
      }
      const rect = host.getBoundingClientRect();
      hostWidthPx = rect.width;
      hostHeightPx = rect.height;
    };

    updateExtent();

    const observer = new ResizeObserver(() => {
      updateExtent();
    });

    observer.observe(host);

    return () => {
      observer.disconnect();
    };
  });
</script>

<div
  class="splitview"
  data-direction={activeDirection}
  data-dragging={dragging ? 'true' : 'false'}
  data-stacked={activeDirection === 'vertical' && direction === 'horizontal' ? 'true' : 'false'}
  bind:this={host}
>
  <PaneGroup
    direction={activeDirection}
    {autoSaveId}
    {keyboardResizeBy}
    onLayoutChange={onLayoutChange ?? undefined}
  >
    <Pane
      class="splitview__pane splitview__pane--first"
      defaultSize={defaultFirstPct}
      minSize={minFirstPct}
      maxSize={maxFirstPct}
    >
      {@render left()}
    </Pane>

    <PaneResizer
      class="splitview__gutter"
      {disabled}
      onDraggingChange={(next) => {
        dragging = next;
      }}
    />

    <Pane class="splitview__pane splitview__pane--second" minSize={minSecondPct}>
      {@render right()}
    </Pane>
  </PaneGroup>
</div>

<style>
  .splitview {
    height: 100%;
    min-height: 0;
    min-width: 0;
  }

  .splitview :global([data-pane-group]) {
    height: 100%;
    width: 100%;
    min-height: 0;
    min-width: 0;
  }

  .splitview :global([data-pane]) {
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }

  .splitview :global([data-pane-resizer]) {
    flex: 0 0 auto;
    position: relative;
    padding: 0;
    border: 0;
    background: transparent;
    outline: none;
    touch-action: none;
    display: grid;
    place-items: center;
  }

  .splitview :global([data-pane-resizer][data-direction='horizontal']) {
    width: 14px;
    cursor: col-resize;
  }

  .splitview :global([data-pane-resizer][data-direction='vertical']) {
    height: 14px;
    cursor: row-resize;
  }

  .splitview :global([data-pane-resizer])::before {
    content: '';
    border-radius: 999px;
    background: color-mix(in oklch, var(--line-strong) 30%, transparent);
    transition:
      background 140ms ease,
      transform 140ms ease,
      opacity 140ms ease;
    opacity: 0.9;
  }

  .splitview :global([data-pane-resizer][data-direction='horizontal'])::before {
    width: 2px;
    height: calc(100% - 20px);
  }

  .splitview :global([data-pane-resizer][data-direction='vertical'])::before {
    width: calc(100% - 20px);
    height: 2px;
  }

  .splitview :global([data-pane-resizer]:hover)::before,
  .splitview :global([data-pane-resizer][data-active])::before,
  .splitview[data-dragging='true'] :global([data-pane-resizer])::before,
  .splitview :global([data-pane-resizer]:focus-visible)::before {
    background: color-mix(in oklch, var(--primary) 40%, transparent);
    opacity: 1;
  }

  .splitview :global([data-pane-resizer][data-direction='horizontal']:hover)::before,
  .splitview :global([data-pane-resizer][data-direction='horizontal'][data-active])::before,
  .splitview[data-dragging='true']
    :global([data-pane-resizer][data-direction='horizontal'])::before,
  .splitview :global([data-pane-resizer][data-direction='horizontal']:focus-visible)::before {
    transform: scaleX(1.2);
  }

  .splitview :global([data-pane-resizer][data-direction='vertical']:hover)::before,
  .splitview :global([data-pane-resizer][data-direction='vertical'][data-active])::before,
  .splitview[data-dragging='true']
    :global([data-pane-resizer][data-direction='vertical'])::before,
  .splitview :global([data-pane-resizer][data-direction='vertical']:focus-visible)::before {
    transform: scaleY(1.2);
  }

  .splitview :global([data-pane-resizer][data-enabled='false']) {
    cursor: default;
    opacity: 0.7;
  }
</style>
