<script lang="ts">
  import DOMPurify from 'dompurify';
  import { marked } from 'marked';

  import { Tabs } from 'bits-ui';

  type RenderMode = 'plain' | 'markdown';

  type Props = {
    title?: string;
    text: string;
    mode?: RenderMode;
    wrap?: boolean;
    initialTab?: 'rendered' | 'text';
  };

  let {
    title,
    text,
    mode = 'plain',
    wrap = true,
    initialTab = 'rendered'
  }: Props = $props();

  let tab = $state<'rendered' | 'text'>('rendered');

  const hasMarkdown = $derived(mode === 'markdown');
  let renderedHtml = $state('');

  $effect(() => {
    tab = initialTab;
  });

  $effect(() => {
    if (!hasMarkdown) {
      renderedHtml = '';
      return;
    }

    let cancelled = false;
    const maybeHtml = marked.parse(text ?? '', {
      gfm: true,
      breaks: true
    });

    if (typeof maybeHtml === 'string') {
      renderedHtml = DOMPurify.sanitize(maybeHtml);
      return;
    }

    void maybeHtml
      .then((html) => {
        if (cancelled) {
          return;
        }
        renderedHtml = DOMPurify.sanitize(html);
      })
      .catch(() => {
        if (cancelled) {
          return;
        }
        renderedHtml = '';
      });

    return () => {
      cancelled = true;
    };
  });
</script>

<div class="viewer">
  {#if title}
    <div class="viewer__title">{title}</div>
  {/if}

  {#if hasMarkdown}
    <Tabs.Root value={tab} onValueChange={(next) => (tab = next as typeof tab)}>
      <Tabs.List class="tabs" aria-label="Preview tabs">
        <Tabs.Trigger
          class={['tab', tab === 'rendered' ? 'tab--active' : ''].join(' ')}
          value="rendered"
        >
          Rendered
        </Tabs.Trigger>
        <Tabs.Trigger
          class={['tab', tab === 'text' ? 'tab--active' : ''].join(' ')}
          value="text"
        >
          Text
        </Tabs.Trigger>
      </Tabs.List>

      <Tabs.Content class="tabs__panel" value="rendered">
        <div class="markdown" class:markdown--wrap={wrap}>
          {@html renderedHtml}
        </div>
      </Tabs.Content>

      <Tabs.Content class="tabs__panel" value="text">
        <pre class={['preview', wrap ? 'preview--wrap' : ''].join(' ')}>{text}</pre>
      </Tabs.Content>
    </Tabs.Root>
  {:else}
    <pre class={['preview', wrap ? 'preview--wrap' : ''].join(' ')}>{text}</pre>
  {/if}
</div>

<style>
  .viewer {
    display: grid;
    gap: 10px;
    min-height: 0;
  }

  .viewer__title {
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }

  .tabs {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--surface-strong) 65%, white);
    border: 1px solid color-mix(in oklch, var(--line) 78%, white);
    width: fit-content;
  }

  .tab {
    appearance: none;
    border: 0;
    cursor: pointer;
    padding: 7px 12px;
    border-radius: 999px;
    font-size: 13px;
    letter-spacing: -0.01em;
    color: var(--ink-soft);
    background: transparent;
  }

  .tab--active {
    color: var(--ink);
    background: color-mix(in oklch, var(--primary) 14%, white);
    box-shadow: 0 10px 24px rgba(40, 34, 24, 0.14);
  }

  .tab:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--primary) 38%, transparent);
    outline-offset: 2px;
  }

  .tabs__panel {
    min-height: 0;
  }

  .preview {
    height: 100%;
    margin: 0;
    padding: 14px;
    border-radius: 18px;
    background: color-mix(in oklch, white 86%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 68%, white);
    overflow: auto;
    white-space: pre-wrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    font-size: 13px;
    line-height: 1.55;
    color: var(--ink);
  }

  .preview--wrap {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .markdown {
    height: 100%;
    padding: 14px;
    border-radius: 18px;
    background: color-mix(in oklch, white 92%, transparent);
    border: 1px solid color-mix(in oklch, var(--line) 68%, white);
    overflow: auto;
    color: var(--ink);
  }

  .markdown--wrap {
    overflow-wrap: anywhere;
  }

  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3) {
    font-family: 'Manrope', sans-serif;
    letter-spacing: -0.03em;
  }

  .markdown :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
    font-size: 0.95em;
    background: color-mix(in oklch, var(--primary) 10%, white);
    border: 1px solid color-mix(in oklch, var(--line) 66%, white);
    padding: 0.1em 0.35em;
    border-radius: 10px;
  }

  .markdown :global(pre) {
    background: color-mix(in oklch, var(--surface-strong) 70%, white);
    border: 1px solid color-mix(in oklch, var(--line) 72%, white);
    padding: 12px 14px;
    border-radius: 18px;
    overflow: auto;
  }
</style>
