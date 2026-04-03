#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_BROWSER="${BUILD_BROWSER:-0}"
ARTIFACT_PATH="${REPO_ROOT}/dist/linux-amd64/grafana-util"
VALIDATION_IMAGE="${VALIDATION_IMAGE:-debian:bookworm-slim}"
VALIDATION_CONTAINER_NAME="${VALIDATION_CONTAINER_NAME:-grafana-utils-rust-linux-amd64-validate}"

if [[ "${BUILD_BROWSER}" != "0" ]]; then
  ARTIFACT_PATH="${REPO_ROOT}/dist/linux-amd64-browser/grafana-util"
  VALIDATION_CONTAINER_NAME="${VALIDATION_CONTAINER_NAME:-grafana-utils-rust-linux-amd64-browser-validate}"
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Error: docker is required for Linux artifact validation." >&2
  exit 1
fi

if [[ ! -x "${ARTIFACT_PATH}" ]]; then
  echo "Error: Linux amd64 artifact not found at ${ARTIFACT_PATH}." >&2
  if [[ "${BUILD_BROWSER}" != "0" ]]; then
    echo "Run 'make build-rust-linux-amd64-browser' first." >&2
  else
    echo "Run 'make build-rust-linux-amd64' first." >&2
  fi
  exit 1
fi

docker rm -f "${VALIDATION_CONTAINER_NAME}" >/dev/null 2>&1 || true

docker run --rm \
  --platform linux/amd64 \
  --name "${VALIDATION_CONTAINER_NAME}" \
  -v "${ARTIFACT_PATH}:/usr/local/bin/grafana-util:ro" \
  "${VALIDATION_IMAGE}" \
  /usr/local/bin/grafana-util "$@"
