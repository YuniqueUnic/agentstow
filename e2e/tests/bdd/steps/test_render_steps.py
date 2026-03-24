from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, parsers, when

from ..support.fs import write_text


@given("a minimal render workspace", target_fixture="workspace")
def given_minimal_render_workspace(scenario_root: Path) -> Path:
    workspace = scenario_root / "workspace"
    write_text(
        workspace / "agentstow.toml",
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
    write_text(workspace / "artifacts" / "hello.txt.tera", "Hello {{ name }}!")
    return workspace


@when(
    parsers.parse('I run render dry-run for artifact "{artifact}" with profile "{profile}"'),
    target_fixture="result",
)
def when_run_render_dry_run(
    workspace: Path,
    run_cli,
    artifact: str,
    profile: str,
):
    return run_cli(
        "--cwd",
        str(workspace),
        "--profile",
        profile,
        "render",
        "--artifact",
        artifact,
        "--dry-run",
    )


@when(
    parsers.parse('I render artifact "{artifact}" to "{out_path}" with profile "{profile}"'),
    target_fixture="result",
)
def when_render_artifact_to_output(
    workspace: Path,
    run_cli,
    artifact: str,
    out_path: str,
    profile: str,
):
    return run_cli(
        "--cwd",
        str(workspace),
        "--profile",
        profile,
        "render",
        "--artifact",
        artifact,
        "--out",
        str(workspace / out_path),
    )


@when("I run mcp render to stdout", target_fixture="result")
def when_run_mcp_render_to_stdout(workspace: Path, run_cli):
    return run_cli("--cwd", str(workspace), "mcp", "render", "--stdout")
