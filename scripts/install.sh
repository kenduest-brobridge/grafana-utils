#!/bin/sh

set -eu

REPO="${REPO:-kenduest-brobridge/grafana-utils}"
BINARY_NAME="${BINARY_NAME:-grafana-util}"
VERSION="${VERSION:-latest}"
BIN_DIR="${BIN_DIR:-}"
ASSET_URL="${ASSET_URL:-}"
INSTALL_TMPDIR="${TMPDIR:-/tmp}"
RUST_ARTIFACT_FLAVOR="${RUST_ARTIFACT_FLAVOR:-standard}"

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'install.sh: %s\n' "$*" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"
command -v install >/dev/null 2>&1 || fail "install is required"
command -v mktemp >/dev/null 2>&1 || fail "mktemp is required"

resolve_artifact_suffix() {
  case "$RUST_ARTIFACT_FLAVOR" in
    standard) printf '%s\n' "" ;;
    browser) printf '%s\n' "-browser" ;;
    *) fail "unsupported RUST_ARTIFACT_FLAVOR: ${RUST_ARTIFACT_FLAVOR}; supported values: standard, browser" ;;
  esac
}

normalize_tag() {
  case "$1" in
    v*) printf '%s\n' "$1" ;;
    *) printf 'v%s\n' "$1" ;;
  esac
}

detect_os() {
  os_name=$(uname -s 2>/dev/null || printf '')
  case "$os_name" in
    Darwin) printf 'macos\n' ;;
    Linux) printf 'linux\n' ;;
    *) fail "unsupported operating system: ${os_name:-unknown}" ;;
  esac
}

detect_arch() {
  arch_name=$(uname -m 2>/dev/null || printf '')
  case "$arch_name" in
    x86_64|amd64) printf 'amd64\n' ;;
    arm64|aarch64) printf 'arm64\n' ;;
    *) fail "unsupported architecture: ${arch_name:-unknown}" ;;
  esac
}

resolve_default_bin_dir() {
  if [ -n "$BIN_DIR" ]; then
    printf '%s\n' "$BIN_DIR"
    return 0
  fi
  if [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
    printf '/usr/local/bin\n'
    return 0
  fi
  user_bin="${HOME}/.local/bin"
  if [ ! -d "$user_bin" ]; then
    mkdir -p "$user_bin" || fail "unable to create ${user_bin}; set BIN_DIR to a writable directory"
  fi
  if [ ! -w "$user_bin" ]; then
    fail "${user_bin} is not writable; set BIN_DIR to a writable directory"
  fi
  printf '%s\n' "$user_bin"
}

resolve_latest_tag() {
  api_url="https://api.github.com/repos/${REPO}/releases/latest"
  response=$(curl -fsSL "$api_url") || fail "failed to query latest release from ${api_url}"
  tag=$(
    printf '%s\n' "$response" |
      sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
      head -n 1
  )
  [ -n "$tag" ] || fail "unable to parse latest release tag from ${api_url}"
  printf '%s\n' "$tag"
}

OS=$(detect_os)
ARCH=$(detect_arch)
PLATFORM="${OS}-${ARCH}"
ARTIFACT_SUFFIX="$(resolve_artifact_suffix)"

case "$PLATFORM" in
  linux-amd64|macos-arm64) ;;
  *)
    fail "no published release binary for ${PLATFORM}; supported targets: linux-amd64, macos-arm64"
    ;;
esac

if [ -n "$ASSET_URL" ]; then
  release_tag="$VERSION"
  archive_url="$ASSET_URL"
else
  if [ "$VERSION" = "latest" ]; then
    release_tag=$(resolve_latest_tag)
  else
    release_tag=$(normalize_tag "$VERSION")
  fi
  archive_name="grafana-utils-rust-${PLATFORM}${ARTIFACT_SUFFIX}-${release_tag}.tar.gz"
  archive_url="https://github.com/${REPO}/releases/download/${release_tag}/${archive_name}"
fi

install_dir=$(resolve_default_bin_dir)
mkdir -p "$install_dir" || fail "unable to create install directory ${install_dir}"
[ -w "$install_dir" ] || fail "${install_dir} is not writable; set BIN_DIR to a writable directory"

tmpdir=$(mktemp -d "${INSTALL_TMPDIR%/}/grafana-util-install.XXXXXX")
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

archive_path="${tmpdir}/grafana-util.tar.gz"
log "Downloading ${BINARY_NAME} for ${PLATFORM} from ${archive_url}"
curl -fsSL "$archive_url" -o "$archive_path" || fail "failed to download ${archive_url}"
tar -xzf "$archive_path" -C "$tmpdir" || fail "failed to extract ${archive_path}"

binary_path="${tmpdir}/${BINARY_NAME}"
[ -f "$binary_path" ] || fail "archive did not contain ${BINARY_NAME}"

target_path="${install_dir}/${BINARY_NAME}"
install -m 0755 "$binary_path" "$target_path" || fail "failed to install ${BINARY_NAME} into ${install_dir}"

log "Installed ${BINARY_NAME} to ${target_path}"
case ":${PATH:-}:" in
  *:"${install_dir}":*) ;;
  *)
    log ""
    log "Add ${install_dir} to PATH if needed:"
    log "  export PATH=\"${install_dir}:\$PATH\""
    ;;
esac
resolve_artifact_suffix() {
  case "$RUST_ARTIFACT_FLAVOR" in
    standard) printf '%s\n' "" ;;
    browser) printf '%s\n' "-browser" ;;
    *) fail "unsupported RUST_ARTIFACT_FLAVOR: ${RUST_ARTIFACT_FLAVOR}; supported values: standard, browser" ;;
  esac
}
