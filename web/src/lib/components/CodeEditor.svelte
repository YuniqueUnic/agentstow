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
  let cmModule: typeof import('codemirror') | null = null;
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
      cmModule = cm;

      const theme = cm.EditorView.theme(
        {
          '&.cm-editor': {
            height: '100%',
            background: 'transparent',
            color: 'var(--ink)',
            fontFamily: '"IBM Plex Mono", "SFMono-Regular", monospace',
            fontSize: '13px'
          },
          '.cm-scroller': {
            overflow: 'auto'
          },
          '.cm-content': {
            padding: '16px 18px 36px'
          },
          '.cm-line': {
            padding: '0 8px'
          },
          '.cm-gutters': {
            background: 'transparent',
            border: 'none',
            color: 'color-mix(in oklch, var(--ink-muted) 88%, transparent)'
          },
          '.cm-activeLine': {
            background: 'color-mix(in oklch, var(--primary) 12%, transparent)'
          },
          '.cm-activeLineGutter': {
            background: 'transparent',
            color: 'var(--ink)'
          },
          '.cm-selectionBackground': {
            background: 'color-mix(in oklch, var(--primary) 32%, black)'
          },
          '&.cm-focused .cm-selectionBackground': {
            background: 'color-mix(in oklch, var(--primary) 36%, black)'
          },
          '&.cm-focused': {
            outline: '1px solid color-mix(in oklch, var(--primary) 42%, transparent)',
            borderRadius: '12px',
            boxShadow: '0 0 0 3px color-mix(in oklch, var(--primary) 16%, transparent)'
          },
          '.cm-cursor': {
            borderLeftColor: 'color-mix(in oklch, var(--ink) 78%, transparent)'
          },
          '.cm-tooltip, .cm-panels': {
            background: 'var(--panel-elevated)',
            color: 'var(--ink)',
            border: '1px solid color-mix(in oklch, var(--line) 90%, transparent)'
          }
        },
        { dark: true }
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

    if (!cmModule) {
      return;
    }

    view.dispatch({
      effects: editable.reconfigure(cmModule.EditorView.editable.of(!readonly))
    });
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
        <span class="editor__spinner" aria-hidden="true"></span>
        <div class="editor__loading-title">加载编辑器…</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor {
    height: 100%;
    position: relative;
    border-radius: 12px;
    background:
      linear-gradient(180deg, color-mix(in oklch, var(--panel-elevated) 78%, var(--panel-bg)), var(--panel-bg));
    border: 1px solid color-mix(in oklch, var(--line) 86%, transparent);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .editor__host {
    height: 100%;
  }

  .editor :global(.cm-editor) {
    border-radius: 12px;
  }

  .editor__loading {
    position: absolute;
    inset: 12px;
    border-radius: 10px;
    border: 1px dashed color-mix(in oklch, var(--line) 70%, transparent);
    background: color-mix(in oklch, var(--canvas-deep) 62%, transparent);
    display: grid;
    place-items: center;
    gap: 10px;
    padding: 16px;
    text-align: center;
    color: var(--ink-soft);
  }

  .editor__spinner {
    width: 18px;
    height: 18px;
    border-radius: 999px;
    border: 2px solid color-mix(in oklch, var(--line-strong) 42%, transparent);
    border-top-color: var(--primary);
    animation: editor-spin 0.9s linear infinite;
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
    border-color: color-mix(in oklch, var(--danger) 34%, transparent);
    background: color-mix(in oklch, var(--danger) 10%, transparent);
    color: color-mix(in oklch, var(--danger) 70%, var(--ink));
  }

  @keyframes editor-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
