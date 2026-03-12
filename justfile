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

# test related things
# if nextest exists, use nextest instead of cargo test
[unix]
test *ARGS="--no-tests=pass":
    #!/usr/bin/env sh
    set -eu
    if command -v cargo-nextest >/dev/null 2>&1; then
        cargo nextest run --workspace --all-features {{ARGS}}
    else
        cargo test --workspace --all-features {{ARGS}}
    fi

[windows]
test:
    if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) { cargo nextest run --workspace --all-features } else { cargo test --workspace --all-features }


# e2e tests (pytest)
[unix]
e2e:
    cd "e2e" && uv run -- pytest -v --tb=short
    # TODO: behave should be included

# e2e tests (pytest)
[windows]
e2e:
    Set-Location "e2e"; uv run -- pytest -v --tb=short
    # TODO: behave should be included

# build --workspcae default
build *ARGS="--workspace":
    cargo build {{ARGS}} --all-features

# run prek
prek *ARGS="-a":
    prek run {{ARGS}}

# run clippy and rustfmt, then run prek
happy:
    cargo clippy --fix --allow-dirty --tests --workspace --all-targets --all-features -- -D warnings
    cargo fmt --all
    just prek

alias pre-commit := prek
alias lint := happy
alias b := build
alias t := test
alias t2 := e2e
