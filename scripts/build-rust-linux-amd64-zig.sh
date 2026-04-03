#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/linux-amd64"
TARGET_TRIPLE="x86_64-unknown-linux-gnu"

bash "${REPO_ROOT}/scripts/set-version.sh" --sync-from-file >/dev/null

if ! command -v zig >/dev/null 2>&1; then
  echo "Error: zig is required for non-Docker Linux amd64 Rust builds." >&2
  exit 1
fi

if ! command -v cargo-zigbuild >/dev/null 2>&1; then
  echo "Error: cargo-zigbuild is required for non-Docker Linux amd64 Rust builds." >&2
  exit 1
fi

export PATH="${HOME}/.cargo/bin:${PATH}"

mkdir -p "${OUTPUT_DIR}"

(
  cd "${RUST_DIR}"
  cargo zigbuild --release --target "${TARGET_TRIPLE}"
)

cp "${RUST_DIR}/target/${TARGET_TRIPLE}/release/grafana-util" "${OUTPUT_DIR}/grafana-util"
echo "Built Linux amd64 Rust binaries with zig:"
echo "  ${OUTPUT_DIR}/grafana-util"
