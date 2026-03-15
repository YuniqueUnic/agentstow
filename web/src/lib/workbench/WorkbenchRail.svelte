<script lang="ts">
  type ViewKey = 'artifacts' | 'links' | 'env' | 'scripts' | 'mcp' | 'impact';

  type Props = {
    view: ViewKey;
    onChange: (next: ViewKey) => void;
  };

  let { view, onChange }: Props = $props();

  const items = [
    {
      key: 'artifacts',
      label: 'Artifacts',
      paths: ['M6 4.75h8.5L19.25 9v10.25H6z', 'M14 4.75V9h5.25']
    },
    {
      key: 'links',
      label: 'Links',
      paths: [
        'M10.5 13.5l-2 2a3 3 0 01-4.25-4.25l3-3a3 3 0 014.25 0',
        'M13.5 10.5l2-2a3 3 0 014.25 4.25l-3 3a3 3 0 01-4.25 0',
        'M8.5 15.5l7-7'
      ]
    },
    {
      key: 'env',
      label: 'Env',
      paths: ['M5 7.75h14', 'M8.5 7.75v8.5', 'M5 16.25h14', 'M15.5 7.75v8.5']
    },
    {
      key: 'scripts',
      label: 'Scripts',
      paths: ['M4.75 6.75h14.5v10.5H4.75z', 'M8.25 10.25l2 2-2 2', 'M12.75 14.25h3.25']
    },
    {
      key: 'mcp',
      label: 'MCP',
      paths: ['M9 5.75v4.5', 'M15 5.75v4.5', 'M7 14.25h10', 'M9 18.25h6', 'M6 10.25h12v4a2 2 0 01-2 2H8a2 2 0 01-2-2z']
    },
    {
      key: 'impact',
      label: 'Impact',
      paths: ['M6 17.25h12', 'M8.5 17.25V11', 'M12 17.25V7.75', 'M15.5 17.25v-5.5']
    }
  ] as const satisfies ReadonlyArray<{
    key: ViewKey;
    label: string;
    paths: readonly string[];
  }>;
</script>

<nav class="rail" aria-label="主导航">
  {#each items as item (item.key)}
    <button
      class={['rail__item', view === item.key ? 'rail__item--active' : ''].join(' ')}
      onclick={() => onChange(item.key)}
      type="button"
      title={item.label}
      aria-label={item.label}
      aria-current={view === item.key ? 'page' : undefined}
    >
      <span class="rail__glyph" aria-hidden="true">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.65"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          {#each item.paths as path (path)}
            <path d={path}></path>
          {/each}
        </svg>
      </span>
    </button>
  {/each}
</nav>
