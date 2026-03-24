from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class StateDirs:
    root: Path
    home_dir: Path
    config_dir: Path
    data_dir: Path
    cache_dir: Path


def ensure_state_dirs(root: Path) -> StateDirs:
    home_dir = root / "home"
    config_dir = root / "config"
    data_dir = root / "data"
    cache_dir = root / "cache"
    for path in (home_dir, config_dir, data_dir, cache_dir):
        path.mkdir(parents=True, exist_ok=True)
    return StateDirs(
        root=root,
        home_dir=home_dir,
        config_dir=config_dir,
        data_dir=data_dir,
        cache_dir=cache_dir,
    )


def build_state_env(
    state_dirs: StateDirs,
    extra_env: dict[str, str] | None = None,
) -> dict[str, str]:
    env = os.environ.copy()
    env.update(
        {
            "AGENTSTOW_HOME": str(state_dirs.home_dir),
            "AGENTSTOW_CONFIG_DIR": str(state_dirs.config_dir),
            "AGENTSTOW_DATA_DIR": str(state_dirs.data_dir),
            "AGENTSTOW_CACHE_DIR": str(state_dirs.cache_dir),
        }
    )
    if extra_env:
        env.update(extra_env)
    return env


def run_agentstow(
    bin_path: Path,
    *args: str,
    cwd: Path | None,
    state_dirs: StateDirs,
    extra_env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(bin_path), *args],
        cwd=cwd,
        env=build_state_env(state_dirs, extra_env),
        check=False,
        text=True,
        capture_output=True,
    )
