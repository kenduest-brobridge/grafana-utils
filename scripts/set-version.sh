#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION_FILE="${REPO_ROOT}/VERSION"
PYPROJECT_TOML="${REPO_ROOT}/pyproject.toml"
CARGO_TOML="${REPO_ROOT}/rust/Cargo.toml"

usage() {
  cat <<'EOF'
Usage:
  bash ./scripts/set-version.sh --sync-from-file
  bash ./scripts/set-version.sh --print-current
  bash ./scripts/set-version.sh --version 0.2.9.dev1
  bash ./scripts/set-version.sh --version 0.2.9-dev.1
  bash ./scripts/set-version.sh --version 0.2.9
  bash ./scripts/set-version.sh --tag v0.2.9
  bash ./scripts/set-version.sh --version 0.2.9.dev1 --dry-run

Options:
  --sync-from-file Sync pyproject.toml and rust/Cargo.toml from ./VERSION.
  --print-current  Print the current Python and Rust package versions.
  --version VALUE  Set both source versions from one version string.
                   Accepts Python dev form (X.Y.Z.devN), Rust dev form
                   (X.Y.Z-dev.N), or release form (X.Y.Z).
  --tag TAG        Set both source versions from one release tag like v0.2.9.
  --dry-run        Print the derived versions without editing files.
  -h, --help       Show this help text.
EOF
}

python_version() {
  sed -n 's/^version = "\(.*\)"$/\1/p' "${PYPROJECT_TOML}" | head -n 1
}

rust_version() {
  sed -n 's/^version = "\(.*\)"$/\1/p' "${CARGO_TOML}" | head -n 1
}

canonical_version() {
  tr -d '[:space:]' < "${VERSION_FILE}"
}

replace_version_line() {
  local file="$1"
  local expected="$2"
  local replacement="$3"
  perl -0pi -e "s/version = \"\Q${expected}\E\"/version = \"${replacement}\"/" "${file}"
}

derive_versions() {
  local value="$1"
  if [[ "${value}" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)$ ]]; then
    DERIVED_PYTHON_VERSION="${BASH_REMATCH[1]}"
    DERIVED_RUST_VERSION="${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "${value}" =~ ^([0-9]+\.[0-9]+\.[0-9]+)\.dev([0-9]+)$ ]]; then
    DERIVED_PYTHON_VERSION="${BASH_REMATCH[1]}.dev${BASH_REMATCH[2]}"
    DERIVED_RUST_VERSION="${BASH_REMATCH[1]}-dev.${BASH_REMATCH[2]}"
    return 0
  fi
  if [[ "${value}" =~ ^([0-9]+\.[0-9]+\.[0-9]+)-dev\.([0-9]+)$ ]]; then
    DERIVED_PYTHON_VERSION="${BASH_REMATCH[1]}.dev${BASH_REMATCH[2]}"
    DERIVED_RUST_VERSION="${BASH_REMATCH[1]}-dev.${BASH_REMATCH[2]}"
    return 0
  fi
  if [[ "${value}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    DERIVED_PYTHON_VERSION="${value}"
    DERIVED_RUST_VERSION="${value}"
    return 0
  fi
  echo "Error: unsupported version format: ${value}" >&2
  echo "Expected X.Y.Z, X.Y.Z.devN, X.Y.Z-dev.N, or vX.Y.Z." >&2
  exit 1
}

PRINT_CURRENT=0
SYNC_FROM_FILE=0
DRY_RUN=0
VERSION_INPUT=""
TAG_INPUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --print-current)
      PRINT_CURRENT=1
      shift
      ;;
    --sync-from-file)
      SYNC_FROM_FILE=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --version)
      VERSION_INPUT="${2:-}"
      shift 2
      ;;
    --tag)
      TAG_INPUT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "${PRINT_CURRENT}" -eq 1 ]]; then
  echo "Canonical: $(canonical_version)"
  echo "Python:    $(python_version)"
  echo "Rust:      $(rust_version)"
  exit 0
fi

if [[ "${SYNC_FROM_FILE}" -eq 1 && ( -n "${VERSION_INPUT}" || -n "${TAG_INPUT}" ) ]]; then
  echo "Error: --sync-from-file cannot be combined with --version or --tag." >&2
  exit 1
fi

if [[ -n "${VERSION_INPUT}" && -n "${TAG_INPUT}" ]]; then
  echo "Error: use either --version or --tag, not both." >&2
  exit 1
fi

if [[ "${SYNC_FROM_FILE}" -eq 0 && -z "${VERSION_INPUT}" && -z "${TAG_INPUT}" ]]; then
  echo "Error: use one of --sync-from-file, --version, or --tag." >&2
  exit 1
fi

if [[ "${SYNC_FROM_FILE}" -eq 1 ]]; then
  derive_versions "$(canonical_version)"
elif [[ -n "${TAG_INPUT}" ]]; then
  derive_versions "${TAG_INPUT}"
else
  derive_versions "${VERSION_INPUT}"
fi

CURRENT_PYTHON_VERSION="$(python_version)"
CURRENT_RUST_VERSION="$(rust_version)"

echo "Current versions:"
echo "  Canonical: $(canonical_version)"
echo "  Python: ${CURRENT_PYTHON_VERSION}"
echo "  Rust:   ${CURRENT_RUST_VERSION}"
echo "Target versions:"
echo "  Python: ${DERIVED_PYTHON_VERSION}"
echo "  Rust:   ${DERIVED_RUST_VERSION}"

if [[ "${DRY_RUN}" -eq 1 ]]; then
  exit 0
fi

if [[ "${SYNC_FROM_FILE}" -eq 0 ]]; then
  printf '%s\n' "${DERIVED_PYTHON_VERSION}" > "${VERSION_FILE}"
fi

replace_version_line "${PYPROJECT_TOML}" "${CURRENT_PYTHON_VERSION}" "${DERIVED_PYTHON_VERSION}"
replace_version_line "${CARGO_TOML}" "${CURRENT_RUST_VERSION}" "${DERIVED_RUST_VERSION}"

echo "Updated:"
echo "  ${VERSION_FILE}"
echo "  ${PYPROJECT_TOML}"
echo "  ${CARGO_TOML}"
