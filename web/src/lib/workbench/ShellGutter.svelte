<script lang="ts">
  type Props = {
    disabled?: boolean;
    onResize: (deltaPx: number) => void;
    onReset?: () => void;
  };

  let { disabled = false, onResize, onReset }: Props = $props();

  let dragging = $state(false);
  let lastX = 0;
  let handle: HTMLButtonElement | null = null;

  function startDrag(event: PointerEvent): void {
    if (disabled) {
      return;
    }

    dragging = true;
    lastX = event.clientX;
    handle?.setPointerCapture(event.pointerId);
  }

  function drag(event: PointerEvent): void {
    if (!dragging) {
      return;
    }

    const delta = event.clientX - lastX;
    lastX = event.clientX;
    onResize(delta);
  }

  function stopDrag(event: PointerEvent): void {
    dragging = false;
    try {
      handle?.releasePointerCapture(event.pointerId);
    } catch {
      // ignore
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (disabled) {
      return;
    }

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      onResize(-24);
      return;
    }

    if (event.key === 'ArrowRight') {
      event.preventDefault();
      onResize(24);
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      onReset?.();
    }
  }
</script>

<button
  bind:this={handle}
  type="button"
  class="shell-gutter"
  data-dragging={dragging ? 'true' : 'false'}
  aria-label="调整资源面板宽度"
  disabled={disabled}
  onpointerdown={startDrag}
  onpointermove={drag}
  onpointerup={stopDrag}
  onpointercancel={stopDrag}
  onkeydown={onKeyDown}
></button>
