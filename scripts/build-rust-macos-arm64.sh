#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
BUILD_BROWSER="${BUILD_BROWSER:-0}"
OUTPUT_DIR="${REPO_ROOT}/dist/macos-arm64"
RUST_RELEASE_RUSTFLAGS="${RUST_RELEASE_RUSTFLAGS:--C debuginfo=0}"
BUILD_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }${RUST_RELEASE_RUSTFLAGS}"
FEATURE_ARGS=()

if [[ "${BUILD_BROWSER}" != "0" ]]; then
  OUTPUT_DIR="${REPO_ROOT}/dist/macos-arm64-browser"
  FEATURE_ARGS=(--features browser)
fi

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
  RUSTFLAGS="${BUILD_RUSTFLAGS}" cargo build --release "${FEATURE_ARGS[@]}"
)

cp "${RUST_DIR}/target/release/grafana-util" "${OUTPUT_DIR}/grafana-util"
codesign --force --sign - "${OUTPUT_DIR}/grafana-util"
if [[ "${BUILD_BROWSER}" != "0" ]]; then
  echo "Built macOS arm64 browser-enabled Rust binaries:"
else
  echo "Built macOS arm64 Rust binaries:"
fi
echo "  ${OUTPUT_DIR}/grafana-util"
