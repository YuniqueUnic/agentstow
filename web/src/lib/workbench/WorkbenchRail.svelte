<script lang="ts">
  type ViewKey = 'artifacts' | 'links' | 'env' | 'scripts' | 'mcp' | 'impact';

  type Props = {
    view: ViewKey;
    onChange: (next: ViewKey) => void;
  };

  let { view, onChange }: Props = $props();

  const items = [
    ['artifacts', 'A'],
    ['links', 'L'],
    ['env', 'E'],
    ['scripts', 'S'],
    ['mcp', 'M'],
    ['impact', 'I']
  ] as const satisfies ReadonlyArray<readonly [ViewKey, string]>;
</script>

<nav class="rail surface" aria-label="主导航">
  {#each items as [key, label] (key)}
    <button
      class={['rail__item', view === key ? 'rail__item--active' : ''].join(' ')}
      onclick={() => onChange(key)}
      type="button"
    >
      <span class="rail__glyph" aria-hidden="true">{label}</span>
      <span class="rail__label">{key}</span>
    </button>
  {/each}
</nav>

