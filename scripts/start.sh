#!/usr/bin/env bash
#
# Launch the installed RepoMan (`repoman`).
# 10/2026 created
#

set -euo pipefail

# -- new command name first, then the pre-rename `git-repo-manager` as a fallback
if BIN="$(command -v repoman 2>/dev/null)"; then
  :
elif [[ -n "${HOME:-}" && -x "${HOME}/.local/bin/repoman" ]]; then
  BIN="${HOME}/.local/bin/repoman"
elif BIN="$(command -v git-repo-manager 2>/dev/null)"; then
  :
elif [[ -n "${HOME:-}" && -x "${HOME}/.local/bin/git-repo-manager" ]]; then
  BIN="${HOME}/.local/bin/git-repo-manager"
else
  cat >&2 <<'EOF'
RepoMan is not installed.

  Install it:    ./scripts/install.sh
  Desktop dev:   pnpm tauri dev
  Browser dev:   ./scripts/dev-up.sh
EOF
  exit 1
fi

exec "$BIN" "$@"
