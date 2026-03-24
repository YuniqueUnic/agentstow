from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, parsers, when

from ..support.fs import write_text


@given("a workspace with a sleepy script", target_fixture="workspace")
def given_workspace_with_sleepy_script(scenario_root: Path) -> Path:
    workspace = scenario_root / "workspace"
    write_text(
        workspace / "agentstow.toml",
        """
[scripts.sleepy]
kind = "shell"
entry = "bash"
args = ["-lc", "sleep 1"]
cwd_policy = "current"
stdin_mode = "none"
stdout_mode = "capture"
stderr_mode = "capture"
timeout_ms = 5000
expected_exit_codes = [0]
""".strip(),
    )
    return workspace


@when(
    parsers.parse('I run the sleepy script with global timeout "{timeout}"'),
    target_fixture="result",
)
def when_run_sleepy_script(workspace: Path, run_cli, timeout: str):
    return run_cli(
        "--cwd",
        str(workspace),
        "--timeout",
        timeout,
        "scripts",
        "run",
        "--id",
        "sleepy",
    )
