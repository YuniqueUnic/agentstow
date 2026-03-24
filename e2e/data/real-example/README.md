# real-example fixture

这个目录现在是 `e2e/data/` 下的完整 workspace fixture。e2e 测试会先把它复制到临时目录，再执行真实 `agentstow` CLI。

它直接对齐当前 `agentstow` 的真实能力：

- `[env.files] + [env]`：先从一个或多个 `.env` 文件加载变量，再由 `[env]` 里的直接声明覆盖同名值，统一注入模板上下文里的 `env.*`。
- `[env.emit.<name>]`：定义命名的 shell 导出集合；`agentstow env emit` 不带 `--set` 时会直接导出 `[env.files] + [env]` 的合并结果，带 `--set demo` 时则导出对应命名集合。
- `file.*`：把参考片段和角色描述注入模板上下文。
- `mcp_servers.file`：从 `mcps.json` 导入 MCP server，再和显式声明的 servers 一起注入模板上下文。
  `mcp_servers.<name>` 默认输出 provider-agnostic 的 MCP snippet：
  会按当前目标文件格式自动渲染；无法判断时回退 JSON。
  如果你要强制格式，可以在模板里写 `{{ mcp_servers.filesystem | toml }}`、`{{ mcp_servers.filesystem | json }}`、`{{ mcp_servers.filesystem | yaml }}`。
  如果目标文件需要 OpenAI Codex MCP 语法，则显式写 `{{ mcp_servers.filesystem | codex }}`。
  如果目标文件需要 Anthropic Claude 或 Gemini CLI 的 MCP 语法，则分别写 `{{ mcp_servers.filesystem | claude }}`、`{{ mcp_servers.filesystem | gemini }}`。
  如果需要承载 provider-only 选项，可以在通用 schema 下写 `[mcp_servers.<name>.options]`：
  例如 Codex 的 `startup_timeout_sec` / `enabled_tools`，Claude 的 `[mcp_servers.<name>.options.oauth]`，Gemini 的 `timeout` / `include_tools` / `auth_provider_type` / `oauth.scopes`。
  `codex` filter 会把通用 schema 适配成 Codex 官方字段：
  stdio server 输出 `command` / `args` / `env` / `env_vars` / `cwd`；
  HTTP server 输出 `url` / `bearer_token_env_var` / `http_headers` / `env_http_headers`，同时会投射 `startup_timeout_sec` / `tool_timeout_sec` / `enabled` / `required` / `enabled_tools` / `disabled_tools`。
  `claude` filter 会输出 Claude Code 兼容的 `type` / `command` / `args` / `env` / `url` / `headers` / `oauth` 片段；Claude 官方当前未声明 stdio `cwd` 字段，因此带 `cwd` 的 stdio server 会直接报错。
  `gemini` filter 会输出 Gemini CLI 兼容的 `command` / `args` / `env` / `cwd` / `url` / `type` / `headers` / `timeout` / `trust` / `description` / `includeTools` / `excludeTools` / `oauth` / `authProviderType` / `targetAudience` / `targetServiceAccount` 片段；若未显式声明 `trust`，会写出官方默认值 `trust = false`。
  也可以继续链式指定输出格式，例如 `{{ mcp_servers.filesystem | codex | toml }}`、`{{ mcp_servers.filesystem | claude | json }}`、`{{ mcp_servers.filesystem | gemini | yaml }}`。
- `kind = "dir" + template = true`：把整棵 `.agents` 目录作为一等渲染产物。
- `copy + symlink`：同一批渲染结果既可复制到目标，也可直接软链接到目标。

## 使用

在仓库根目录执行：

```bash
cargo run -p agentstow-cli -- --cwd e2e/data/real-example --profile base render --artifact workspace_agents --dry-run
cargo run -p agentstow-cli -- --cwd e2e/data/real-example --profile base render --artifact agents_dir --out /tmp/agentstow-real-example/.agents
cargo run -p agentstow-cli -- --cwd e2e/data/real-example mcp render --stdout
cargo run -p agentstow-cli -- --cwd e2e/data/real-example env emit --shell bash
cargo run -p agentstow-cli -- --cwd e2e/data/real-example env emit --set demo --shell bash
cargo run -p agentstow-cli -- --cwd e2e/data/real-example link
cargo run -p agentstow-cli -- --cwd e2e/data/real-example link status
```

## 预期结果

在某个临时 workspace 根目录执行 `link` 之后：

- `demo-targets/codex-lab/AGENTS.md`：复制出的已渲染文件。
- `demo-targets/codex-lab/.agents`：指向 render store 的目录软链接。
- `demo-targets/codex-lab/.env`：指向 render store 的文件软链接。
- `demo-targets-2/AGENTS.md`：指向 render store 的文件软链接。
- `demo-targets-2/.agents`：指向 render store 的目录软链接。
- `demo-targets-2/.env`：指向 render store 的文件软链接。

提交到 Git 的只有 fixture 源码和空目录占位，不包含运行后生成的 cache、copy 文件或 symlink。
