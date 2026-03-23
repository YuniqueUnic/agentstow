# real-example

这个目录是 `docs/example` 的可执行替代版本，直接对齐当前 `agentstow` 的真实能力：

- `[env.files] + [env]`：先从一个或多个 `.env` 文件加载变量，再由 `[env]` 里的直接声明覆盖同名值，统一注入模板上下文里的 `env.*`。
- `file.*`：把参考片段和角色描述注入模板上下文。
- `mcp_servers.file`：从 `mcps.json` 导入 MCP server，再和显式声明的 servers 一起注入模板上下文。
  `mcp_servers.<name>` 会按当前目标文件格式自动渲染；无法判断时回退 JSON。
  如果你要强制格式，可以在模板里写 `{{ mcp_servers.filesystem | toml }}`、`{{ mcp_servers.filesystem | json }}`、`{{ mcp_servers.filesystem | yaml }}`。
- `kind = "dir" + template = true`：把整棵 `.agents` 目录作为一等渲染产物。
- `copy + symlink`：同一批渲染结果既可复制到目标，也可直接软链接到目标。

## 使用

在仓库根目录执行：

```bash
cargo run -p agentstow-cli -- --cwd real-example --profile base render --artifact workspace_agents --dry-run
cargo run -p agentstow-cli -- --cwd real-example --profile base render --artifact agents_dir --out demo-render/.agents
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
