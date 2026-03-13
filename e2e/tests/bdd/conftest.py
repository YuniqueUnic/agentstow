from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest
from pytest_bdd import parsers, then


@pytest.fixture(scope="session")
def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


@pytest.fixture(scope="session")
def agentstow_bin(repo_root: Path) -> Path:
    candidate = repo_root / "target" / "debug" / "agentstow"
    if not candidate.exists():
        subprocess.run(
            ["cargo", "build", "-p", "agentstow-cli"],
            cwd=repo_root,
            check=True,
        )
    return candidate


@pytest.fixture()
def tmp_workspace(tmp_path: Path) -> Path:
    return tmp_path


@pytest.fixture()
def run_cli(agentstow_bin: Path):
    def _run(*args: str, cwd: Path | None = None, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(agentstow_bin), *args],
            cwd=cwd,
            env=env,
            check=False,
            text=True,
            capture_output=True,
        )

    return _run


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def write_json(path: Path, data: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False))


__all__ = ["write_json", "write_text"]


@then("the command succeeds")
def then_command_succeeds(result) -> None:
    assert result.returncode == 0, result.stderr


@then(parsers.parse("the command fails with exit code {code:d}"))
def then_command_fails(result, code: int) -> None:
    assert result.returncode == code


@then(parsers.parse('stdout contains "{text}"'))
def then_stdout_contains(result, text: str) -> None:
    assert text in result.stdout


@then(parsers.parse('stderr contains "{text}"'))
def then_stderr_contains(result, text: str) -> None:
    assert text in result.stderr
