from __future__ import annotations

import subprocess
from pathlib import Path


def init_git_repo(repo_root: Path) -> None:
    subprocess.run(
        ["git", "init", "-b", "main"],
        cwd=repo_root,
        check=True,
        text=True,
        capture_output=True,
    )


def commit_all(repo_root: Path, message: str = "init") -> None:
    subprocess.run(
        ["git", "add", "."],
        cwd=repo_root,
        check=True,
        text=True,
        capture_output=True,
    )
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=AgentStow",
            "-c",
            "user.email=agentstow@example.com",
            "commit",
            "-m",
            message,
        ],
        cwd=repo_root,
        check=True,
        text=True,
        capture_output=True,
    )
