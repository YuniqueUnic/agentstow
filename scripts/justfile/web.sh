#!/usr/bin/env sh
set -eu

usage() {
    cat <<'EOF'
usage:
  web.sh install

environment:
  AGENTSTOW_WEB_INSTALL=missing|always|never  default: missing
  AGENTSTOW_WEB_PM=auto|bun|npm               default: auto
EOF
}

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WEB_DIR="$REPO_ROOT/web"

mode=${AGENTSTOW_WEB_INSTALL:-missing}
pm=${AGENTSTOW_WEB_PM:-auto}

need_install() {
    [ ! -d "$WEB_DIR/node_modules" ]
}

run_bun_install() {
    command -v bun >/dev/null 2>&1 || return 127
    cd "$WEB_DIR"
    bun install --frozen-lockfile
}

run_npm_install() {
    command -v npm >/dev/null 2>&1 || return 127
    cd "$WEB_DIR"
    if [ -f package-lock.json ]; then
        npm ci
    else
        npm install
    fi
}

install() {
    case "$mode" in
        never)
            printf '[agentstow:web-install] skip install (AGENTSTOW_WEB_INSTALL=never)\n'
            return 0
            ;;
        missing)
            if ! need_install; then
                printf '[agentstow:web-install] reuse existing web/node_modules; skip network install\n'
                return 0
            fi
            ;;
        always)
            ;;
        *)
            printf 'error: unsupported AGENTSTOW_WEB_INSTALL=%s\n' "$mode" >&2
            return 1
            ;;
    esac

    case "$pm" in
        bun)
            run_bun_install
            ;;
        npm)
            run_npm_install
            ;;
        auto)
            if run_bun_install; then
                return 0
            fi
            printf '[agentstow:web-install] bun install failed; fallback to npm\n' >&2
            run_npm_install
            ;;
        *)
            printf 'error: unsupported AGENTSTOW_WEB_PM=%s\n' "$pm" >&2
            return 1
            ;;
    esac
}

cmd=${1:-}
case "$cmd" in
    install)
        install
        ;;
    ""|-h|--help|help)
        usage
        ;;
    *)
        printf 'error: unsupported command: %s\n' "$cmd" >&2
        usage >&2
        exit 1
        ;;
esac
