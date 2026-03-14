<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  type Props = {
    left: Snippet;
    right: Snippet;
    initialLeftPct?: number;
    minLeftPx?: number;
    minRightPx?: number;
    disabled?: boolean;
  };

  let {
    left,
    right,
    initialLeftPct = 52,
    minLeftPx = 320,
    minRightPx = 320,
    disabled = false
  }: Props = $props();

  let host: HTMLDivElement | null = null;
  let gutter: HTMLButtonElement | null = null;

  let leftPct = $state<number | null>(null);
  let dragging = $state(false);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function clampLeftPct(nextPct: number): number {
    if (!host) {
      return clamp(nextPct, 0, 100);
    }

    const rect = host.getBoundingClientRect();
    const width = rect.width;
    if (width <= 0) {
      return clamp(nextPct, 0, 100);
    }

    const minPct = (minLeftPx / width) * 100;
    const maxPct = 100 - (minRightPx / width) * 100;
    return clamp(nextPct, minPct, maxPct);
  }

  function updateFromClientX(clientX: number): void {
    if (!host) {
      return;
    }

    const rect = host.getBoundingClientRect();
    const width = rect.width;
    if (width <= 0) {
      return;
    }

    const nextPct = ((clientX - rect.left) / width) * 100;
    leftPct = clampLeftPct(nextPct);
  }

  function onPointerDown(event: PointerEvent): void {
    if (disabled) {
      return;
    }
    if (!gutter) {
      return;
    }

    dragging = true;
    gutter.setPointerCapture(event.pointerId);
    updateFromClientX(event.clientX);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!dragging) {
      return;
    }
    updateFromClientX(event.clientX);
  }

  function onPointerUp(event: PointerEvent): void {
    dragging = false;
    try {
      gutter?.releasePointerCapture(event.pointerId);
    } catch {
      // ignore
    }
  }

  function nudge(deltaPct: number): void {
    const current = leftPct ?? initialLeftPct;
    leftPct = clampLeftPct(current + deltaPct);
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (disabled) {
      return;
    }

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      nudge(-2);
      return;
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      nudge(2);
      return;
    }
    if (event.key === 'Home') {
      event.preventDefault();
      leftPct = clampLeftPct(initialLeftPct);
      return;
    }
  }

  onMount(() => {
    leftPct = clampLeftPct(initialLeftPct);
  });
</script>

<div class="splitview" bind:this={host} data-dragging={dragging ? 'true' : 'false'}>
  <div
    class="splitview__pane splitview__pane--left"
    style={`flex-basis: ${leftPct ?? initialLeftPct}%;`}
  >
    {@render left()}
  </div>
  <button
    type="button"
    class="splitview__gutter"
    bind:this={gutter}
    aria-label="调整分屏宽度"
    disabled={disabled}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
    onkeydown={onKeyDown}
  ></button>
  <div class="splitview__pane splitview__pane--right">
    {@render right()}
  </div>
</div>

<style>
  .splitview {
    height: 100%;
    display: flex;
    min-height: 0;
    min-width: 0;
  }

  .splitview__pane {
    min-height: 0;
    min-width: 0;
    flex: 1 1 0;
  }

  .splitview__pane--left {
    flex: 0 0 auto;
  }

  .splitview__gutter {
    flex: 0 0 14px;
    width: 14px;
    padding: 0;
    position: relative;
    cursor: col-resize;
    border-radius: 999px;
    border: 0;
    background: transparent;
    outline: none;
  }

  .splitview__gutter::before {
    content: '';
    position: absolute;
    inset: 10px 0;
    width: 2px;
    left: calc(50% - 1px);
    border-radius: 999px;
    background: color-mix(in oklch, var(--line-strong) 30%, transparent);
    transition:
      background 140ms ease,
      transform 140ms ease,
      opacity 140ms ease;
    opacity: 0.9;
  }

  .splitview__gutter:hover::before,
  .splitview[data-dragging='true'] .splitview__gutter::before,
  .splitview__gutter:focus-visible::before {
    background: color-mix(in oklch, var(--primary) 40%, transparent);
    transform: scaleX(1.2);
    opacity: 1;
  }

  .splitview__gutter:disabled {
    cursor: default;
    opacity: 0.7;
  }
</style>
