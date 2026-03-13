# Configure PowerShell for Windows

set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# show the recipe list
default:
    @just --list

# install all needed tools (Unix: bash, macOS, Linux)
[unix]
init:
    ./scripts/justfile/init.sh --component rust-analyzer clippy rustfmt
    ./scripts/justfile/init.sh --install prek cargo-nextest
    @echo "[0] init successfully"

# install all needed tools (Windows: PowerShell)
[windows]
init:
    pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/justfile/init.ps1 --component rust-analyzer clippy rustfmt
    pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/justfile/init.ps1 --install prek cargo-nextest
    @echo "[0] init successfully"

# install prek (which is the alternative tool of pre-commit)
install-prek:
    prek uninstall
    prek install .

# e2e env bootstrap (uv)
[unix]
init-e2e:
    cd "e2e" && (uv venv .venv --allow-existing 2>/dev/null || uv venv .venv) && uv sync

# e2e env bootstrap (uv)
[windows]
init-e2e:
    Set-Location "e2e"; try { uv venv .venv --allow-existing } catch { uv venv .venv }; uv sync

# formatting / lint / build
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

check *ARGS="--workspace --all-features":
    cargo check {{ ARGS }}

clippy *ARGS="--workspace --all-targets --all-features -- -D warnings":
    cargo clippy {{ ARGS }}

# test related things

# if nextest exists, use nextest instead of cargo test
[unix]
test *ARGS="":
    #!/usr/bin/env sh
    set -eu
    if command -v cargo-nextest >/dev/null 2>&1; then
        cargo nextest run --workspace --all-features {{ ARGS }}
    else
        cargo test --workspace --all-features {{ ARGS }}
    fi

[windows]
test *ARGS="":
    if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) { cargo nextest run --workspace --all-features {{ ARGS }} } else { cargo test --workspace --all-features {{ ARGS }} }

# e2e tests (pytest)
[unix]
e2e *ARGS="":
    cd "e2e" && uv run -- pytest -v --tb=short {{ ARGS }}
    # TODO: behave should be included

# e2e tests (pytest)
[windows]
e2e *ARGS="":
    Set-Location "e2e"; uv run -- pytest -v --tb=short {{ ARGS }}
    # TODO: behave should be included

# build / run
build *ARGS="--workspace --all-features":
    cargo build {{ ARGS }}

build-release *ARGS="--workspace --all-features":
    cargo build --release {{ ARGS }}

run *ARGS="workspace status":
    cargo run -p agentstow-cli -- {{ ARGS }}

[unix]
serve *ARGS="":
    #!/usr/bin/env bash
    set -euo pipefail
    set -- {{ ARGS }}
    addr="${AGENTSTOW_ADDR:-127.0.0.1:8787}"
    if [ "$#" -gt 0 ] && [[ "$1" != -* ]]; then
        addr="$1"
        shift
    fi
    cargo run -p agentstow-cli -- "$@" serve --addr "$addr"

[windows]
serve *ARGS="":
    pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/justfile/serve.ps1 {{ ARGS }}

web-install:
    cd web && bun install

web-dev *ARGS="":
    cd web && bun run dev -- {{ ARGS }}

web-build:
    cd web && bun run build

web-preview *ARGS="":
    cd web && bun run preview -- {{ ARGS }}

web-check:
    cd web && bun run typecheck

[unix]
dev *ARGS="":
    just web-build
    just serve {{ ARGS }}

[windows]
dev *ARGS="":
    just web-build
    just serve {{ ARGS }}

# run prek
prek *ARGS="-a":
    prek run {{ ARGS }}

# run clippy and rustfmt, then run prek
happy:
    cargo clippy --fix --allow-dirty --tests --workspace --all-targets --all-features -- -D warnings
    cargo fmt --all
    just prek

qa:
    just fmt-check
    just check
    just test
    just clippy
    just web-check

ci:
    just qa
    just web-build

alias pre-commit := prek
alias lint := happy
alias b := build
alias t := test
alias t2 := e2e
