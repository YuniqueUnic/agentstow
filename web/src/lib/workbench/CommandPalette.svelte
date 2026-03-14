<script lang="ts">
  import { tick } from 'svelte';

  export type PaletteCommand = {
    id: string;
    group: string;
    title: string;
    subtitle?: string;
    keywords?: string;
    disabled?: boolean;
    run: () => void | Promise<void>;
  };

  type Props = {
    open: boolean;
    commands: PaletteCommand[];
    onClose: () => void;
  };

  let { open, commands, onClose }: Props = $props();

  let query = $state('');
  let activeIndex = $state(0);
  let wasOpen = $state(false);
  let listEl = $state<HTMLDivElement | null>(null);
  let fieldEl = $state<HTMLInputElement | null>(null);

  function normalize(value: string): string {
    return value.trim().toLowerCase();
  }

  function scoreCommand(cmd: PaletteCommand, tokens: string[]): number | null {
    const hay = normalize(
      [cmd.group, cmd.title, cmd.subtitle ?? '', cmd.keywords ?? ''].join(' ')
    );

    let score = 0;
    for (const tok of tokens) {
      if (!tok) {
        continue;
      }
      const idx = hay.indexOf(tok);
      if (idx === -1) {
        return null;
      }
      score += idx === 0 ? 20 : idx < 12 ? 10 : 4;
    }

    if (cmd.disabled) {
      score -= 6;
    }
    return score;
  }

  const results = $derived.by(() => {
    if (!open) {
      return [];
    }

    const tokens = normalize(query).split(/\s+/).filter(Boolean);
    const scored: Array<{ cmd: PaletteCommand; score: number }> = [];
    for (const cmd of commands) {
      if (tokens.length === 0) {
        scored.push({ cmd, score: cmd.disabled ? -1 : 0 });
        continue;
      }

      const score = scoreCommand(cmd, tokens);
      if (score === null) {
        continue;
      }
      scored.push({ cmd, score });
    }

    scored.sort((a, b) => b.score - a.score || a.cmd.group.localeCompare(b.cmd.group));
    return scored.map((it) => it.cmd);
  });

  async function focusField(): Promise<void> {
    await tick();
    fieldEl?.focus?.();
  }

  function close(): void {
    onClose();
  }

  async function execute(cmd: PaletteCommand | null | undefined): Promise<void> {
    if (!cmd || cmd.disabled) {
      return;
    }
    close();
    await cmd.run();
  }

  function clampActiveIndex(): void {
    if (!results.length) {
      activeIndex = 0;
      return;
    }
    activeIndex = Math.min(Math.max(activeIndex, 0), results.length - 1);
  }

  $effect(() => {
    if (open && !wasOpen) {
      query = '';
      activeIndex = 0;
      void focusField();
    }
    wasOpen = open;
  });

  $effect(() => {
    if (!open) {
      return;
    }
    clampActiveIndex();

    const active = listEl?.querySelector('[data-active="true"]') as HTMLElement | null;
    active?.scrollIntoView({ block: 'nearest' });
  });
</script>

{#if open}
  <div
    class="palette-backdrop"
    role="presentation"
    onclick={close}
  >
    <div
      class="palette surface"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => {
        event.stopPropagation();

        if (event.key === 'Escape') {
          event.preventDefault();
          close();
          return;
        }

        const isMod = event.metaKey || event.ctrlKey;
        if (isMod && event.key.toLowerCase() === 'k') {
          event.preventDefault();
          close();
          return;
        }

        if (event.key === 'ArrowDown') {
          event.preventDefault();
          activeIndex = Math.min(activeIndex + 1, Math.max(0, results.length - 1));
          return;
        }

        if (event.key === 'ArrowUp') {
          event.preventDefault();
          activeIndex = Math.max(activeIndex - 1, 0);
          return;
        }

        if (event.key === 'Enter') {
          event.preventDefault();
          void execute(results[activeIndex]);
        }
      }}
    >
      <div class="palette__head">
        <span class="palette__prompt mono" aria-hidden="true">&gt;</span>
        <input
          bind:this={fieldEl}
          class="palette__input"
          type="text"
          placeholder="输入命令，或搜索 artifacts/targets…"
          value={query}
          oninput={(event) => {
            const target = event.currentTarget as HTMLInputElement | null;
            query = target?.value ?? '';
            activeIndex = 0;
          }}
        />
      </div>

      <div class="palette__results" bind:this={listEl} role="listbox" aria-label="Commands">
        {#if results.length === 0}
          <div class="palette__empty muted">（没有匹配结果）</div>
        {:else}
          {#each results as cmd, idx (cmd.id)}
            {#if idx === 0 || cmd.group !== results[idx - 1]?.group}
              <div class="palette__group">{cmd.group}</div>
            {/if}
            <button
              class={[
                'palette__item',
                idx === activeIndex ? 'palette__item--active' : ''
              ].join(' ')}
              type="button"
              role="option"
              aria-selected={idx === activeIndex}
              disabled={cmd.disabled}
              data-active={idx === activeIndex}
              onclick={() => void execute(cmd)}
              onmousemove={() => (activeIndex = idx)}
              title={cmd.subtitle ?? cmd.title}
            >
              <div class="palette__item-main">
                <div class="palette__item-title">{cmd.title}</div>
                {#if cmd.subtitle}
                  <div class="palette__item-sub mono">{cmd.subtitle}</div>
                {/if}
              </div>
              {#if cmd.disabled}
                <span class="palette__item-hint muted">disabled</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>

      <div class="palette__foot muted">
        Enter 执行 · ↑↓ 选择 · Esc 关闭 · Cmd/Ctrl+K 切换
      </div>
    </div>
  </div>
{/if}
