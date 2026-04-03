#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RUST_DIR="${REPO_ROOT}/rust"
TARGET_TRIPLE="x86_64-unknown-linux-gnu"
BINARY_NAME="grafana-util"

CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
RUST_RELEASE_RUSTFLAGS="${RUST_RELEASE_RUSTFLAGS:--C debuginfo=0}"
ZIG_BUILD_EXTRA_RUSTFLAGS="${ZIG_BUILD_EXTRA_RUSTFLAGS:--C codegen-units=1}"
BUILD_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }${RUST_RELEASE_RUSTFLAGS} ${ZIG_BUILD_EXTRA_RUSTFLAGS}"
VERBOSE="${VERBOSE:-0}"
BUILD_BROWSER="${BUILD_BROWSER:-0}"
ZIG_RELEASE_LTO="${ZIG_RELEASE_LTO:-off}"

require_tool() {
  local tool_name="$1"
  local reason="$2"

  if ! command -v "${tool_name}" >/dev/null 2>&1; then
    echo "Error: ${tool_name} is required for ${reason}." >&2
    exit 1
  fi
}

resolve_output_dir() {
  if [[ "${BUILD_BROWSER}" != "0" ]]; then
    printf '%s\n' "${REPO_ROOT}/dist/linux-amd64-browser"
    return 0
  fi

  printf '%s\n' "${REPO_ROOT}/dist/linux-amd64"
}

build_flavor_label() {
  if [[ "${BUILD_BROWSER}" != "0" ]]; then
    printf '%s\n' "browser-enabled"
    return 0
  fi

  printf '%s\n' "default"
}

run_zigbuild() {
  local cargo_args=(
    zigbuild
    --release
    --jobs "${CARGO_BUILD_JOBS}"
    --target "${TARGET_TRIPLE}"
  )

  if [[ "${VERBOSE}" != "0" ]]; then
    cargo_args=(-vv "${cargo_args[@]}")
  fi

  if [[ "${BUILD_BROWSER}" != "0" ]]; then
    cargo_args+=(--features browser)
  fi

  (
    cd "${RUST_DIR}"
    CARGO_PROFILE_RELEASE_LTO="${ZIG_RELEASE_LTO}" \
      RUSTFLAGS="${BUILD_RUSTFLAGS}" \
      cargo "${cargo_args[@]}"
  )
}

copy_artifact() {
  local output_dir="$1"
  local source_path="${RUST_DIR}/target/${TARGET_TRIPLE}/release/${BINARY_NAME}"
  local target_path="${output_dir}/${BINARY_NAME}"

  mkdir -p "${output_dir}"
  cp "${source_path}" "${target_path}"
}

print_summary() {
  local output_dir="$1"
  local flavor_label="$2"

  echo "Built Linux amd64 ${flavor_label} Rust binary with zig:"
  echo "  ${output_dir}/${BINARY_NAME}"
}

main() {
  local output_dir
  local flavor_label

  require_tool "zig" "non-Docker Linux amd64 Rust builds"
  require_tool "cargo-zigbuild" "non-Docker Linux amd64 Rust builds"

  export PATH="${HOME}/.cargo/bin:${PATH}"

  output_dir="$(resolve_output_dir)"
  flavor_label="$(build_flavor_label)"

  run_zigbuild
  copy_artifact "${output_dir}"
  print_summary "${output_dir}" "${flavor_label}"
}

main "$@"
