from __future__ import annotations

import json
from pathlib import Path

from pytest_bdd import given, scenarios, then, when

from .conftest import write_text


scenarios("features/link_plan.feature")


@given("a minimal link workspace", target_fixture="link_workspace")
def _link_workspace(tmp_workspace: Path) -> Path:
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

[targets.out]
artifact = "hello"
profile = "base"
target_path = "proj/out.txt"
method = "copy"
""".strip(),
    )
    write_text(tmp_workspace / "artifacts" / "hello.txt.tera", "Hello {{ name }}!")
    (tmp_workspace / "proj").mkdir(parents=True, exist_ok=True)
    return tmp_workspace


@when("I run link plan as json", target_fixture="result")
def _run_link_plan(link_workspace: Path, run_cli):
    return run_cli("--cwd", str(link_workspace), "--json", "link", "--plan")


@then("stdout is a json array with 1 item")
def _stdout_is_json_array(result):
    payload = json.loads(result.stdout)
    assert isinstance(payload, list)
    assert len(payload) == 1

