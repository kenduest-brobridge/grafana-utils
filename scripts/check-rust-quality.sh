#!/usr/bin/env bash

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="${RUST_DIR:-${ROOT_DIR}/rust}"
CARGO_BIN="${CARGO:-cargo}"
STATUS=0

log() {
  printf '%s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

run_step() {
  local label="$1"
  shift

  log "==> ${label}"
  if ! "$@"; then
    STATUS=1
  fi
}

cargo_subcommand_available() {
  local subcommand="$1"
  "$CARGO_BIN" "$subcommand" --help >/dev/null 2>&1
}

cd "$RUST_DIR" || exit 1

run_step "cargo test" \
  "$CARGO_BIN" test --quiet

if cargo_subcommand_available fmt; then
  run_step "cargo fmt --check" \
    "$CARGO_BIN" fmt --check
else
  warn "skipping cargo fmt --check; rustfmt is not installed"
fi

if cargo_subcommand_available clippy; then
  run_step "cargo clippy --all-targets -- -D warnings" \
    "$CARGO_BIN" clippy --all-targets -- -D warnings
else
  warn "skipping cargo clippy; clippy is not installed"
fi

if [ "$STATUS" -ne 0 ]; then
  warn "rust quality checks finished with failures"
fi

exit "$STATUS"
