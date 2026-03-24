from __future__ import annotations

from pathlib import Path

from pytest_bdd import given, parsers

from ..support.fs import copy_fixture_tree


@given(parsers.parse('the workspace fixture "{fixture_name}"'), target_fixture="workspace")
def given_workspace_fixture(
    fixture_name: str,
    data_root: Path,
    scenario_root: Path,
) -> Path:
    return copy_fixture_tree(data_root / fixture_name, scenario_root / "workspace")


@given(parsers.parse('the provider workspace "{provider}"'), target_fixture="workspace")
def given_provider_workspace(
    provider: str,
    data_root: Path,
    scenario_root: Path,
) -> Path:
    return copy_fixture_tree(
        data_root / "providers" / provider,
        scenario_root / "workspace",
    )
