#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 <output-dir> <binary-dir> <package-name>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="$1"
BINARY_DIR="$2"
PACKAGE_NAME="$3"

mkdir -p "${OUTPUT_DIR}"

for binary in grafana-util grafana-access-utils; do
  if [[ ! -f "${BINARY_DIR}/${binary}" ]]; then
    echo "Error: missing Rust binary ${BINARY_DIR}/${binary}" >&2
    exit 1
  fi
done

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/grafana-utils-rust-package.XXXXXX")"
PACKAGE_DIR="${TMP_DIR}/${PACKAGE_NAME}"
trap 'rm -rf "${TMP_DIR}"' EXIT

mkdir -p "${PACKAGE_DIR}/bin" "${PACKAGE_DIR}/docs"

cp "${BINARY_DIR}/grafana-util" "${PACKAGE_DIR}/bin/grafana-util"
cp "${BINARY_DIR}/grafana-access-utils" "${PACKAGE_DIR}/bin/grafana-access-utils"
cp "${REPO_ROOT}/README.md" "${PACKAGE_DIR}/README.md"
cp "${REPO_ROOT}/README.zh-TW.md" "${PACKAGE_DIR}/README.zh-TW.md"
cp "${REPO_ROOT}/LICENSE" "${PACKAGE_DIR}/LICENSE"
cp "${REPO_ROOT}/docs/user-guide.md" "${PACKAGE_DIR}/docs/user-guide.md"
cp "${REPO_ROOT}/docs/user-guide-TW.md" "${PACKAGE_DIR}/docs/user-guide-TW.md"

tar -C "${TMP_DIR}" -czf "${OUTPUT_DIR}/${PACKAGE_NAME}.tar.gz" "${PACKAGE_NAME}"

echo "Packaged Rust distribution:"
echo "  ${OUTPUT_DIR}/${PACKAGE_NAME}.tar.gz"
