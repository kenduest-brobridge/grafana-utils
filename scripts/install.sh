#!/bin/sh

set -eu

REPO="${REPO:-kenduest-brobridge/grafana-util}"
BINARY_NAME="${BINARY_NAME:-grafana-util}"
VERSION="${VERSION:-latest}"
BIN_DIR="${BIN_DIR:-}"
ASSET_URL="${ASSET_URL:-}"
INSTALL_TMPDIR="${TMPDIR:-/tmp}"
RUST_ARTIFACT_FLAVOR="${RUST_ARTIFACT_FLAVOR:-standard}"
INSTALL_COMPLETION="${INSTALL_COMPLETION:-}"
COMPLETION_DIR="${COMPLETION_DIR:-}"
INSTALL_TTY="${INSTALL_TTY:-/dev/tty}"
INTERACTIVE=0

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'install.sh: %s\n' "$*" >&2
  exit 1
}

print_help() {
  cat <<'EOF'
Usage:
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive

Environment overrides:
  VERSION=0.10.0          Install one specific release tag instead of latest.
  BIN_DIR=/custom/bin     Install the binary into one writable directory.
  REPO=owner/repo         Override the GitHub repository source.
  ASSET_URL=file:///...   Install from one explicit archive URL.
  BINARY_NAME=name        Override the binary name inside the archive.
  RUST_ARTIFACT_FLAVOR=browser
                          Install the browser-enabled archive lane.
  INSTALL_COMPLETION=auto Install shell completion after the binary.
                          Supported values: auto, bash, zsh.
  COMPLETION_DIR=/custom  Override the completion output directory.

Options:
  --interactive           Ask for install directory and shell completion setup.
  -h, --help              Show this help.

Install directory selection:
  1. Use BIN_DIR if you set it.
  2. Otherwise use /usr/local/bin when it exists and is writable.
  3. Otherwise fall back to $HOME/.local/bin.

After install:
  If the install directory is not already on PATH, the script prints the
  exact export command to add it for the current shell.

Completion:
  INSTALL_COMPLETION=auto detects bash or zsh from SHELL. The installer writes
  Bash completion to ~/.local/share/bash-completion/completions/grafana-util
  and Zsh completion to ~/.zfunc/_grafana-util unless COMPLETION_DIR is set.
  In interactive Zsh installs, the installer can also add the required fpath
  setup to ~/.zshrc with a managed marker block.

Interactive mode:
  Use sh -s -- --interactive when piping through curl. Values provided through
  BIN_DIR, INSTALL_COMPLETION, or COMPLETION_DIR are treated as already chosen
  and are not asked again.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help|help)
      print_help
      exit 0
      ;;
    --interactive)
      INTERACTIVE=1
      shift
      ;;
    *)
      fail "unknown option: $1"
      ;;
  esac
done

require_tool() {
  tool_name="$1"
  command -v "$tool_name" >/dev/null 2>&1 || fail "${tool_name} is required"
}

require_tools() {
  require_tool curl
  require_tool tar
  require_tool install
  require_tool mktemp
}

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

expand_user_path() {
  case "$1" in
    "~") printf '%s\n' "$HOME" ;;
    "~/"*) printf '%s/%s\n' "$HOME" "${1#~/}" ;;
    *) printf '%s\n' "$1" ;;
  esac
}

detect_completion_shell() {
  shell_path="${SHELL:-}"
  [ -n "$shell_path" ] || return 1
  shell_name=$(basename "$shell_path")
  case "$shell_name" in
    bash|zsh)
      printf '%s\n' "$shell_name"
      ;;
    *)
      return 1
      ;;
  esac
}

open_interactive_tty() {
  [ "$INTERACTIVE" -eq 1 ] || return 0
  [ -r "$INSTALL_TTY" ] || fail "--interactive requires a readable terminal; tried ${INSTALL_TTY}"
  exec 3< "$INSTALL_TTY"
}

prompt_line() {
  prompt_text="$1"
  default_value="$2"
  answer=""

  if [ -n "$default_value" ]; then
    printf '%s [%s]: ' "$prompt_text" "$default_value" >&2
  else
    printf '%s: ' "$prompt_text" >&2
  fi

  if ! IFS= read -r answer <&3; then
    fail "unable to read interactive input"
  fi

  if [ -z "$answer" ]; then
    printf '%s\n' "$default_value"
  else
    printf '%s\n' "$answer"
  fi
}

prompt_yes_no() {
  prompt_text="$1"
  default_value="$2"

  while :; do
    answer=$(prompt_line "$prompt_text" "$default_value")
    case "$answer" in
      y|Y|yes|YES|Yes) return 0 ;;
      n|N|no|NO|No) return 1 ;;
      *) log "Please answer y or n." ;;
    esac
  done
}

prompt_install_dir() {
  [ "$INTERACTIVE" -eq 1 ] || return 0
  [ -z "$BIN_DIR" ] || return 0

  default_dir=$(resolve_default_bin_dir)
  while :; do
    chosen_dir=$(prompt_line "Install ${BINARY_NAME} into" "$default_dir")
    chosen_dir=$(expand_user_path "$chosen_dir")
    if mkdir -p "$chosen_dir" 2>/dev/null && [ -w "$chosen_dir" ]; then
      BIN_DIR="$chosen_dir"
      export BIN_DIR
      return 0
    fi
    log "Directory is not writable: ${chosen_dir}"
  done
}

prompt_completion_shell() {
  [ "$INTERACTIVE" -eq 1 ] || return 0
  [ -z "$INSTALL_COMPLETION" ] || return 0

  detected_shell=""
  if detected_shell=$(detect_completion_shell); then
    completion_prompt="Install ${detected_shell} shell completion?"
  else
    completion_prompt="Install shell completion?"
  fi

  if ! prompt_yes_no "$completion_prompt" "Y"; then
    INSTALL_COMPLETION="none"
    export INSTALL_COMPLETION
    return 0
  fi

  if [ -n "$detected_shell" ]; then
    INSTALL_COMPLETION="$detected_shell"
    export INSTALL_COMPLETION
    return 0
  fi

  while :; do
    chosen_shell=$(prompt_line "Which shell completion should be installed? (bash/zsh/none)" "none")
    case "$chosen_shell" in
      bash|zsh|none)
        INSTALL_COMPLETION="$chosen_shell"
        export INSTALL_COMPLETION
        return 0
        ;;
      *)
        log "Please choose bash, zsh, or none."
        ;;
    esac
  done
}

default_completion_dir() {
  case "$1" in
    bash) printf '%s\n' "${HOME}/.local/share/bash-completion/completions" ;;
    zsh) printf '%s\n' "${HOME}/.zfunc" ;;
    *) printf '%s\n' "" ;;
  esac
}

prompt_completion_dir() {
  [ "$INTERACTIVE" -eq 1 ] || return 0
  [ -z "$COMPLETION_DIR" ] || return 0

  case "$INSTALL_COMPLETION" in
    bash|zsh) ;;
    *) return 0 ;;
  esac

  default_dir=$(default_completion_dir "$INSTALL_COMPLETION")
  while :; do
    chosen_dir=$(prompt_line "Install ${INSTALL_COMPLETION} completion into" "$default_dir")
    chosen_dir=$(expand_user_path "$chosen_dir")
    if mkdir -p "$chosen_dir" 2>/dev/null && [ -w "$chosen_dir" ]; then
      COMPLETION_DIR="$chosen_dir"
      export COMPLETION_DIR
      return 0
    fi
    log "Directory is not writable: ${chosen_dir}"
  done
}

resolve_completion_shell() {
  case "$INSTALL_COMPLETION" in
    ""|none|false|0|no)
      printf '%s\n' ""
      ;;
    bash|zsh)
      printf '%s\n' "$INSTALL_COMPLETION"
      ;;
    auto)
      shell_path="${SHELL:-}"
      [ -n "$shell_path" ] || fail "INSTALL_COMPLETION=auto could not detect bash or zsh because SHELL is unset; set INSTALL_COMPLETION=bash or INSTALL_COMPLETION=zsh"
      shell_name=$(basename "$shell_path")
      case "$shell_name" in
        bash|zsh)
          printf '%s\n' "$shell_name"
          ;;
        *)
          fail "INSTALL_COMPLETION=auto could not detect bash or zsh from SHELL=${SHELL:-unset}; set INSTALL_COMPLETION=bash or INSTALL_COMPLETION=zsh"
          ;;
      esac
      ;;
    *)
      fail "unsupported INSTALL_COMPLETION: ${INSTALL_COMPLETION}; supported values: auto, bash, zsh"
      ;;
  esac
}

install_shell_completion() {
  completion_shell="$1"
  installed_binary="$2"

  [ -n "$completion_shell" ] || return 0
  [ -n "${HOME:-}" ] || fail "HOME is required when INSTALL_COMPLETION is set"

  case "$completion_shell" in
    bash)
      completion_dir="${COMPLETION_DIR:-${HOME}/.local/share/bash-completion/completions}"
      completion_path="${completion_dir}/${BINARY_NAME}"
      mkdir -p "$completion_dir" || fail "unable to create completion directory ${completion_dir}"
      "$installed_binary" completion bash > "$completion_path" || fail "failed to generate Bash completion"
      log "Installed Bash completion to ${completion_path}"
      ;;
    zsh)
      completion_dir="${COMPLETION_DIR:-${HOME}/.zfunc}"
      completion_path="${completion_dir}/_${BINARY_NAME}"
      mkdir -p "$completion_dir" || fail "unable to create completion directory ${completion_dir}"
      "$installed_binary" completion zsh > "$completion_path" || fail "failed to generate Zsh completion"
      log "Installed Zsh completion to ${completion_path}"
      maybe_update_zshrc_for_completion "$completion_dir"
      ;;
    *)
      fail "unsupported completion shell: ${completion_shell}"
      ;;
  esac
}

quote_for_zsh_double_quotes() {
  printf '%s\n' "$1" | sed 's/[\\`"$]/\\&/g'
}

zsh_fpath_entry() {
  completion_dir="$1"
  if [ "$completion_dir" = "${HOME}/.zfunc" ]; then
    printf '%s\n' '$HOME/.zfunc'
  else
    quote_for_zsh_double_quotes "$completion_dir"
  fi
}

zsh_completion_block() {
  fpath_entry=$(zsh_fpath_entry "$1")
  cat <<EOF
# >>> grafana-util completion fpath >>>
fpath=("${fpath_entry}" \$fpath)
# <<< grafana-util completion fpath <<<
EOF
}

zsh_completion_compdef_block() {
  cat <<'EOF'
# >>> grafana-util completion compdef >>>
if (( $+functions[compdef] )); then
  autoload -Uz _grafana-util
  compdef _grafana-util grafana-util
fi
# <<< grafana-util completion compdef <<<
EOF
}

write_zsh_completion_block() {
  completion_dir="$1"
  [ -n "${HOME:-}" ] || fail "HOME is required to update .zshrc"

  zshrc_path="${ZSHRC:-${HOME}/.zshrc}"
  zshrc_dir=$(dirname "$zshrc_path")
  mkdir -p "$zshrc_dir" || fail "unable to create ${zshrc_dir}"
  [ -e "$zshrc_path" ] || : > "$zshrc_path" || fail "unable to create ${zshrc_path}"
  [ -w "$zshrc_path" ] || fail "${zshrc_path} is not writable"

  block=$(zsh_completion_block "$completion_dir")
  compdef_block=$(zsh_completion_compdef_block)
  block_path=$(mktemp "${INSTALL_TMPDIR%/}/grafana-util-zshrc-block.XXXXXX")
  compdef_block_path=$(mktemp "${INSTALL_TMPDIR%/}/grafana-util-zshrc-compdef-block.XXXXXX")
  clean_path=$(mktemp "${INSTALL_TMPDIR%/}/grafana-util-zshrc-clean.XXXXXX")
  output_path=$(mktemp "${INSTALL_TMPDIR%/}/grafana-util-zshrc-output.XXXXXX")
  printf '%s\n' "$block" > "$block_path" || {
    rm -f "$block_path" "$compdef_block_path" "$clean_path" "$output_path"
    fail "unable to prepare Zsh completion block"
  }
  printf '%s\n' "$compdef_block" > "$compdef_block_path" || {
    rm -f "$block_path" "$compdef_block_path" "$clean_path" "$output_path"
    fail "unable to prepare Zsh completion compdef block"
  }

  awk '
    $0 == "# >>> grafana-util completion >>>" { skip = 1; next }
    $0 == "# <<< grafana-util completion <<<" { skip = 0; next }
    $0 == "# >>> grafana-util completion fpath >>>" { skip = 1; next }
    $0 == "# <<< grafana-util completion fpath <<<" { skip = 0; next }
    $0 == "# >>> grafana-util completion compdef >>>" { skip = 1; next }
    $0 == "# <<< grafana-util completion compdef <<<" { skip = 0; next }
    skip != 1 { print }
  ' "$zshrc_path" > "$clean_path" || {
    rm -f "$block_path" "$compdef_block_path" "$clean_path" "$output_path"
    fail "unable to read ${zshrc_path}"
  }

  awk -v block_path="$block_path" -v compdef_block_path="$compdef_block_path" '
    function print_block() {
      while ((getline block_line < block_path) > 0) {
        print block_line
      }
      close(block_path)
    }
    function print_compdef_block() {
      while ((getline compdef_block_line < compdef_block_path) > 0) {
        print compdef_block_line
      }
      close(compdef_block_path)
    }
    inserted != 1 && ($0 ~ /oh-my-zsh\.sh/ || $0 ~ /(^|[[:space:]])compinit([[:space:]]|$)/) {
      print_block()
      inserted = 1
    }
    { print }
    inserted == 1 && compdef_inserted != 1 && ($0 ~ /oh-my-zsh\.sh/ || $0 ~ /(^|[[:space:]])compinit([[:space:]]|$)/) {
      print_compdef_block()
      compdef_inserted = 1
    }
    END {
      if (inserted != 1) {
        if (NR > 0) {
          print ""
        }
        print_block()
      }
      if (compdef_inserted != 1) {
        print_compdef_block()
      }
    }
  ' "$clean_path" > "$output_path" || {
    rm -f "$block_path" "$compdef_block_path" "$clean_path" "$output_path"
    fail "unable to update ${zshrc_path}"
  }

  mv "$output_path" "$zshrc_path" || {
    rm -f "$block_path" "$compdef_block_path" "$clean_path" "$output_path"
    fail "unable to write ${zshrc_path}"
  }
  rm -f "$block_path" "$compdef_block_path" "$clean_path"
  log "Updated ${zshrc_path} to load Zsh completion."
  clear_zsh_completion_cache
  log "Open a new shell or run: exec zsh"
}

clear_zsh_completion_cache() {
  [ -n "${HOME:-}" ] || return 0

  cleared=0
  for compdump_path in "${HOME}"/.zcompdump "${HOME}"/.zcompdump-* "${HOME}"/.zcompdump.*; do
    [ -e "$compdump_path" ] || continue
    if rm -f "$compdump_path" 2>/dev/null; then
      cleared=1
    else
      log "Could not remove ${compdump_path}; remove it manually if Zsh completion stays stale."
    fi
  done

  if [ "$cleared" = "1" ]; then
    log "Cleared Zsh completion cache."
  fi
}

maybe_update_zshrc_for_completion() {
  completion_dir="$1"

  if [ "$INTERACTIVE" -eq 1 ]; then
    if prompt_yes_no "Update ~/.zshrc to load ${BINARY_NAME} completion?" "Y"; then
      write_zsh_completion_block "$completion_dir"
    else
      log "Skipped ~/.zshrc update."
      log "For Zsh, ensure this appears before compinit: fpath=(\"${completion_dir}\" \$fpath)"
    fi
  else
    log "For Zsh, ensure this appears before compinit: fpath=(\"${completion_dir}\" \$fpath)"
  fi
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

resolve_platform() {
  OS=$(detect_os)
  ARCH=$(detect_arch)
  PLATFORM="${OS}-${ARCH}"

  case "$PLATFORM" in
    linux-amd64|macos-arm64) ;;
    *)
      fail "no published release binary for ${PLATFORM}; supported targets: linux-amd64, macos-arm64"
      ;;
  esac

  export PLATFORM
}

resolve_archive_url() {
  ARTIFACT_SUFFIX="$(resolve_artifact_suffix)"

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

  export release_tag archive_url
}

configure_interactive_choices() {
  open_interactive_tty
  prompt_install_dir
  prompt_completion_shell
  prompt_completion_dir
}

resolve_install_dir() {
  install_dir=$(resolve_default_bin_dir)
  mkdir -p "$install_dir" || fail "unable to create install directory ${install_dir}"
  [ -w "$install_dir" ] || fail "${install_dir} is not writable; set BIN_DIR to a writable directory"

  export install_dir
}

prepare_tempdir() {
  tmpdir=$(mktemp -d "${INSTALL_TMPDIR%/}/grafana-util-install.XXXXXX")
  archive_path="${tmpdir}/grafana-util.tar.gz"

  export tmpdir archive_path
}

cleanup() {
  rm -rf "$tmpdir"
}

download_archive() {
  log "Downloading ${BINARY_NAME} for ${PLATFORM} from ${archive_url}"
  curl -fsSL "$archive_url" -o "$archive_path" || fail "failed to download ${archive_url}"
}

extract_binary() {
  tar -xzf "$archive_path" -C "$tmpdir" || fail "failed to extract ${archive_path}"

  binary_path="${tmpdir}/${BINARY_NAME}"
  [ -f "$binary_path" ] || fail "archive did not contain ${BINARY_NAME}"

  export binary_path
}

install_binary() {
  target_path="${install_dir}/${BINARY_NAME}"
  install -m 0755 "$binary_path" "$target_path" || fail "failed to install ${BINARY_NAME} into ${install_dir}"

  log "Installed ${BINARY_NAME} to ${target_path}"
  export target_path
}

install_requested_completion() {
  completion_shell=$(resolve_completion_shell)
  install_shell_completion "$completion_shell" "$target_path"
}

print_path_notice() {
  case ":${PATH:-}:" in
    *:"${install_dir}":*) return 0 ;;
  esac

  shell_name=$(basename "${SHELL:-sh}")
  log ""
  log "The install directory is not currently on PATH."
  log "Add ${install_dir} to PATH if needed:"
  case "$shell_name" in
    zsh)
      log "  echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.zshrc"
      log "  exec zsh"
      ;;
    bash)
      log "  echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.bashrc"
      log "  exec bash"
      ;;
    *)
      log "  export PATH=\"${install_dir}:\$PATH\""
      ;;
  esac
}

main() {
  require_tools
  resolve_platform
  resolve_archive_url
  configure_interactive_choices
  resolve_install_dir
  prepare_tempdir
  trap cleanup EXIT INT TERM
  download_archive
  extract_binary
  install_binary
  install_requested_completion
  print_path_notice
}

main
