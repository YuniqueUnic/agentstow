import type {
  McpRenderResponse,
  McpServerSummaryResponse,
  McpTestResponse,
  McpValidateResponse,
  WorkspaceSummaryResponse
} from '../../src/lib/types';

export const workspaceSummaryFixture: WorkspaceSummaryResponse = {
  workspace_root: '/tmp/agentstow-workspace',
  counts: {
    profile_count: 1,
    artifact_count: 2,
    target_count: 1,
    env_emit_set_count: 1,
    script_count: 1,
    mcp_server_count: 1,
    link_count: 0,
    healthy_link_count: 0,
    unhealthy_link_count: 0
  },
  profiles: [
    {
      id: 'base',
      extends: [],
      variable_keys: ['name'],
      target_ids: ['hello-target'],
      artifact_ids: ['hello', 'bye']
    }
  ],
  artifacts: [
    {
      id: 'hello',
      kind: 'file',
      source_path: '/tmp/agentstow-workspace/artifacts/hello.txt.tera',
      template: true,
      validate_as: 'none',
      target_ids: ['hello-target'],
      profiles: ['base']
    },
    {
      id: 'bye',
      kind: 'file',
      source_path: '/tmp/agentstow-workspace/artifacts/bye.txt.tera',
      template: false,
      validate_as: 'none',
      target_ids: [],
      profiles: ['base']
    }
  ],
  targets: [
    {
      id: 'hello-target',
      artifact_id: 'hello',
      profile: 'base',
      target_path: '/tmp/project/hello.txt',
      method: 'copy'
    }
  ],
  env_emit_sets: [
    {
      id: 'default',
      vars: [
        {
          key: 'OPENAI_API_KEY',
          binding: 'env:OPENAI_API_KEY',
          binding_kind: 'env',
          source_env_var: 'OPENAI_API_KEY',
          rendered_placeholder: '${OPENAI_API_KEY}',
          available: false,
          diagnostic: '缺少环境变量：OPENAI_API_KEY',
          referrers: [
            {
              owner_kind: 'env_emit_set',
              owner_id: 'default',
              label: 'Env emit default'
            },
            {
              owner_kind: 'script',
              owner_id: 'sync',
              label: 'Script sync'
            },
            {
              owner_kind: 'mcp_server',
              owner_id: 'local',
              label: 'MCP local'
            }
          ]
        }
      ],
      available_count: 0,
      missing_count: 1,
      referrers: [
        {
          owner_kind: 'script',
          owner_id: 'sync',
          label: 'Script sync'
        },
        {
          owner_kind: 'mcp_server',
          owner_id: 'local',
          label: 'MCP local'
        }
      ]
    }
  ],
  scripts: [
    {
      id: 'sync',
      kind: 'shell',
      entry: 'python3',
      args: ['-c', 'print("sync")'],
      cwd_policy: 'workspace',
      env_keys: ['OPENAI_API_KEY'],
      env_bindings: [
        {
          key: 'OPENAI_API_KEY',
          binding: 'env:OPENAI_API_KEY',
          binding_kind: 'env',
          source_env_var: 'OPENAI_API_KEY',
          rendered_placeholder: '${OPENAI_API_KEY}',
          available: false,
          diagnostic: '缺少环境变量：OPENAI_API_KEY',
          referrers: [
            {
              owner_kind: 'script',
              owner_id: 'sync',
              label: 'Script sync'
            }
          ]
        }
      ],
      stdin_mode: 'text',
      stdout_mode: 'capture',
      stderr_mode: 'capture',
      timeout_ms: null,
      expected_exit_codes: [0]
    }
  ],
  mcp_servers: [
    {
      id: 'local',
      transport_kind: 'stdio',
      location: '/tmp/agentstow-workspace/agentstow.toml',
      command: 'npx',
      args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
      url: null,
      headers: [],
      env_keys: ['OPENAI_API_KEY'],
      env_bindings: [
        {
          key: 'OPENAI_API_KEY',
          binding: 'env:OPENAI_API_KEY',
          binding_kind: 'env',
          source_env_var: 'OPENAI_API_KEY',
          rendered_placeholder: '${OPENAI_API_KEY}',
          available: false,
          diagnostic: '缺少环境变量：OPENAI_API_KEY',
          referrers: [
            {
              owner_kind: 'mcp_server',
              owner_id: 'local',
              label: 'MCP local'
            }
          ]
        }
      ]
    }
  ],
  issues: []
};

export const mcpServerFixture: McpServerSummaryResponse = workspaceSummaryFixture.mcp_servers[0];

export const mcpRenderFixture: McpRenderResponse = {
  server_id: 'local',
  transport_kind: 'stdio',
  launcher_preview: 'npx -y @modelcontextprotocol/server-filesystem .',
  config_json: JSON.stringify(
    {
      mcpServers: {
        local: {
          command: 'npx',
          args: ['-y', '@modelcontextprotocol/server-filesystem', '.'],
          env: {
            OPENAI_API_KEY: '${OPENAI_API_KEY}'
          }
        }
      }
    },
    null,
    2
  ),
  env_bindings: mcpServerFixture.env_bindings
};

export const mcpValidateFixture: McpValidateResponse = {
  server_id: 'local',
  ok: false,
  issues: [
    {
      severity: 'warn',
      scope: 'mcp_server',
      subject_id: 'local',
      code: 'mcp_env_unavailable',
      message: 'OPENAI_API_KEY 尚未绑定到当前运行环境。'
    }
  ]
};

export const envEmitSetFixture = workspaceSummaryFixture.env_emit_sets[0];

export const mcpTestFixture: McpTestResponse = {
  server_id: 'local',
  ok: false,
  checks: [
    {
      code: 'validate',
      status: 'ok',
      message: 'Codex transport 校验通过',
      detail: null
    },
    {
      code: 'launcher',
      status: 'ok',
      message: 'stdio launcher 已解析',
      detail: 'npx -y @modelcontextprotocol/server-filesystem .'
    },
    {
      code: 'render',
      status: 'ok',
      message: 'Codex 单 server 配置可渲染',
      detail: null
    },
    {
      code: 'env:OPENAI_API_KEY',
      status: 'error',
      message: '环境变量 `OPENAI_API_KEY` 缺失',
      detail: '缺少环境变量：OPENAI_API_KEY'
    }
  ]
};
