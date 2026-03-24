from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from .support.cli import StateDirs, ensure_state_dirs, run_agentstow
from .steps import (  # noqa: F401
    test_common_steps,
    test_fixture_steps,
    test_link_steps,
    test_render_steps,
    test_scripts_steps,
    test_workspace_steps,
)


@pytest.fixture(scope="session")
def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


@pytest.fixture(scope="session")
def agentstow_bin(repo_root: Path) -> Path:
    candidate = repo_root / "target" / "debug" / "agentstow"
    subprocess.run(
        ["cargo", "build", "-p", "agentstow-cli", "--bin", "agentstow"],
        cwd=repo_root,
        check=True,
    )
    return candidate


@pytest.fixture()
def scenario_root(tmp_path: Path) -> Path:
    return tmp_path


@pytest.fixture()
def data_root(repo_root: Path) -> Path:
    return repo_root / "e2e" / "data"


@pytest.fixture()
def state_dirs(scenario_root: Path) -> StateDirs:
    return ensure_state_dirs(scenario_root / ".agentstow-state")


@pytest.fixture()
def run_cli(agentstow_bin: Path, state_dirs: StateDirs):
    def _run(
        *args: str,
        cwd: Path | None = None,
        extra_env: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        return run_agentstow(
            agentstow_bin,
            *args,
            cwd=cwd,
            state_dirs=state_dirs,
            extra_env=extra_env,
        )

    return _run
