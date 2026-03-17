#!/usr/bin/env bash

set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${PYTHON:-python3}"
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

run_optional_cmd() {
  local tool_name="$1"
  local label="$2"
  shift 2

  if ! command -v "$tool_name" >/dev/null 2>&1; then
    warn "skipping ${label}; ${tool_name} is not installed"
    return 0
  fi

  run_step "$label" "$@"
}

run_optional_python_module() {
  local module_name="$1"
  local label="$2"
  shift 2

  if ! "$PYTHON_BIN" -c "import ${module_name}" >/dev/null 2>&1; then
    warn "skipping ${label}; python module ${module_name} is not installed"
    return 0
  fi

  run_step "$label" "$PYTHON_BIN" -m "$module_name" "$@"
}

cd "$ROOT_DIR" || exit 1

PYTHON_SRC_DIR="${ROOT_DIR}/python/grafana_utils"
PYTHON_TEST_DIR="${ROOT_DIR}/python/tests"
PYTHON_WRAPPER_DIR="${ROOT_DIR}/python"

run_step "python bytecode compile check" \
  "$PYTHON_BIN" -m compileall -q "$PYTHON_SRC_DIR" "$PYTHON_TEST_DIR" "$PYTHON_WRAPPER_DIR"

run_step "python unittest suite" \
  env PYTHONPATH="$PYTHON_WRAPPER_DIR${PYTHONPATH:+:$PYTHONPATH}" \
    "$PYTHON_BIN" -m unittest discover -s "$PYTHON_TEST_DIR" -v

run_optional_python_module ruff "ruff lint" \
  check "$PYTHON_SRC_DIR" "$PYTHON_TEST_DIR"

run_optional_python_module mypy "mypy type check" \
  --disable-error-code import-not-found \
  --disable-error-code import-untyped \
  "$PYTHON_SRC_DIR"

run_optional_python_module black "black format check" \
  --check --target-version py39 "$PYTHON_SRC_DIR" "$PYTHON_TEST_DIR"

if [ "$STATUS" -ne 0 ]; then
  warn "python quality checks finished with failures"
fi

exit "$STATUS"
