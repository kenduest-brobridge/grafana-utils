#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/macos-arm64"
PACKAGE_SCRIPT="${REPO_ROOT}/scripts/package-rust-artifacts.sh"
PACKAGE_VERSION="$(sed -n 's/^version = \"\\([^\"]*\\)\"/\\1/p' "${RUST_DIR}/Cargo.toml" | head -n 1)"
PACKAGE_NAME="grafana-utils-rust-macos-arm64-v${PACKAGE_VERSION}"

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
cp "${RUST_DIR}/target/release/grafana-access-utils" "${OUTPUT_DIR}/grafana-access-utils"
"${PACKAGE_SCRIPT}" "${OUTPUT_DIR}" "${RUST_DIR}/target/release" "${PACKAGE_NAME}"
echo "Built macOS arm64 Rust binaries:"
echo "  ${OUTPUT_DIR}/grafana-util"
echo "  ${OUTPUT_DIR}/grafana-access-utils"
