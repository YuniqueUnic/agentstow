from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, scenarios, when

from .conftest import write_text


scenarios("features/render.feature")


@given("a minimal render workspace", target_fixture="render_workspace")
def _render_workspace(tmp_workspace: Path) -> Path:
    write_text(
        tmp_workspace / "agentstow.toml",
        """
[profiles.base]
vars = { name = "BDD" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
""".strip(),
    )
    write_text(tmp_workspace / "artifacts" / "hello.txt.tera", "Hello {{ name }}!")
    return tmp_workspace


@when('I run render dry-run for artifact "hello"', target_fixture="result")
def _run_render(render_workspace: Path, run_cli):
    return run_cli(
        "--cwd",
        str(render_workspace),
        "--profile",
        "base",
        "render",
        "--artifact",
        "hello",
        "--dry-run",
    )
