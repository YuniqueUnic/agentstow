## AgentStow

AgentStow 是一个 `Git-native` 的 `source-of-truth` 管理器，用于集中管理 AI
instructions、MCP 配置、环境变量定义与模板化文本 artifact，并通过
`render / validate / link / emit` 将它们一致地分发到多个项目与运行环境中。

本仓库的产品定义与测试策略见：

- `docs/PRD.md`
- `docs/TEST-PRD.md`

## 快速开始（最小可用）

### 1) 在某个 workspace 根目录创建 `agentstow.toml`

```toml
[profiles.base]
vars = { name = "AgentStow" }

[artifacts.agents]
kind = "file"
source = "artifacts/AGENTS.md.tera"
template = true
validate_as = "none"

[targets.my_project_agents]
artifact = "agents"
profile = "base"
target_path = "../my-project/AGENTS.md"
method = "copy"
```

并创建模板文件：

```text
artifacts/AGENTS.md.tera
```

内容示例：

```md
# {{ name }}
```

### 2) 渲染/校验/安装

```bash
# 开发期也可用：
# cargo run -p agentstow-cli -- <args...>

# 渲染预览（stdout）
agentstow --profile base render --artifact agents --dry-run

# 生成 link plan（机器可读）
agentstow --json link --plan

# 执行安装（render → validate → link）
agentstow link

# 查看 link 状态（从本地 sqlite link graph 读取）
agentstow link status
```

### 3) 本地 Web（Material3 风格最小 UI）

```bash
agentstow serve --addr 127.0.0.1:8787
```

打开 `http://127.0.0.1:8787`。

## 状态与测试隔离（强制）

为避免污染用户环境，支持以下环境变量（见 `docs/TEST-PRD.md` 6.4）：

- `AGENTSTOW_HOME`
- `AGENTSTOW_CONFIG_DIR`
- `AGENTSTOW_DATA_DIR`
- `AGENTSTOW_CACHE_DIR`

建议在测试/CI 中至少设置 `AGENTSTOW_HOME` 指向临时目录。

## 质量门禁

```bash
cargo fmt --all
cargo check --workspace --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
