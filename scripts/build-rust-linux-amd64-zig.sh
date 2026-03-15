#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/linux-amd64"
TARGET_TRIPLE="x86_64-unknown-linux-gnu"
PACKAGE_SCRIPT="${REPO_ROOT}/scripts/package-rust-artifacts.sh"
PACKAGE_VERSION="$(sed -n 's/^version = \"\\([^\"]*\\)\"/\\1/p' "${RUST_DIR}/Cargo.toml" | head -n 1)"
PACKAGE_NAME="grafana-utils-rust-linux-amd64-v${PACKAGE_VERSION}"

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
cp "${RUST_DIR}/target/${TARGET_TRIPLE}/release/grafana-access-utils" "${OUTPUT_DIR}/grafana-access-utils"
"${PACKAGE_SCRIPT}" "${OUTPUT_DIR}" "${RUST_DIR}/target/${TARGET_TRIPLE}/release" "${PACKAGE_NAME}"
echo "Built Linux amd64 Rust binaries with zig:"
echo "  ${OUTPUT_DIR}/grafana-util"
echo "  ${OUTPUT_DIR}/grafana-access-utils"
