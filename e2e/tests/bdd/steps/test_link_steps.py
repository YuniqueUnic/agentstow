from __future__ import annotations

import json
from pathlib import Path

from pytest_bdd import given, parsers, then, when

from ..support.fs import write_text


def _link_status_entry(workspace: Path, run_cli, suffix: str) -> dict:
    result = run_cli("--cwd", str(workspace), "--json", "link", "status")
    assert result.returncode == 0, result.stderr
    payload = json.loads(result.stdout)
    item = next(
        (entry for entry in payload if entry["target_path"].endswith(suffix)),
        None,
    )
    assert item is not None, payload
    return item


@given("link has been applied to the workspace")
def given_link_applied(workspace: Path, run_cli) -> None:
    result = run_cli("--cwd", str(workspace), "link")
    assert result.returncode == 0, result.stderr


@given(parsers.parse('I write the exact text "{text}" to "{relative_path}"'))
def given_write_exact_text(workspace: Path, relative_path: str, text: str) -> None:
    write_text(workspace / relative_path, text)


@when("I run link plan as json", target_fixture="result")
def when_run_link_plan_as_json(workspace: Path, run_cli):
    return run_cli("--cwd", str(workspace), "--json", "link", "--plan")


@when("I run link apply", target_fixture="result")
def when_run_link_apply(workspace: Path, run_cli):
    return run_cli("--cwd", str(workspace), "link")


@when(parsers.parse('I run link with force for target "{target}"'), target_fixture="result")
def when_run_link_force_for_target(workspace: Path, run_cli, target: str):
    return run_cli(
        "--cwd",
        str(workspace),
        "link",
        "--force",
        "--target",
        target,
    )


@when("I run link repair with force", target_fixture="result")
def when_run_link_repair_with_force(workspace: Path, run_cli):
    return run_cli("--cwd", str(workspace), "link", "repair", "--force")


@when("I run link status as json", target_fixture="result")
def when_run_link_status_as_json(workspace: Path, run_cli):
    return run_cli("--cwd", str(workspace), "--json", "link", "status")


@then(parsers.parse('link status json marks target path suffix "{suffix}" as healthy'))
def then_link_status_marks_path_healthy(workspace: Path, run_cli, suffix: str) -> None:
    item = _link_status_entry(workspace, run_cli, suffix)
    assert item["ok"] is True, item


@then(parsers.parse('link status json marks target path suffix "{suffix}" as unhealthy'))
def then_link_status_marks_path_unhealthy(workspace: Path, run_cli, suffix: str) -> None:
    item = _link_status_entry(workspace, run_cli, suffix)
    assert item["ok"] is False, item


@then(
    parsers.parse(
        'link status json marks target path suffix "{suffix}" with method "{method}" and healthy "{value}"'
    )
)
def then_link_status_marks_path_method_and_health(
    workspace: Path,
    run_cli,
    suffix: str,
    method: str,
    value: str,
) -> None:
    item = _link_status_entry(workspace, run_cli, suffix)
    assert item["method"] == method, item
    assert item["ok"] is (value.lower() == "true"), item
