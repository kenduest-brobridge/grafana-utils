#!/usr/bin/env bash

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATUS=0

run_gate() {
  local script_path="$1"

  if ! bash "$script_path"; then
    STATUS=1
  fi
}

run_gate "${ROOT_DIR}/scripts/check-python-quality.sh"
run_gate "${ROOT_DIR}/scripts/check-rust-quality.sh"

exit "$STATUS"
