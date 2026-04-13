#!/bin/sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
BINARY_NAME="${BINARY_NAME:-grafana-util}"
COMPLETION_SHELL="${COMPLETION_SHELL:-zsh}"
VERSION="${VERSION:-vlocal}"
KEEP_TMP="${KEEP_TMP:-0}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'test-local-installer.sh: %s\n' "$*" >&2
  exit 1
}

require_tool() {
  tool_name="$1"
  command -v "$tool_name" >/dev/null 2>&1 || fail "${tool_name} is required"
}

print_help() {
  cat <<'EOF'
Usage:
  scripts/test-local-installer.sh

Environment overrides:
  COMPLETION_SHELL=zsh  Shell completion to test through INSTALL_COMPLETION=auto.
                        Supported values: bash, zsh.
  BINARY_PATH=/path     Use one existing grafana-util binary instead of building.
  VERSION=vlocal        Local test release tag recorded in the installer flow.
  KEEP_TMP=1            Keep the temporary test home, bin dir, archive, and logs.

What it does:
  1. Builds or reuses a local grafana-util binary.
  2. Packs it into a local tar.gz release-style archive.
  3. Runs scripts/install.sh with ASSET_URL=file://... and INSTALL_COMPLETION=auto.
  4. Verifies the installed binary and generated completion file.
EOF
}

case "${1:-}" in
  -h|--help|help)
    print_help
    exit 0
    ;;
  "")
    ;;
  *)
    fail "unknown option: $1"
    ;;
esac

case "$COMPLETION_SHELL" in
  bash|zsh) ;;
  *) fail "unsupported COMPLETION_SHELL: ${COMPLETION_SHELL}; supported values: bash, zsh" ;;
esac

require_tool tar
require_tool mktemp
require_tool sh

tmp_base="${TMPDIR:-/tmp}"
tmpdir=$(mktemp -d "${tmp_base%/}/grafana-util-local-install.XXXXXX")
cleanup() {
  if [ "$KEEP_TMP" = "1" ]; then
    log "Kept temporary test directory: ${tmpdir}"
  else
    rm -rf "$tmpdir"
  fi
}
trap cleanup EXIT INT TERM

artifact_dir="${tmpdir}/artifact"
install_home="${tmpdir}/home"
install_bin="${tmpdir}/bin"
completion_dir="${tmpdir}/completion"
mkdir -p "$artifact_dir" "$install_home" "$install_bin" "$completion_dir"

if [ -n "${BINARY_PATH:-}" ]; then
  source_binary="$BINARY_PATH"
else
  require_tool cargo
  log "Building ${BINARY_NAME} locally..."
  cargo build --manifest-path "${ROOT_DIR}/rust/Cargo.toml" --bin "$BINARY_NAME"
  source_binary="${ROOT_DIR}/rust/target/debug/${BINARY_NAME}"
fi

[ -f "$source_binary" ] || fail "binary does not exist: ${source_binary}"
[ -x "$source_binary" ] || fail "binary is not executable: ${source_binary}"

cp "$source_binary" "${artifact_dir}/${BINARY_NAME}"
archive_path="${tmpdir}/grafana-utils-rust-local-${VERSION}.tar.gz"
tar -czf "$archive_path" -C "$artifact_dir" "$BINARY_NAME"

if [ -x "/bin/${COMPLETION_SHELL}" ]; then
  shell_path="/bin/${COMPLETION_SHELL}"
else
  shell_path=$(command -v "$COMPLETION_SHELL") || fail "${COMPLETION_SHELL} is not available"
fi

log "Running local installer smoke for ${COMPLETION_SHELL} completion..."
ASSET_URL="file://${archive_path}" \
  VERSION="$VERSION" \
  BIN_DIR="$install_bin" \
  HOME="$install_home" \
  SHELL="$shell_path" \
  INSTALL_COMPLETION=auto \
  COMPLETION_DIR="$completion_dir" \
  sh "${ROOT_DIR}/scripts/install.sh"

installed_binary="${install_bin}/${BINARY_NAME}"
[ -x "$installed_binary" ] || fail "installed binary is missing or not executable: ${installed_binary}"

case "$COMPLETION_SHELL" in
  bash) completion_path="${completion_dir}/${BINARY_NAME}" ;;
  zsh) completion_path="${completion_dir}/_${BINARY_NAME}" ;;
esac
[ -s "$completion_path" ] || fail "completion file is missing or empty: ${completion_path}"

"$installed_binary" --version >/dev/null || fail "installed binary --version failed"

log "Local installer smoke passed."
log "Installed binary: ${installed_binary}"
log "Completion file: ${completion_path}"
