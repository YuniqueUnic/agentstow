<script lang="ts">
  import Self from './ArtifactTreeNode.svelte';

  import type { ArtifactTreeNode } from '$lib/workbench/artifacts/artifact_tree';

  type Props = {
    node: ArtifactTreeNode;
    depth: number;
    activeArtifactId: string | null;
    searchActive: boolean;
    onOpenArtifact: (artifactId: string) => void;
  };

  let { node, depth, activeArtifactId, searchActive, onOpenArtifact }: Props = $props();

  const isActiveFile = $derived(node.kind === 'file' && node.artifact.id === activeArtifactId);
  const defaultOpen = $derived(searchActive || depth <= 0);
</script>

<li class="tree__item" style={`--depth:${depth}`}>
  {#if node.kind === 'dir'}
    <details class="tree__dir" open={defaultOpen}>
      <summary class="tree__row tree__row--dir">
        <span class="tree__chevron" aria-hidden="true">▸</span>
        <span class="tree__name">{node.name}</span>
        <span class="tree__meta mono">{node.children.length}</span>
      </summary>

      <ul class="tree__children">
        {#each node.children as child (child.path)}
          <Self
            node={child}
            depth={depth + 1}
            activeArtifactId={activeArtifactId}
            searchActive={searchActive}
            onOpenArtifact={onOpenArtifact}
          />
        {/each}
      </ul>
    </details>
  {:else}
    <button
      class={['tree__row', 'tree__row--file', isActiveFile ? 'tree__row--active' : ''].join(' ')}
      data-testid={`artifact-tree-item:${node.artifact.id}`}
      onclick={() => onOpenArtifact(node.artifact.id)}
      type="button"
      title={`${node.artifact.id} · ${node.rel_source_path}`}
    >
      <span class="tree__dot" aria-hidden="true"></span>
      <span class="tree__name">{node.name}</span>
      <span class="tree__meta mono">{node.artifact.id}</span>
      <span class="tree__meta">{node.artifact.validate_as}</span>
    </button>
  {/if}
</li>
