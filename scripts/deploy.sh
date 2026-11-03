#!/usr/bin/env bash
#
# Release RepoMan: bump the version, build the release binary, publish it to the
# topbox release store, and verify the anonymous read.
# 11/2026 created
#

set -euo pipefail

usage() {
  cat <<'EOF'
Release RepoMan: bump the version, build, and publish to the release store.

Usage:
  ./scripts/deploy.sh [VERSION] [options]

  VERSION                 Exact version to release (default: bump the patch).

Options:
  --version VERSION       Exact version to release (same as the positional form).
  --dry-run               Build and preview the publish plan; do not upload.
  --no-build              Reuse the existing src-tauri/target/release/repoman.
  --notes TEXT            Release notes (default: "RepoMan <version>").
  --publisher PATH        publish_package.py to call.
  --output-dir DIR        Keep the built release tree in DIR.
  --transport MODE        auto | mc | ssh-docker.
  --mc-alias NAME         MinIO client alias for the mc transport.
  --ssh-host HOST         SSH host for the ssh-docker transport.
  --ssh-identity PATH     SSH identity/public key for ssh-docker.
  --container NAME        MinIO container name (default: minio-www).
  --container-endpoint URL
                          MinIO URL inside the container.
  --no-verify             Skip the post-upload size/digest check.
  -h, --help              Show this help and exit.

Environment:
  TOPBOX_PUBLISHER        Path to publish_package.py. Defaults to
                          ~/devel/topbox/scripts/publish_package.py, then a
                          sibling ../topbox checkout.
  TOPBOX_PUBLISH_*        Store, transport, and SSH settings read by the
                          publisher (for example TOPBOX_PUBLISH_SSH_HOST).

Examples:
  # preview the patch release without changing versions or uploading
  ./scripts/deploy.sh --dry-run --ssh-host root@arm2.srv.topinfra.de

  # release 0.2.0 through the MinIO container
  ./scripts/deploy.sh 0.2.0 --ssh-host root@arm2.srv.topinfra.de \
      --ssh-identity ~/.ssh/id_ed25519.pub

  # bump the patch and publish through a configured mc alias
  ./scripts/deploy.sh --transport mc --mc-alias topdata

The script leaves the version bump in the working tree so you can review and
commit it (and tag the release) after a successful publish.
EOF
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit "${2:-1}"
}

VERSION=""
DRY_RUN=0
BUILD=1
NOTES=""
PUBLISHER="${TOPBOX_PUBLISHER:-}"
OUTPUT_DIR=""
TRANSPORT=""
MC_ALIAS=""
SSH_HOST=""
SSH_IDENTITY=""
CONTAINER=""
CONTAINER_ENDPOINT=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      shift
      [[ $# -gt 0 ]] || die "--version needs a value" 2
      VERSION="$1"
      ;;
    --version=*) VERSION="${1#*=}" ;;
    --dry-run) DRY_RUN=1 ;;
    --no-build) BUILD=0 ;;
    --notes)
      shift
      [[ $# -gt 0 ]] || die "--notes needs a value" 2
      NOTES="$1"
      ;;
    --notes=*) NOTES="${1#*=}" ;;
    --publisher)
      shift
      [[ $# -gt 0 ]] || die "--publisher needs a path" 2
      PUBLISHER="$1"
      ;;
    --publisher=*) PUBLISHER="${1#*=}" ;;
    --output-dir)
      shift
      [[ $# -gt 0 ]] || die "--output-dir needs a directory" 2
      OUTPUT_DIR="$1"
      ;;
    --output-dir=*) OUTPUT_DIR="${1#*=}" ;;
    --transport)
      shift
      [[ $# -gt 0 ]] || die "--transport needs a mode" 2
      TRANSPORT="$1"
      ;;
    --transport=*) TRANSPORT="${1#*=}" ;;
    --mc-alias)
      shift
      [[ $# -gt 0 ]] || die "--mc-alias needs a name" 2
      MC_ALIAS="$1"
      ;;
    --mc-alias=*) MC_ALIAS="${1#*=}" ;;
    --ssh-host)
      shift
      [[ $# -gt 0 ]] || die "--ssh-host needs a host" 2
      SSH_HOST="$1"
      ;;
    --ssh-host=*) SSH_HOST="${1#*=}" ;;
    --ssh-identity)
      shift
      [[ $# -gt 0 ]] || die "--ssh-identity needs a path" 2
      SSH_IDENTITY="$1"
      ;;
    --ssh-identity=*) SSH_IDENTITY="${1#*=}" ;;
    --container)
      shift
      [[ $# -gt 0 ]] || die "--container needs a name" 2
      CONTAINER="$1"
      ;;
    --container=*) CONTAINER="${1#*=}" ;;
    --container-endpoint)
      shift
      [[ $# -gt 0 ]] || die "--container-endpoint needs a URL" 2
      CONTAINER_ENDPOINT="$1"
      ;;
    --container-endpoint=*) CONTAINER_ENDPOINT="${1#*=}" ;;
    --no-verify) EXTRA_ARGS+=(--no-verify) ;;
    -h|--help) usage; exit 0 ;;
    --) shift; break ;;
    -*) die "unknown option: $1 (try --help)" 2 ;;
    *)
      [[ -z $VERSION ]] || die "unexpected argument: $1 (try --help)" 2
      VERSION="$1"
      ;;
  esac
  shift
done

[[ $# -eq 0 ]] || die "unexpected argument: $1 (try --help)" 2

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

package_version() {
  sed -n 's/^  "version": "\([^"]*\)",$/\1/p' package.json | head -1
}

tauri_version() {
  sed -n 's/^  "version": "\([^"]*\)",$/\1/p' src-tauri/tauri.conf.json | head -1
}

cargo_version() {
  sed -n 's/^version = "\([^"]*\)"$/\1/p' src-tauri/Cargo.toml | head -1
}

set_version() {
  local version=$1
  sed -i "0,/^  \"version\": \".*\"/s//  \"version\": \"$version\"/" package.json
  sed -i "0,/^  \"version\": \".*\"/s//  \"version\": \"$version\"/" src-tauri/tauri.conf.json
  sed -i "0,/^version = \".*\"/s//version = \"$version\"/" src-tauri/Cargo.toml
}

verify_version() {
  local expected=$1
  [[ "$(package_version)" == "$expected" ]] \
    || die "package.json is $(package_version), expected $expected"
  [[ "$(tauri_version)" == "$expected" ]] \
    || die "tauri.conf.json is $(tauri_version), expected $expected"
  [[ "$(cargo_version)" == "$expected" ]] \
    || die "Cargo.toml is $(cargo_version), expected $expected"
}

bump_patch() {
  local current=$1 major minor patch
  [[ $current =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] \
    || die "cannot patch-bump non-numeric version '$current'; pass VERSION explicitly" 2
  major=${BASH_REMATCH[1]}
  minor=${BASH_REMATCH[2]}
  patch=${BASH_REMATCH[3]}
  printf '%s.%s.%s\n' "$major" "$minor" "$((patch + 1))"
}

CURRENT_VERSION="$(package_version)"
[[ -n $CURRENT_VERSION ]] || die "could not read the version from package.json"
[[ "$(tauri_version)" == "$CURRENT_VERSION" ]] \
  || die "package.json ($CURRENT_VERSION) and tauri.conf.json ($(tauri_version)) disagree"
[[ "$(cargo_version)" == "$CURRENT_VERSION" ]] \
  || die "package.json ($CURRENT_VERSION) and Cargo.toml ($(cargo_version)) disagree"

if [[ -z $VERSION ]]; then
  if [[ $DRY_RUN -eq 1 ]]; then
    TARGET_VERSION="$CURRENT_VERSION"
  else
    TARGET_VERSION="$(bump_patch "$CURRENT_VERSION")"
  fi
else
  TARGET_VERSION="${VERSION#v}"
  [[ $TARGET_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]] \
    || die "invalid version '$VERSION' (expected MAJOR.MINOR.PATCH)" 2
fi

CHANGED=0
SUCCESS=0

restore_version() {
  if [[ $SUCCESS -eq 0 && $CHANGED -eq 1 ]]; then
    set_version "$CURRENT_VERSION"
    printf 'Restored version %s after the failed release.\n' "$CURRENT_VERSION" >&2
  fi
}
trap restore_version EXIT

if [[ $DRY_RUN -eq 0 && $TARGET_VERSION != "$CURRENT_VERSION" ]]; then
  set_version "$TARGET_VERSION"
  verify_version "$TARGET_VERSION"
  CHANGED=1
  printf '==> Version %s -> %s\n' "$CURRENT_VERSION" "$TARGET_VERSION"
else
  printf '==> Version %s\n' "$TARGET_VERSION"
fi

if [[ -n $(git status --porcelain) && $CHANGED -eq 0 ]]; then
  printf 'warning: the working tree has uncommitted changes; the artifact will include them.\n' >&2
fi

if [[ -z $PUBLISHER ]]; then
  for candidate in \
    "$HOME/devel/topbox/scripts/publish_package.py" \
    "$ROOT/../topbox/scripts/publish_package.py"; do
    if [[ -f $candidate ]]; then
      PUBLISHER="$candidate"
      break
    fi
  done
fi

[[ -n $PUBLISHER && -f $PUBLISHER ]] \
  || die "publisher not found; pass --publisher PATH or set TOPBOX_PUBLISHER"
command -v uv >/dev/null 2>&1 || die "uv not found; install uv to run the publisher"

BIN_SRC="src-tauri/target/release/repoman"

if [[ $BUILD -eq 1 ]]; then
  command -v pnpm >/dev/null 2>&1 || die "pnpm not found"
  command -v cargo >/dev/null 2>&1 || die "cargo not found"

  if [[ ! -x node_modules/.bin/vite ]]; then
    printf '==> Installing frontend dependencies...\n'
    pnpm install
  fi

  printf '==> Building RepoMan %s (release)...\n' "$TARGET_VERSION"
  if ! pnpm exec tauri build --no-bundle; then
    printf '\n==> Tauri CLI build failed; retrying with a direct cargo build...\n' >&2
    pnpm build
    cargo build \
      --manifest-path src-tauri/Cargo.toml \
      --release \
      --features tauri/custom-protocol
  fi
else
  printf '==> Reusing existing release binary\n'
fi

[[ -x $BIN_SRC ]] || die "$BIN_SRC not found; run without --no-build"

PUBLISH_ARGS=(
  --app-id repoman
  --name RepoMan
  --version "$TARGET_VERSION"
  --binary "$BIN_SRC"
  --description "Multi-repository Git GUI"
  --notes "${NOTES:-RepoMan $TARGET_VERSION}"
)

[[ -n $OUTPUT_DIR ]] && PUBLISH_ARGS+=(--output-dir "$OUTPUT_DIR")
[[ -n $TRANSPORT ]] && PUBLISH_ARGS+=(--transport "$TRANSPORT")
[[ -n $MC_ALIAS ]] && PUBLISH_ARGS+=(--mc-alias "$MC_ALIAS")
[[ -n $SSH_HOST ]] && PUBLISH_ARGS+=(--ssh-host "$SSH_HOST")
[[ -n $SSH_IDENTITY ]] && PUBLISH_ARGS+=(--ssh-identity "$SSH_IDENTITY")
[[ -n $CONTAINER ]] && PUBLISH_ARGS+=(--container "$CONTAINER")
[[ -n $CONTAINER_ENDPOINT ]] && PUBLISH_ARGS+=(--container-endpoint "$CONTAINER_ENDPOINT")
[[ $DRY_RUN -eq 1 ]] && PUBLISH_ARGS+=(--dry-run)

printf '==> Publishing RepoMan %s\n' "$TARGET_VERSION"
uv run --script "$PUBLISHER" "${PUBLISH_ARGS[@]}" "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"

SUCCESS=1

if [[ $DRY_RUN -eq 1 ]]; then
  printf '\nDry run complete for RepoMan %s; nothing was uploaded.\n' "$TARGET_VERSION"
  exit 0
fi

printf '\nRepoMan %s released.\n' "$TARGET_VERSION"
if [[ $CHANGED -eq 1 ]]; then
  printf '  Next: review the version bump, then commit and tag it:\n'
  printf '    git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml\n'
  printf '    git commit -m "chore(release): repoman %s"\n' "$TARGET_VERSION"
  printf '    git tag v%s\n' "$TARGET_VERSION"
fi
