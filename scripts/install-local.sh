#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
BINARY_NAME="${BINARY_NAME:-grafana-util}"
VERSION="${VERSION:-vlocal}"
KEEP_TMP="${KEEP_TMP:-0}"
LOCAL_INSTALL_RELEASE="${LOCAL_INSTALL_RELEASE:-0}"
INSTALL_TMPDIR="${TMPDIR:-/tmp}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'install-local.sh: %s\n' "$*" >&2
  exit 1
}

require_tool() {
  tool_name="$1"
  command -v "$tool_name" >/dev/null 2>&1 || fail "${tool_name} is required"
}

print_help() {
  cat <<'EOF'
Usage:
  scripts/install-local.sh [--interactive]

Environment overrides:
  BINARY_PATH=/path         Use one existing grafana-util binary instead of building.
  BINARY_NAME=name          Override the binary name inside the local archive.
  VERSION=vlocal           Local test release tag recorded in the installer flow.
  LOCAL_INSTALL_RELEASE=1   Build the release binary instead of the debug binary.
  KEEP_TMP=1               Keep the temporary archive directory.

Pass-through installer overrides:
  BIN_DIR=/custom/bin
  INSTALL_COMPLETION=auto|bash|zsh
  COMPLETION_DIR=/custom

What it does:
  1. Builds or reuses one local grafana-util binary.
  2. Packs it into a release-style tar.gz archive.
  3. Runs scripts/install.sh with ASSET_URL=file://... against that archive.
EOF
}

case "${1:-}" in
  -h|--help|help)
    print_help
    exit 0
    ;;
  --interactive|"")
    ;;
  *)
    fail "unknown option: $1"
    ;;
esac

require_tool tar
require_tool mktemp
require_tool sh

tmpdir=$(mktemp -d "${INSTALL_TMPDIR%/}/grafana-util-local-install.XXXXXX")
cleanup() {
  if [ "$KEEP_TMP" = "1" ]; then
    log "Kept temporary local install archive directory: ${tmpdir}"
  else
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT INT TERM

artifact_dir="${tmpdir}/artifact"
mkdir -p "$artifact_dir"

if [ -n "${BINARY_PATH:-}" ]; then
  source_binary="$BINARY_PATH"
else
  require_tool cargo
  if [ "$LOCAL_INSTALL_RELEASE" = "1" ]; then
    log "Building local release ${BINARY_NAME}..."
    cargo build --release --manifest-path "${ROOT_DIR}/rust/Cargo.toml" --bin "$BINARY_NAME"
    source_binary="${ROOT_DIR}/rust/target/release/${BINARY_NAME}"
  else
    log "Building local debug ${BINARY_NAME}..."
    cargo build --manifest-path "${ROOT_DIR}/rust/Cargo.toml" --bin "$BINARY_NAME"
    source_binary="${ROOT_DIR}/rust/target/debug/${BINARY_NAME}"
  fi
fi

[ -f "$source_binary" ] || fail "binary does not exist: ${source_binary}"
[ -x "$source_binary" ] || fail "binary is not executable: ${source_binary}"

cp "$source_binary" "${artifact_dir}/${BINARY_NAME}"
archive_path="${tmpdir}/grafana-utils-rust-local-${VERSION}.tar.gz"
tar -czf "$archive_path" -C "$artifact_dir" "$BINARY_NAME"

log "Installing local ${BINARY_NAME} through scripts/install.sh..."
ASSET_URL="file://${archive_path}" \
  VERSION="$VERSION" \
  sh "${ROOT_DIR}/scripts/install.sh" "$@"
