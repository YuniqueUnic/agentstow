<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import type { Compartment as CompartmentType } from '@codemirror/state';
  import type { EditorView as EditorViewType } from 'codemirror';

  type Props = {
    value: string;
    readonly?: boolean;
    onChange?: (next: string) => void;
  };

  let { value, readonly = false, onChange }: Props = $props();

  let host: HTMLDivElement | null = null;
  let view: EditorViewType | null = null;
  let lastFromEditor = '';

  let editable: CompartmentType | null = null;
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let alive = true;

  async function attachEditor(): Promise<void> {
    if (!host) {
      return;
    }

    loading = true;
    loadError = null;

    try {
      const [{ Compartment }, cm, lang] = await Promise.all([
        import('@codemirror/state'),
        import('codemirror'),
        import('@codemirror/lang-jinja')
      ]);

      if (!alive || !host) {
        return;
      }

      editable = new Compartment();

      const theme = cm.EditorView.theme(
        {
          '&.cm-editor': {
            height: '100%',
            background: 'transparent',
            fontFamily:
              'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
            fontSize: '13px'
          },
          '.cm-scroller': {
            overflow: 'auto'
          },
          '.cm-content': {
            padding: '14px 14px 28px'
          },
          '.cm-line': {
            padding: '0 6px'
          },
          '.cm-gutters': {
            background: 'transparent',
            border: 'none',
            color: 'color-mix(in oklch, var(--ink-soft) 70%, transparent)'
          },
          '.cm-activeLine': {
            background:
              'color-mix(in oklch, var(--primary) 10%, color-mix(in oklch, white 80%, transparent))'
          },
          '.cm-activeLineGutter': {
            background: 'transparent',
            color: 'var(--ink)'
          },
          '.cm-selectionBackground': {
            background: 'color-mix(in oklch, var(--primary) 22%, white)'
          },
          '&.cm-focused .cm-selectionBackground': {
            background: 'color-mix(in oklch, var(--primary) 24%, white)'
          },
          '&.cm-focused': {
            outline: '1px solid color-mix(in oklch, var(--primary) 30%, transparent)',
            borderRadius: '16px',
            boxShadow: '0 0 0 6px color-mix(in oklch, var(--primary) 10%, transparent)'
          },
          '.cm-cursor': {
            borderLeftColor: 'color-mix(in oklch, var(--ink) 70%, transparent)'
          }
        },
        { dark: false }
      );

      view = new cm.EditorView({
        parent: host,
        doc: value,
        extensions: [
          cm.basicSetup,
          theme,
          lang.jinja(),
          cm.EditorView.lineWrapping,
          editable.of(cm.EditorView.editable.of(!readonly)),
          cm.EditorView.updateListener.of((update) => {
            if (!update.docChanged) {
              return;
            }

            const next = update.state.doc.toString();
            lastFromEditor = next;
            onChange?.(next);
          })
        ]
      });
      lastFromEditor = value;
    } catch (error) {
      loadError = error instanceof Error ? error.message : '编辑器加载失败。';
    } finally {
      loading = false;
    }
  }

  function detachEditor(): void {
    view?.destroy();
    view = null;
  }

  onMount(() => {
    void attachEditor();
  });

  onDestroy(() => {
    alive = false;
    detachEditor();
  });

  $effect(() => {
    if (!view) {
      return;
    }

    if (value === lastFromEditor) {
      return;
    }

    const current = view.state.doc.toString();
    if (value === current) {
      lastFromEditor = value;
      return;
    }

    view.dispatch({
      changes: {
        from: 0,
        to: view.state.doc.length,
        insert: value
      }
    });
    lastFromEditor = value;
  });

  $effect(() => {
    if (!view) {
      return;
    }

    if (!editable) {
      return;
    }

    void (async () => {
      const cm = await import('codemirror');
      if (!alive || !view || !editable) {
        return;
      }
      view.dispatch({
        effects: editable.reconfigure(cm.EditorView.editable.of(!readonly))
      });
    })();
  });
</script>

<div class="editor">
  <div class="editor__host" bind:this={host}></div>
  {#if loading || loadError}
    <div class={['editor__loading', loadError ? 'editor__loading--error' : ''].join(' ')}>
      {#if loadError}
        <div class="editor__loading-title">编辑器加载失败</div>
        <div class="editor__loading-detail mono">{loadError}</div>
      {:else}
        <md-circular-progress indeterminate></md-circular-progress>
        <div class="editor__loading-title">加载编辑器…</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor {
    height: 100%;
    position: relative;
    border-radius: 18px;
    background: color-mix(in oklch, var(--surface) 70%, white);
    border: 1px solid color-mix(in oklch, var(--line) 70%, white);
  }

  .editor__host {
    height: 100%;
  }

  .editor :global(.cm-editor) {
    border-radius: 18px;
  }

  .editor__loading {
    position: absolute;
    inset: 10px;
    border-radius: 16px;
    border: 1px dashed color-mix(in oklch, var(--line) 72%, white);
    background: color-mix(in oklch, white 86%, transparent);
    display: grid;
    place-items: center;
    gap: 10px;
    padding: 16px;
    text-align: center;
    color: var(--ink-soft);
  }

  .editor__loading-title {
    font-size: 13px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .editor__loading-detail {
    font-size: 12px;
    max-width: 60ch;
    overflow-wrap: anywhere;
  }

  .editor__loading--error {
    border-style: solid;
    border-color: color-mix(in oklch, var(--danger) 22%, white);
    background: color-mix(in oklch, var(--danger) 8%, white);
    color: color-mix(in oklch, var(--danger) 70%, var(--ink));
  }
</style>
