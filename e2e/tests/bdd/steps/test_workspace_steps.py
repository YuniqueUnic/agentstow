from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, parsers, when

from ..support.fs import write_text
from ..support.git import commit_all, init_git_repo


@given("an empty temp directory as the working root", target_fixture="working_root")
def given_empty_working_root(scenario_root: Path) -> Path:
    working_root = scenario_root / "working-root"
    working_root.mkdir(parents=True, exist_ok=True)
    return working_root


@given("a clean git repository workspace", target_fixture="working_root")
def given_clean_git_repository_workspace(scenario_root: Path) -> Path:
    working_root = scenario_root / "git-repo"
    working_root.mkdir(parents=True, exist_ok=True)
    init_git_repo(working_root)
    write_text(working_root / "README.md", "demo\n")
    commit_all(working_root)
    return working_root


@given("a dirty git repository workspace", target_fixture="working_root")
def given_dirty_git_repository_workspace(scenario_root: Path) -> Path:
    working_root = scenario_root / "git-repo"
    working_root.mkdir(parents=True, exist_ok=True)
    init_git_repo(working_root)
    write_text(working_root / "README.md", "demo\n")
    commit_all(working_root)
    write_text(working_root / "README.md", "demo\nchanged\n")
    return working_root


@when(
    parsers.parse('I run workspace init with git into "{workspace_name}"'),
    target_fixture="result",
)
def when_run_workspace_init_with_git(
    working_root: Path,
    run_cli,
    workspace_name: str,
):
    return run_cli(
        "--cwd",
        str(working_root),
        "--workspace",
        str(working_root / workspace_name),
        "--json",
        "workspace",
        "init",
        "--git-init",
    )


@when("I run workspace status as json", target_fixture="result")
def when_run_workspace_status_as_json(working_root: Path, run_cli):
    return run_cli("--cwd", str(working_root), "--json", "workspace", "status")
