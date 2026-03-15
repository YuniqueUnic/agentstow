<script lang="ts">
  import { structuredPatch } from 'diff';

  type Props = {
    original: string;
    modified: string;
    fromLabel?: string;
    toLabel?: string;
    contextLines?: number;
    testId?: string;
  };

  type DiffRowKind = 'context' | 'add' | 'remove' | 'meta';

  type DiffRow = {
    kind: DiffRowKind;
    marker: string;
    oldNumber: number | null;
    newNumber: number | null;
    text: string;
  };

  type DiffHunk = {
    header: string;
    rows: DiffRow[];
  };

  let {
    original,
    modified,
    fromLabel = 'saved',
    toLabel = 'edited',
    contextLines = 3,
    testId
  }: Props = $props();

  function buildHunkHeader(
    oldStart: number,
    oldLines: number,
    newStart: number,
    newLines: number,
    section?: string
  ): string {
    const base = `@@ -${oldStart},${oldLines} +${newStart},${newLines} @@`;
    return section ? `${base} ${section}` : base;
  }

  function buildDiffHunks(): DiffHunk[] {
    if ((original ?? '') === (modified ?? '')) {
      return [];
    }

    const patch = structuredPatch(
      fromLabel,
      toLabel,
      original ?? '',
      modified ?? '',
      '',
      '',
      {
        context: contextLines
      }
    );

    return patch.hunks.map((hunk) => {
      let oldLine = hunk.oldStart;
      let newLine = hunk.newStart;

      const rows = hunk.lines.map((line): DiffRow => {
        const marker = line[0] ?? ' ';

        if (marker === '-') {
          const row = {
            kind: 'remove' as const,
            marker,
            oldNumber: oldLine,
            newNumber: null,
            text: line.slice(1)
          };
          oldLine += 1;
          return row;
        }

        if (marker === '+') {
          const row = {
            kind: 'add' as const,
            marker,
            oldNumber: null,
            newNumber: newLine,
            text: line.slice(1)
          };
          newLine += 1;
          return row;
        }

        if (marker === ' ') {
          const row = {
            kind: 'context' as const,
            marker,
            oldNumber: oldLine,
            newNumber: newLine,
            text: line.slice(1)
          };
          oldLine += 1;
          newLine += 1;
          return row;
        }

        return {
          kind: 'meta',
          marker,
          oldNumber: null,
          newNumber: null,
          text: line
        };
      });

      return {
        header: buildHunkHeader(hunk.oldStart, hunk.oldLines, hunk.newStart, hunk.newLines),
        rows
      };
    });
  }

  const hunks = $derived.by(buildDiffHunks);
</script>

{#if hunks.length === 0}
  <div class="diff-viewer diff-viewer--empty" data-testid={testId}>（无改动）</div>
{:else}
  <div class="diff-viewer" aria-label="Structured diff preview" data-testid={testId}>
    {#each hunks as hunk, hunkIndex (`${hunk.header}:${hunkIndex}`)}
      <section class="diff-hunk">
        <div class="diff-hunk__header mono">{hunk.header}</div>
        <div class="diff-hunk__rows">
          {#each hunk.rows as row, rowIndex (`${hunkIndex}:${rowIndex}:${row.marker}:${row.oldNumber ?? 'x'}:${row.newNumber ?? 'x'}`)}
            <div class={['diff-row', `diff-row--${row.kind}`].join(' ')}>
              <span class="diff-row__line mono">{row.oldNumber ?? ''}</span>
              <span class="diff-row__line mono">{row.newNumber ?? ''}</span>
              <span class="diff-row__marker mono">{row.marker}</span>
              <span class="diff-row__content mono">{row.text || ' '}</span>
            </div>
          {/each}
        </div>
      </section>
    {/each}
  </div>
{/if}
