#!/usr/bin/env sh
set -eu

usage() {
    cat <<'EOF'
usage:
  init.sh --component <components...>
  init.sh --install <crates...>

examples:
  ./scripts/justfile/init.sh --component rust-analyzer clippy rustfmt
  ./scripts/justfile/init.sh --install prek cargo-nextest
EOF
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        printf "error: missing required command: %s\n" "$1" >&2
        exit 1
    }
}

ensure_rustup_components() {
    installed="$(rustup component list --installed 2>/dev/null || true)"
    missing=""

    for component in "$@"; do
        printf '%s\n' "$installed" | grep -Eq "^${component}-" || missing="$missing $component"
    done

    missing="${missing# }"
    if [ -n "$missing" ]; then
        # shellcheck disable=SC2086
        rustup component add $missing
    fi
}

ensure_cargo_tool() {
    crate="$1"
    binary="$crate"

    command -v "$binary" >/dev/null 2>&1 && return 0

    if command -v cargo-binstall >/dev/null 2>&1; then
        cargo binstall "$crate" 2>/dev/null || cargo install --locked "$crate"
    else
        cargo install --locked "$crate"
    fi
}

if [ $# -eq 0 ]; then
    usage
    exit 2
fi

while [ $# -gt 0 ]; do
    case "$1" in
        -h | --help)
            usage
            exit 0
            ;;
        --component)
            shift
            if [ $# -eq 0 ] || [ "${1#--}" != "$1" ]; then
                printf "error: --component requires at least 1 value\n" >&2
                exit 2
            fi

            need_cmd rustup
            components=""
            while [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; do
                components="$components $1"
                shift
            done

            # shellcheck disable=SC2086
            ensure_rustup_components $components
            ;;
        --install)
            shift
            if [ $# -eq 0 ] || [ "${1#--}" != "$1" ]; then
                printf "error: --install requires at least 1 value\n" >&2
                exit 2
            fi

            need_cmd cargo
            crates=""
            while [ $# -gt 0 ] && [ "${1#--}" = "$1" ]; do
                crates="$crates $1"
                shift
            done

            for crate in $crates; do
                ensure_cargo_tool "$crate"
            done
            ;;
        *)
            printf "error: unknown argument: %s\n" "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
done
