## e2e data

`e2e/data/` 只放可复制到临时目录执行的 fixture 源数据，不放运行后生成物。

### 目录约定

- `providers/`
  - `codex/`：Codex 原生格式 fixture，验证 `| codex` filter、timeout/tool allowlist 等 provider-only 选项。
  - `claude/`：Claude Code 原生格式 fixture，验证 `| claude` filter 与 `options.oauth` 映射。
  - `gemini/`：Gemini CLI 原生格式 fixture，验证 `| gemini` filter、`httpUrl`、tool filtering 与 OAuth/auth provider 选项。
- `real-example/`
  - 完整 workspace fixture，覆盖 `env/file/mcp_servers/artifacts/targets` 的黑盒链路。
- `targets/`
  - 专门验证 target 安装行为的 fixture，例如目录 copy、目录/file symlink 等。

### 基线要求

- 只提交源码输入与必要空目录占位。
- 不提交 render cache、sqlite state、运行后生成的 copy 文件或主机相关 symlink。
- 新增 fixture 时，优先按“单一能力一个目录”组织，避免一个 fixture 同时承载过多不相关断言。
