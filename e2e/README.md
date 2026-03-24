## AgentStow e2e

本目录用于放置 `pytest` / `pytest-bdd` 驱动的端到端与验收测试。

当前实现遵循 `docs/TEST-PRD.md` 的方向：

- 使用 `pytest-bdd`，而不是 `behave`
- 通过真实 `agentstow` CLI 做黑盒验证
- 每个场景都复制 fixture 到临时目录运行
- 每个场景都隔离 `AGENTSTOW_HOME` / `AGENTSTOW_CONFIG_DIR` / `AGENTSTOW_DATA_DIR` / `AGENTSTOW_CACHE_DIR`

### 当前覆盖

- `render --dry-run`
- provider-specific `codex / claude / gemini` 模板渲染
- `mcp render --stdout`
- `link --plan --json`
- `link` 的 copy / symlink / repair / force / status
- 目录 target 的 `copy`
- `workspace init --git-init`
- `workspace status --json` 的 clean / dirty git 状态
- `scripts run` 的全局 `--timeout`

### 二进制选择

测试优先使用仓库根目录的 `target/debug/agentstow`。

每轮 e2e session 都会先在仓库根目录执行一次：

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
  data/
    providers/
      codex/
      claude/
      gemini/
    real-example/
    targets/
  tests/
    bdd/
      features/
      steps/
      support/
```

### 结构说明

- `e2e/data/real-example/`：完整 workspace fixture，覆盖真实 render/link/status/repair 链路。
- `e2e/data/providers/`：按 provider 分类的原生模板 fixture，用来验证 `codex / claude / gemini` filter 和默认格式推断。
- `e2e/data/targets/`：专门覆盖 target 安装行为的 fixture，例如目录 copy。
- `e2e/tests/bdd/features/`：业务场景。
- `e2e/tests/bdd/steps/`：step definitions。
- `e2e/tests/bdd/support/`：CLI、fixture、git 等公共 helper。

### 说明

- 这些测试是黑盒验收测试，不替代 Rust crate 里的单元/集成测试。
- fixture 目录只保存源码基线，不保存 render cache、真实 symlink 或运行后的状态数据库。
