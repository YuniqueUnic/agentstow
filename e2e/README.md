## AgentStow e2e

本目录用于放置 `pytest` / `pytest-bdd` 驱动的端到端与验收测试。

当前最小可运行方案遵循 `docs/TEST-PRD.md` 的方向：

- 使用 `pytest-bdd`，而不是 `behave`
- 通过真实 `agentstow` CLI 做黑盒验证
- 只使用本地临时目录与 `AGENTSTOW_HOME`，避免污染用户环境

### 当前覆盖

- `render --dry-run`
- `link --plan --json`
- `scripts run` 的全局 `--timeout`

### 二进制选择

测试优先使用仓库根目录的 `target/debug/agentstow`。

如果该二进制不存在，测试会自动在仓库根目录执行：

```bash
cargo build -p agentstow-cli --bin agentstow
```

这样比每个场景都用 `cargo run` 更稳定，也更快。

### 运行

使用 `uv`：

```bash
cd e2e
uv sync
uv run pytest
```

如果只跑 BDD 场景：

```bash
cd e2e
uv run pytest tests/bdd
```

### 目录结构

```text
e2e/
  tests/
    bdd/
      features/
      steps/
      support/
```

### 后续建议

下一步最值得补的是：

- `link status` / `repair`
- `env emit --stdout`
- MCP validate / render
- 容器化场景（后续结合 `testcontainers-python`）

