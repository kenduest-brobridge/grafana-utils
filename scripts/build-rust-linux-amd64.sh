#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
OUTPUT_DIR="${REPO_ROOT}/dist/linux-amd64"
RUST_IMAGE="${RUST_IMAGE:-rust:1.89-bookworm}"
RUST_BUILD_CONTAINER_NAME="${RUST_BUILD_CONTAINER_NAME:-grafana-utils-rust-linux-amd64-build}"
TARGET_TRIPLE="x86_64-unknown-linux-gnu"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
RUST_RELEASE_RUSTFLAGS="${RUST_RELEASE_RUSTFLAGS:--C debuginfo=0}"
BUILD_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }${RUST_RELEASE_RUSTFLAGS}"
BUILD_BROWSER="${BUILD_BROWSER:-0}"
CARGO_FEATURE_ARGS=""

if [[ "${BUILD_BROWSER}" != "0" ]]; then
  OUTPUT_DIR="${REPO_ROOT}/dist/linux-amd64-browser"
  RUST_BUILD_CONTAINER_NAME="${RUST_BUILD_CONTAINER_NAME:-grafana-utils-rust-linux-amd64-browser-build}"
  CARGO_FEATURE_ARGS="--features browser"
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker is required for Linux amd64 Rust builds." >&2
  exit 1
fi

mkdir -p "${OUTPUT_DIR}"

docker rm -f "${RUST_BUILD_CONTAINER_NAME}" >/dev/null 2>&1 || true

docker run --rm \
  --platform linux/amd64 \
  --name "${RUST_BUILD_CONTAINER_NAME}" \
  --user "$(id -u):$(id -g)" \
  -e CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS}" \
  -e RUSTFLAGS="${BUILD_RUSTFLAGS}" \
  -v "${REPO_ROOT}:/workspace" \
  -w /workspace/rust \
  "${RUST_IMAGE}" \
  bash -lc "
    set -euo pipefail
    export PATH=\"/usr/local/cargo/bin:\$PATH\"
    if command -v rustup >/dev/null 2>&1; then
      rustup target add ${TARGET_TRIPLE}
    fi
    cargo build --release --jobs \"\${CARGO_BUILD_JOBS}\" --target ${TARGET_TRIPLE} ${CARGO_FEATURE_ARGS}
  "

cp "${RUST_DIR}/target/${TARGET_TRIPLE}/release/grafana-util" "${OUTPUT_DIR}/grafana-util"
if [[ "${BUILD_BROWSER}" != "0" ]]; then
  echo "Built Linux amd64 browser-enabled Rust binaries:"
else
  echo "Built Linux amd64 Rust binaries:"
fi
echo "  ${OUTPUT_DIR}/grafana-util"
