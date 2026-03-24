import type { WorkspaceSummaryResponse } from '$lib/types';

export type ManifestInsertKind =
  | 'profile'
  | 'artifact'
  | 'target'
  | 'env_set'
  | 'script'
  | 'mcp_server';

function nextId(base: string, existing: string[]): string {
  if (!existing.includes(base)) {
    return base;
  }

  let index = 2;
  while (existing.includes(`${base}_${index}`)) {
    index += 1;
  }
  return `${base}_${index}`;
}

function section(text: string): string {
  return `${text.trim()}\n`;
}

export function manifestInsertLabel(kind: ManifestInsertKind): string {
  switch (kind) {
    case 'profile':
      return 'profile';
    case 'artifact':
      return 'artifact';
    case 'target':
      return 'target';
    case 'env_set':
      return 'env export set';
    case 'script':
      return 'script';
    case 'mcp_server':
      return 'MCP server';
  }
}

export function buildManifestSnippet(
  kind: ManifestInsertKind,
  summary: WorkspaceSummaryResponse | null
): string {
  const profileIds = summary?.profiles.map((profile) => profile.id) ?? [];
  const artifactIds = summary?.artifacts.map((artifact) => artifact.id) ?? [];
  const targetIds = summary?.targets.map((target) => target.id) ?? [];
  const envSetIds = summary?.env_emit_sets.map((envSet) => envSet.id) ?? [];
  const scriptIds = summary?.scripts.map((script) => script.id) ?? [];
  const mcpIds = summary?.mcp_servers.map((server) => server.id) ?? [];

  const firstProfile = profileIds[0] ?? 'base';
  const firstArtifact = artifactIds[0] ?? 'hello';
  const firstEnvKey = summary?.env_emit_sets[0]?.vars[0]?.key ?? 'OPENAI_API_KEY';

  switch (kind) {
    case 'profile': {
      const id = nextId('new_profile', profileIds);
      return section(`
[profiles.${id}]
extends = ["${firstProfile}"]
vars = { example = "value" }
`);
    }
    case 'artifact': {
      const id = nextId('new_artifact', artifactIds);
      return section(`
[artifacts.${id}]
kind = "file"
source = "artifacts/${id}.md.tera"
template = true
validate_as = "markdown"
`);
    }
    case 'target': {
      const id = nextId('new_target', targetIds);
      return section(`
[targets.${id}]
artifact = "${firstArtifact}"
profile = "${firstProfile}"
target_path = "targets/${id}.md"
method = "copy"
`);
    }
    case 'env_set': {
      const id = nextId('default', envSetIds);
      return section(`
[env.emit.${id}]
# kind = "env" 会读取 agentstow serve 进程当前继承到的宿主环境；修改后需要重启服务重新探测。
# 开发期如果想直接写值，可改成：{ key = "INLINE_EXAMPLE", binding = { kind = "literal", value = "replace-me" } }
vars = [
  { key = "${firstEnvKey}", binding = { kind = "env", var = "${firstEnvKey}" } }
]
`);
    }
    case 'script': {
      const id = nextId('sync', scriptIds);
      return section(`
[scripts.${id}]
kind = "shell"
entry = "echo"
args = ["hello from ${id}"]
cwd_policy = "current"
stdin_mode = "none"
stdout_mode = "capture"
stderr_mode = "capture"
timeout_ms = 5000
expected_exit_codes = [0]
`);
    }
    case 'mcp_server': {
      const id = nextId('local', mcpIds);
      return section(`
[mcp_servers.${id}]
transport = { kind = "stdio", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "."] }
env = [
  { key = "${firstEnvKey}", binding = { kind = "env", var = "${firstEnvKey}" } }
]
`);
    }
  }
}
