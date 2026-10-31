#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build RepoMan in release mode and install it as `repoman` in a directory on your PATH.

Usage:
  ./scripts/install.sh              # install to ~/.local/bin
  ./scripts/install.sh --prefix DIR # install elsewhere (e.g. /usr/local/bin)
  ./scripts/install.sh --symlink    # symlink instead of copying (live rebuilds)
  ./scripts/install.sh --no-build   # reinstall the existing release binary

Environment:
  PREFIX   install prefix (default: ~/.local/bin)
EOF
}

MODE="copy"
BUILD=1
PREFIX="${PREFIX:-$HOME/.local/bin}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --symlink) MODE="symlink" ;;
    --copy) MODE="copy" ;;
    --no-build) BUILD=0 ;;
    --prefix)
      shift
      [[ $# -gt 0 ]] || { printf 'error: --prefix needs a directory\n' >&2; exit 2; }
      PREFIX="$1"
      ;;
    --prefix=*) PREFIX="${1#*=}" ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

cd "$(dirname "$0")/.."

APP_NAME="repoman"
LEGACY_APP_NAME="git-repo-manager"
BIN_SRC="src-tauri/target/release/$APP_NAME"

command -v pnpm >/dev/null || { printf 'error: pnpm not found\n' >&2; exit 1; }
command -v cargo >/dev/null || { printf 'error: cargo not found\n' >&2; exit 1; }

if [[ ! -x node_modules/.bin/vite ]]; then
  printf '==> Installing frontend dependencies...\n'
  pnpm install
fi

if [[ $BUILD -eq 1 ]]; then
  printf '==> Building %s (release)...\n' "$APP_NAME"
  if ! pnpm exec tauri build --no-bundle; then
    printf '\n==> Tauri CLI build failed; retrying with a direct cargo build...\n'
    pnpm build
    cargo build \
      --manifest-path src-tauri/Cargo.toml \
      --release \
      --features tauri/custom-protocol
  fi
else
  printf '==> Skipping build, reusing existing release binary\n'
fi

[[ -x "$BIN_SRC" ]] || {
  printf 'error: %s not found - run without --no-build\n' "$BIN_SRC" >&2
  exit 1
}

mkdir -p "$PREFIX"
BIN_DST="$PREFIX/$APP_NAME"

if [[ "$MODE" == "symlink" ]]; then
  BIN_ABS="$(cd "$(dirname "$BIN_SRC")" && pwd)/$(basename "$BIN_SRC")"
  rm -f "$BIN_DST"
  ln -s "$BIN_ABS" "$BIN_DST"
  printf '==> Symlinked %s -> %s\n' "$BIN_DST" "$BIN_ABS"
else
  install -m 0755 "$BIN_SRC" "$BIN_DST.tmp"
  mv -f "$BIN_DST.tmp" "$BIN_DST"
  printf '==> Installed %s\n' "$BIN_DST"
fi

case ":$PATH:" in
  *":$PREFIX:"*) ;;
  *)
    printf '\nwarning: %s is not in your PATH. Add this to ~/.zshrc:\n' "$PREFIX"
    printf '  export PATH="%s:$PATH"\n' "$PREFIX"
    ;;
esac

# -- the command was renamed git-repo-manager -> repoman; drop the stale one
if [[ -e "$PREFIX/$LEGACY_APP_NAME" || -L "$PREFIX/$LEGACY_APP_NAME" ]]; then
  rm -f "$PREFIX/$LEGACY_APP_NAME"
  printf '==> Removed renamed command %s\n' "$PREFIX/$LEGACY_APP_NAME"
fi

APP_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -1)"
printf '\nRepoMan %s installed.\n' "${APP_VERSION:-unknown}"
printf '  Desktop: %s\n' "$BIN_DST"
printf '  Browser: %s --web --port 8787 --dist %s/dist\n' "$BIN_DST" "$PWD"
