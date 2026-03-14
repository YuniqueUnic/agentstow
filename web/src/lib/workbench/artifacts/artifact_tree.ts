import type { ArtifactSummaryResponse } from '$lib/types';

export type ArtifactTreeDirNode = {
  kind: 'dir';
  name: string;
  path: string;
  children: ArtifactTreeNode[];
};

export type ArtifactTreeFileNode = {
  kind: 'file';
  name: string;
  path: string;
  rel_source_path: string;
  artifact: ArtifactSummaryResponse;
};

export type ArtifactTreeNode = ArtifactTreeDirNode | ArtifactTreeFileNode;

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/\/{2,}/g, '/').replace(/\/+$/, '');
}

export function relativeSourcePath(sourcePath: string, workspaceRoot: string | null): string {
  const src = normalizePath(sourcePath);
  if (!workspaceRoot) {
    return src;
  }

  const root = normalizePath(workspaceRoot);
  const srcLower = src.toLowerCase();
  const rootLower = root.toLowerCase();

  if (srcLower === rootLower) {
    return '';
  }

  const prefix = `${rootLower}/`;
  if (!srcLower.startsWith(prefix)) {
    return src;
  }

  return src.slice(root.length).replace(/^\/+/, '');
}

export function buildArtifactTree(
  artifacts: ArtifactSummaryResponse[],
  workspaceRoot: string | null
): ArtifactTreeNode[] {
  const root: ArtifactTreeDirNode = { kind: 'dir', name: '', path: '', children: [] };

  const ensureDir = (parent: ArtifactTreeDirNode, name: string, path: string): ArtifactTreeDirNode => {
    const found = parent.children.find(
      (child): child is ArtifactTreeDirNode =>
        child.kind === 'dir' && child.path === path
    );
    if (found) {
      return found;
    }

    const next: ArtifactTreeDirNode = { kind: 'dir', name, path, children: [] };
    parent.children.push(next);
    return next;
  };

  for (const artifact of artifacts) {
    const rel = relativeSourcePath(artifact.source_path, workspaceRoot);
    const segments = rel.split('/').filter(Boolean);
    const fallbackName = artifact.id;
    const parts = segments.length ? segments : [fallbackName];

    let cursor = root;
    let dirPath = '';
    for (const seg of parts.slice(0, -1)) {
      dirPath = dirPath ? `${dirPath}/${seg}` : seg;
      cursor = ensureDir(cursor, seg, dirPath);
    }

    const fileName = parts.at(-1) ?? fallbackName;
    const filePath = dirPath ? `${dirPath}/${fileName}` : fileName;
    cursor.children.push({
      kind: 'file',
      name: fileName,
      path: filePath,
      rel_source_path: rel,
      artifact
    });
  }

  return sortArtifactTree(root.children);
}

export function sortArtifactTree(nodes: ArtifactTreeNode[]): ArtifactTreeNode[] {
  const sorted = [...nodes].sort((a, b) => {
    if (a.kind !== b.kind) {
      return a.kind === 'dir' ? -1 : 1;
    }
    return a.name.localeCompare(b.name, 'zh-CN');
  });

  for (const node of sorted) {
    if (node.kind === 'dir') {
      node.children = sortArtifactTree(node.children);
    }
  }

  return sorted;
}

