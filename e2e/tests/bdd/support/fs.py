from __future__ import annotations

import json
import shutil
import tomllib
from pathlib import Path
from typing import Any


def copy_fixture_tree(src: Path, dst: Path) -> Path:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst, symlinks=False)
    return dst


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def write_json(path: Path, data: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False))


def read_json(path: Path) -> Any:
    return json.loads(path.read_text())


def read_toml(path: Path) -> Any:
    return tomllib.loads(path.read_text())


def parse_expected_scalar(value: str) -> Any:
    try:
        return json.loads(value)
    except json.JSONDecodeError:
        return value


def get_nested_value(data: Any, field_path: str) -> Any:
    current = data
    for segment in _parse_field_path(field_path):
        if isinstance(segment, int):
            current = current[segment]
        else:
            current = current[segment]
    return current


def list_relative_files(root: Path) -> set[str]:
    return {
        str(path.relative_to(root))
        for path in root.rglob("*")
        if path.is_file()
    }


def _parse_field_path(field_path: str) -> list[str | int]:
    segments: list[str | int] = []
    buf = ""
    idx = 0
    while idx < len(field_path):
        ch = field_path[idx]
        if ch == ".":
            if buf:
                segments.append(buf)
                buf = ""
            idx += 1
            continue
        if ch == "[":
            if buf:
                segments.append(buf)
                buf = ""
            end = field_path.index("]", idx)
            segments.append(int(field_path[idx + 1 : end]))
            idx = end + 1
            continue
        buf += ch
        idx += 1
    if buf:
        segments.append(buf)
    return segments
