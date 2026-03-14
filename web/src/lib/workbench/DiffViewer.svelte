<script lang="ts">
  import { createTwoFilesPatch } from 'diff';

  type Props = {
    original: string;
    modified: string;
    fromLabel?: string;
    toLabel?: string;
    contextLines?: number;
  };

  let {
    original,
    modified,
    fromLabel = 'saved',
    toLabel = 'edited',
    contextLines = 3
  }: Props = $props();

  const patch = $derived.by(() => {
    if ((original ?? '') === (modified ?? '')) {
      return '';
    }

    return createTwoFilesPatch(fromLabel, toLabel, original ?? '', modified ?? '', '', '', {
      context: contextLines
    });
  });
</script>

{#if patch.length === 0}
  <div class="diff-viewer__empty">（无改动）</div>
{:else}
  <pre class="diff-viewer">{patch}</pre>
{/if}

