#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RUST_LIVE_SCOPE=sync
exec "${ROOT_DIR}/scripts/test-rust-live-grafana.sh"
