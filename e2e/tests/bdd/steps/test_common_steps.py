from __future__ import annotations

import json
from pathlib import Path

import pytest
from pytest_bdd import parsers, then

from ..support.fs import (
    get_nested_value,
    list_relative_files,
    parse_expected_scalar,
    read_json,
    read_toml,
)


def _path_root(request: pytest.FixtureRequest) -> Path:
    for fixture_name in ("workspace", "working_root", "scenario_root"):
        if fixture_name in request.fixturenames:
            return request.getfixturevalue(fixture_name)
    raise AssertionError("未找到可用于路径断言的根目录 fixture")


def _expected_text(text: str) -> str:
    return text.replace('\\"', '"').replace("\\n", "\n")


@then("the command succeeds")
def then_command_succeeds(result) -> None:
    assert result.returncode == 0, result.stderr


@then(parsers.parse("the command fails with exit code {code:d}"))
def then_command_fails(result, code: int) -> None:
    assert result.returncode == code, result.stderr


@then(parsers.parse('stdout contains "{text}"'))
def then_stdout_contains(result, text: str) -> None:
    assert _expected_text(text) in result.stdout


@then(parsers.parse('stderr contains "{text}"'))
def then_stderr_contains(result, text: str) -> None:
    assert text in result.stderr


@then(parsers.parse("stdout json has {count:d} items"))
def then_stdout_json_has_items(result, count: int) -> None:
    payload = json.loads(result.stdout)
    assert isinstance(payload, list)
    assert len(payload) == count


@then(parsers.parse('stdout json field "{field}" equals "{value}"'))
def then_stdout_json_field_equals(result, field: str, value: str) -> None:
    payload = json.loads(result.stdout)
    assert payload[field] == value


@then(parsers.parse('stdout json boolean field "{field}" is {value}'))
def then_stdout_json_bool_field_equals(result, field: str, value: str) -> None:
    payload = json.loads(result.stdout)
    expected = value.lower() == "true"
    assert payload[field] is expected


@then(parsers.parse('stdout json nested field "{field}" equals "{value}"'))
def then_stdout_json_nested_field_equals(result, field: str, value: str) -> None:
    payload = json.loads(result.stdout)
    assert get_nested_value(payload, field) == parse_expected_scalar(value)


@then(parsers.parse('the path "{relative_path}" exists'))
def then_path_exists(request: pytest.FixtureRequest, relative_path: str) -> None:
    assert (_path_root(request) / relative_path).exists()


@then(parsers.parse('the path "{relative_path}" is a symlink'))
def then_path_is_symlink(request: pytest.FixtureRequest, relative_path: str) -> None:
    assert (_path_root(request) / relative_path).is_symlink()


@then(parsers.parse('the file "{relative_path}" contains "{text}"'))
def then_file_contains(
    request: pytest.FixtureRequest,
    relative_path: str,
    text: str,
) -> None:
    body = (_path_root(request) / relative_path).read_text()
    assert _expected_text(text) in body


@then(parsers.parse('the file "{relative_path}" contains exactly "{text}"'))
def then_file_contains_exactly(
    request: pytest.FixtureRequest,
    relative_path: str,
    text: str,
) -> None:
    assert (_path_root(request) / relative_path).read_text() == _expected_text(text)


@then(parsers.parse('the json file "{relative_path}" has field "{field}" equal to "{value}"'))
def then_json_file_field_equals(
    request: pytest.FixtureRequest,
    relative_path: str,
    field: str,
    value: str,
) -> None:
    payload = read_json(_path_root(request) / relative_path)
    assert get_nested_value(payload, field) == parse_expected_scalar(value)


@then(parsers.parse('the json file "{relative_path}" array field "{field}" contains "{value}"'))
def then_json_file_array_contains(
    request: pytest.FixtureRequest,
    relative_path: str,
    field: str,
    value: str,
) -> None:
    payload = read_json(_path_root(request) / relative_path)
    actual = get_nested_value(payload, field)
    assert isinstance(actual, list)
    assert parse_expected_scalar(value) in actual


@then(parsers.parse('the toml file "{relative_path}" has field "{field}" equal to "{value}"'))
def then_toml_file_field_equals(
    request: pytest.FixtureRequest,
    relative_path: str,
    field: str,
    value: str,
) -> None:
    payload = read_toml(_path_root(request) / relative_path)
    assert get_nested_value(payload, field) == parse_expected_scalar(value)


@then(parsers.parse('the toml file "{relative_path}" array field "{field}" contains "{value}"'))
def then_toml_file_array_contains(
    request: pytest.FixtureRequest,
    relative_path: str,
    field: str,
    value: str,
) -> None:
    payload = read_toml(_path_root(request) / relative_path)
    actual = get_nested_value(payload, field)
    assert isinstance(actual, list)
    assert parse_expected_scalar(value) in actual


@then(parsers.parse('the directory "{relative_path}" contains file "{child_path}"'))
def then_directory_contains_file(
    request: pytest.FixtureRequest,
    relative_path: str,
    child_path: str,
) -> None:
    root = _path_root(request) / relative_path
    assert child_path in list_relative_files(root)


@then("the state database exists")
def then_state_database_exists(state_dirs) -> None:
    assert (state_dirs.data_dir / "agentstow.db").exists()
