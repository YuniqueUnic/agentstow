# real-example

这个目录是 `docs/example` 的可执行替代版本，直接对齐当前 `agentstow` 的真实能力：

- `[env.files] + [env]`：先从一个或多个 `.env` 文件加载变量，再由 `[env]` 里的直接声明覆盖同名值，统一注入模板上下文里的 `env.*`。
- `[env.emit.<name>]`：定义命名的 shell 导出集合；`agentstow env emit` 不带 `--set` 时会直接导出 `[env.files] + [env]` 的合并结果，带 `--set demo` 时则导出对应命名集合。
- `file.*`：把参考片段和角色描述注入模板上下文。
- `mcp_servers.file`：从 `mcps.json` 导入 MCP server，再和显式声明的 servers 一起注入模板上下文。
  `mcp_servers.<name>` 默认输出 provider-agnostic 的 MCP snippet：
  会按当前目标文件格式自动渲染；无法判断时回退 JSON。
  如果你要强制格式，可以在模板里写 `{{ mcp_servers.filesystem | toml }}`、`{{ mcp_servers.filesystem | json }}`、`{{ mcp_servers.filesystem | yaml }}`。
  如果目标文件需要 OpenAI Codex MCP 语法，则显式写 `{{ mcp_servers.filesystem | codex }}`。
  如果目标文件需要 Anthropic Claude 或 Gemini CLI 的 MCP 语法，则分别写 `{{ mcp_servers.filesystem | claude }}`、`{{ mcp_servers.filesystem | gemini }}`。
  `codex` filter 会把通用 schema 适配成 Codex 官方字段：
  stdio server 输出 `command` / `args` / `env` / `env_vars` / `cwd`；
  HTTP server 输出 `url` / `bearer_token_env_var` / `http_headers` / `env_http_headers`。
  `claude` filter 会输出 Claude Code 兼容的 `type` / `command` / `args` / `env` / `url` / `headers` 片段；Claude 官方当前未声明 stdio `cwd` 字段，因此带 `cwd` 的 stdio server 会直接报错。
  `gemini` filter 会输出 Gemini CLI 兼容的 `command` / `args` / `env` / `cwd` / `url` / `type` / `headers` 片段，并显式写出官方默认值 `trust = false`。
  也可以继续链式指定输出格式，例如 `{{ mcp_servers.filesystem | codex | toml }}`、`{{ mcp_servers.filesystem | claude | json }}`、`{{ mcp_servers.filesystem | gemini | yaml }}`。
- `kind = "dir" + template = true`：把整棵 `.agents` 目录作为一等渲染产物。
- `copy + symlink`：同一批渲染结果既可复制到目标，也可直接软链接到目标。

## 使用

在仓库根目录执行：

```bash
cargo run -p agentstow-cli -- --cwd real-example --profile base render --artifact workspace_agents --dry-run
cargo run -p agentstow-cli -- --cwd real-example --profile base render --artifact agents_dir --out demo-render/.agents
cargo run -p agentstow-cli -- --cwd real-example env emit --shell bash
cargo run -p agentstow-cli -- --cwd real-example env emit --set demo --shell bash
cargo run -p agentstow-cli -- --cwd real-example link
cargo run -p agentstow-cli -- --cwd real-example link status
```

## 预期结果

执行 `link` 之后：

- `real-example/demo-targets/codex-lab/AGENTS.md`：复制出的已渲染文件。
- `real-example/demo-targets/codex-lab/.agents`：指向 render store 的目录软链接。
- `real-example/demo-targets/codex-lab/.env`：指向 render store 的文件软链接。
- `real-example/demo-targets-2/AGENTS.md`：指向 render store 的文件软链接。
- `real-example/demo-targets-2/.agents`：指向 render store 的目录软链接。
- `real-example/demo-targets-2/.env`：指向 render store 的文件软链接。

提交到 Git 的只有示例源码和空目录占位，不包含运行后生成的 cache symlink。
