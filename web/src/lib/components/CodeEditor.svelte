<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import { Compartment } from '@codemirror/state';
  import { EditorView, basicSetup } from 'codemirror';
  import { jinja } from '@codemirror/lang-jinja';

  type Props = {
    value: string;
    readonly?: boolean;
    onChange?: (next: string) => void;
  };

  let { value, readonly = false, onChange }: Props = $props();

  let host: HTMLDivElement | null = null;
  let view: EditorView | null = null;
  let lastFromEditor = '';

  const editable = new Compartment();

  const theme = EditorView.theme(
    {
      '&.cm-editor': {
        height: '100%',
        background: 'transparent',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace',
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

  function attachEditor(): void {
    if (!host) {
      return;
    }

    view = new EditorView({
      parent: host,
      doc: value,
      extensions: [
        basicSetup,
        theme,
        jinja(),
        EditorView.lineWrapping,
        editable.of(EditorView.editable.of(!readonly)),
        EditorView.updateListener.of((update) => {
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
  }

  function detachEditor(): void {
    view?.destroy();
    view = null;
  }

  onMount(() => {
    attachEditor();
  });

  onDestroy(() => {
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

    view.dispatch({
      effects: editable.reconfigure(EditorView.editable.of(!readonly))
    });
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    border-radius: 18px;
    background: color-mix(in oklch, var(--surface) 70%, white);
    border: 1px solid color-mix(in oklch, var(--line) 70%, white);
  }

  .editor :global(.cm-editor) {
    border-radius: 18px;
  }
</style>
