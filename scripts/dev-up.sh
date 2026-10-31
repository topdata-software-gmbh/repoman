#!/usr/bin/env bash
#
# Start local development: Rust web API + Vite UI (mprocs)
# 10/2026 created
#

set -euo pipefail

usage() {
  cat <<EOF
Usage: ./scripts/dev-up.sh [--skip-kit-build]

Builds the linked design-system packages, then starts two mprocs panes:

  backend   cargo run --manifest-path src-tauri/Cargo.toml -- --web --port 8787
  ui        pnpm dev (Vite on port 5183, proxying /api to 8787)

Open http://localhost:5183 in a browser once both panes are running.

Options:
  -h, --help            Show this help and exit.
  --skip-kit-build      Do not rebuild design-tokens and ui-kit-web-vue3.
                        Requires their dist/ to already exist.

mprocs keys:
  q                     Quit and stop both processes.
  s                     Stop the selected process.
  r                     Restart the selected process.
  Tab / arrows          Switch panes.

Prerequisites: pnpm, cargo, mprocs, and installed dependencies in both
repoman and ../topdata-design-system.
EOF
}

SKIP_KIT_BUILD=false
for arg in "$@"; do
  case "$arg" in
    -h|--help)
      usage
      exit 0
      ;;
    --skip-kit-build)
      SKIP_KIT_BUILD=true
      ;;
    *)
      echo "ERROR: unknown option: $arg" >&2
      echo "Run './scripts/dev-up.sh --help' for usage." >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DESIGN_SYSTEM_ROOT=$(cd "$PROJECT_ROOT/../topdata-design-system" 2>/dev/null && pwd || true)
KIT_DIR="$DESIGN_SYSTEM_ROOT/packages/ui-kit-web-vue3"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "ERROR: '$1' is required but was not found on PATH." >&2
    if [ "$1" = "mprocs" ]; then
      echo "       Install it with: npm i -g mprocs" >&2
    fi
    exit 1
  fi
}

require_command pnpm
require_command cargo
require_command mprocs

if [ -z "$DESIGN_SYSTEM_ROOT" ] || [ ! -d "$KIT_DIR" ]; then
  echo "ERROR: ../topdata-design-system with packages/ui-kit-web-vue3 was not found." >&2
  echo "       repoman links the design system from a sibling checkout." >&2
  exit 1
fi

if [ ! -d "$PROJECT_ROOT/node_modules" ]; then
  echo "ERROR: repoman dependencies are missing." >&2
  echo "       Run: pnpm install" >&2
  exit 1
fi

if [ ! -d "$DESIGN_SYSTEM_ROOT/node_modules" ]; then
  echo "ERROR: design-system dependencies are missing." >&2
  echo "       Run: (cd \"$DESIGN_SYSTEM_ROOT\" && pnpm install)" >&2
  exit 1
fi

if [ "$SKIP_KIT_BUILD" = true ]; then
  if [ ! -f "$KIT_DIR/dist/index.js" ] || [ ! -f "$KIT_DIR/dist/topdata-ui-kit.css" ]; then
    echo "ERROR: --skip-kit-build was passed but the ui kit has not been built." >&2
    echo "       Run: (cd \"$DESIGN_SYSTEM_ROOT\" && pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 build)" >&2
    exit 1
  fi
else
  echo "---- Building design-system packages ----"
  (
    cd "$DESIGN_SYSTEM_ROOT"
    pnpm --filter @topdata-software-gmbh/design-tokens build
    pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 build
  )
  echo ""
fi

echo "---- Starting local development ----"
echo "UI:  http://localhost:5183"
echo "API: http://127.0.0.1:8787"
echo "Press q in mprocs to stop both processes."
echo ""

cd "$PROJECT_ROOT"
exec mprocs \
  --names "backend,ui" \
  "cargo run --manifest-path src-tauri/Cargo.toml -- --web --port 8787" \
  "pnpm dev"
