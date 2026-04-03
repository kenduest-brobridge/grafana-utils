#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/macos-arm64"

bash "${REPO_ROOT}/scripts/set-version.sh" --sync-from-file >/dev/null

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: build-rust-macos-arm64 must run on macOS." >&2
  exit 1
fi

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "Error: build-rust-macos-arm64 expects Apple Silicon (arm64)." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"

(
  cd "${RUST_DIR}"
  cargo build --release
)

cp "${RUST_DIR}/target/release/grafana-util" "${OUTPUT_DIR}/grafana-util"
echo "Built macOS arm64 Rust binaries:"
echo "  ${OUTPUT_DIR}/grafana-util"
