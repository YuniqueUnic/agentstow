from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, scenarios, when, parsers

from .conftest import write_text


scenarios("features/scripts_timeout.feature")


@given("a workspace with a sleepy script", target_fixture="scripts_workspace")
def _scripts_workspace(tmp_workspace: Path) -> Path:
    write_text(
        tmp_workspace / "agentstow.toml",
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
    return tmp_workspace


@when(parsers.parse('I run the sleepy script with global timeout "{timeout}"'), target_fixture="result")
def _run_sleepy_script(scripts_workspace: Path, run_cli, timeout: str):
    return run_cli(
        "--cwd",
        str(scripts_workspace),
        "--timeout",
        timeout,
        "scripts",
        "run",
        "--id",
        "sleepy",
    )

