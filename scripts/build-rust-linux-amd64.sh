#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/linux-amd64"
RUST_IMAGE="${RUST_IMAGE:-rust:1.89-bookworm}"
TARGET_TRIPLE="x86_64-unknown-linux-gnu"

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker is required for Linux amd64 Rust builds." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"

docker run --rm \
  --platform linux/amd64 \
  --user "$(id -u):$(id -g)" \
  -v "${REPO_ROOT}:/workspace" \
  -w /workspace/rust \
  "${RUST_IMAGE}" \
  bash -lc "
    set -euo pipefail
    export PATH=\"/usr/local/cargo/bin:\$PATH\"
    if command -v rustup >/dev/null 2>&1; then
      rustup target add ${TARGET_TRIPLE}
    fi
    cargo build --release --target ${TARGET_TRIPLE}
  "

cp "${RUST_DIR}/target/${TARGET_TRIPLE}/release/grafana-utils" "${OUTPUT_DIR}/grafana-utils"
echo "Built Linux amd64 Rust binaries:"
echo "  ${OUTPUT_DIR}/grafana-utils"
