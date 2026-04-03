#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
out_file="$repo_root/docs/context/FILE_INDEX.json"

timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

core_files='grafana_utils/__main__.py
grafana_utils/unified_cli.py
grafana_utils/dashboard_cli.py
grafana_utils/alert_cli.py
grafana_utils/access_cli.py
grafana_utils/access/parser.py
grafana_utils/access/workflows.py
grafana_utils/http_transport.py
grafana_utils/clients/dashboard_client.py
grafana_utils/clients/alert_client.py
grafana_utils/clients/access_client.py
grafana_utils/clients/datasource_client.py
rust/src/bin/grafana-util.rs
rust/src/access_org.rs
rust/src/access.rs
rust/src/access_cli_defs.rs
tests/test_python_unified_cli.py
tests/test_python_dashboard_cli.py
tests/test_python_alert_cli.py
tests/test_python_access_cli.py
tests/test_python_datasource_client.py
Makefile
pyproject.toml
README.md
docs/DEVELOPER.md
docs/context/PROJECT_MAP.md
docs/context/CHANGE_IMPACT.md'

json_array_from_lines() {
  if [ "$#" -eq 0 ]; then
    printf '[]'
    return
  fi

  first=1
  printf '['
  for item in "$@"; do
    if [ "$first" -eq 1 ]; then
      first=0
    else
      printf ','
    fi
    esc=$(printf '%s' "$item" | sed 's/\\/\\\\/g; s/"/\\"/g')
    printf '"%s"' "$esc"
  done
  printf ']'
}

json_array_from_block() {
  block="$1"
  if [ -z "$block" ]; then
    printf '[]'
    return
  fi
  old_ifs=$IFS
  IFS='
'
  # shellcheck disable=SC2086
  set -- $block
  IFS=$old_ifs
  json_array_from_lines "$@"
}

extract_public_apis() {
  file="$1"
  abs="$repo_root/$file"
  if [ ! -f "$abs" ]; then
    printf '[]'
    return
  fi

  if printf '%s' "$file" | rg -q '\.py$'; then
    set +e
    names=$(rg -N '^(def|class)\s+[A-Za-z_][A-Za-z0-9_]*' "$abs" | sed -E 's/^(def|class)\s+([A-Za-z_][A-Za-z0-9_]*).*/\2/' | sort -u)
    rc=$?
    set -e
    if [ "$rc" -ne 0 ]; then
      names=''
    fi
    json_array_from_block "$names"
    return
  fi

  if printf '%s' "$file" | rg -q '\.rs$'; then
    set +e
    names=$(rg -N '^pub\s+(fn|struct|enum|mod)\s+[A-Za-z_][A-Za-z0-9_]*' "$abs" | sed -E 's/^pub\s+(fn|struct|enum|mod)\s+([A-Za-z_][A-Za-z0-9_]*).*/\2/' | sort -u)
    rc=$?
    set -e
    if [ "$rc" -ne 0 ]; then
      names=''
    fi
    json_array_from_block "$names"
    return
  fi

  printf '[]'
}

purpose_for() {
  case "$1" in
    grafana_utils/unified_cli.py) printf '%s' 'Unified Python CLI dispatcher for command groups.' ;;
    grafana_utils/__main__.py) printf '%s' 'Python module entrypoint that invokes unified CLI.' ;;
    grafana_utils/dashboard_cli.py) printf '%s' 'Dashboard command parser and workflow orchestration.' ;;
    grafana_utils/alert_cli.py) printf '%s' 'Alert command parser and workflow orchestration.' ;;
    grafana_utils/access_cli.py) printf '%s' 'Access command parser and workflow orchestration.' ;;
    grafana_utils/access/parser.py) printf '%s' 'Argparse wiring and CLI argument definitions for access commands.' ;;
    grafana_utils/access/workflows.py) printf '%s' 'Access user/org/team/service-account workflow operations.' ;;
    grafana_utils/http_transport.py) printf '%s' 'Shared pluggable JSON HTTP transport implementation and selection.' ;;
    grafana_utils/clients/dashboard_client.py) printf '%s' 'Dashboard API client over shared transport.' ;;
    grafana_utils/clients/alert_client.py) printf '%s' 'Alert API client over shared transport.' ;;
    grafana_utils/clients/access_client.py) printf '%s' 'Access API client over shared transport.' ;;
    grafana_utils/clients/datasource_client.py) printf '%s' 'Datasource API client over shared transport.' ;;
    rust/src/bin/grafana-util.rs) printf '%s' 'Rust unified CLI binary entrypoint.' ;;
    rust/src/access_org.rs) printf '%s' 'Rust organization list/add/modify/delete/export/import implementation.' ;;
    rust/src/access.rs) printf '%s' 'Rust access command orchestration and dispatch.' ;;
    rust/src/access_cli_defs.rs) printf '%s' 'Rust access CLI argument and subcommand definitions.' ;;
    tests/test_python_unified_cli.py) printf '%s' 'Python unittest coverage for unified CLI routing and help UX.' ;;
    tests/test_python_dashboard_cli.py) printf '%s' 'Python unittest coverage for dashboard CLI behavior.' ;;
    tests/test_python_alert_cli.py) printf '%s' 'Python unittest coverage for alert CLI behavior.' ;;
    tests/test_python_access_cli.py) printf '%s' 'Python unittest coverage for access CLI behavior.' ;;
    tests/test_python_datasource_client.py) printf '%s' 'Python unittest coverage for datasource client and transport interactions.' ;;
    Makefile) printf '%s' 'Top-level build and test task shortcuts.' ;;
    pyproject.toml) printf '%s' 'Python package metadata and console entrypoints.' ;;
    README.md) printf '%s' 'Primary operator-facing usage documentation.' ;;
    docs/DEVELOPER.md) printf '%s' 'Internal maintainer behavior and tradeoff notes.' ;;
    docs/context/PROJECT_MAP.md) printf '%s' 'Fast-path architecture and edit map for AI agents.' ;;
    docs/context/CHANGE_IMPACT.md) printf '%s' 'Change-impact matrix linking modules to tests/docs.' ;;
    *) printf '%s' 'Repository file.' ;;
  esac
}

depends_on_for() {
  case "$1" in
    grafana_utils/unified_cli.py) json_array_from_lines grafana_utils/dashboard_cli.py grafana_utils/alert_cli.py grafana_utils/access_cli.py grafana_utils/datasource_cli.py grafana_utils/sync_cli.py ;;
    grafana_utils/dashboard_cli.py) json_array_from_lines grafana_utils/clients/dashboard_client.py grafana_utils/http_transport.py ;;
    grafana_utils/alert_cli.py) json_array_from_lines grafana_utils/clients/alert_client.py grafana_utils/http_transport.py ;;
    grafana_utils/access_cli.py) json_array_from_lines grafana_utils/access/parser.py grafana_utils/access/workflows.py grafana_utils/clients/access_client.py grafana_utils/http_transport.py ;;
    grafana_utils/access/parser.py) json_array_from_lines grafana_utils/http_transport.py ;;
    grafana_utils/access/workflows.py) json_array_from_lines grafana_utils/clients/access_client.py grafana_utils/access/models.py ;;
    grafana_utils/http_transport.py) json_array_from_lines requests httpx ;;
    rust/src/access_org.rs) json_array_from_lines rust/src/access.rs rust/src/access_cli_defs.rs rust/src/access_render.rs ;;
    rust/src/access.rs) json_array_from_lines rust/src/access_cli_defs.rs rust/src/access_org.rs ;;
    tests/test_python_unified_cli.py) json_array_from_lines grafana_utils/unified_cli.py ;;
    tests/test_python_dashboard_cli.py) json_array_from_lines grafana_utils/dashboard_cli.py ;;
    tests/test_python_alert_cli.py) json_array_from_lines grafana_utils/alert_cli.py ;;
    tests/test_python_access_cli.py) json_array_from_lines grafana_utils/access_cli.py grafana_utils/access/parser.py grafana_utils/access/workflows.py ;;
    tests/test_python_datasource_client.py) json_array_from_lines grafana_utils/clients/datasource_client.py grafana_utils/http_transport.py ;;
    *) json_array_from_lines ;;
  esac
}

owned_tests_for() {
  case "$1" in
    grafana_utils/unified_cli.py|grafana_utils/__main__.py) json_array_from_lines tests/test_python_unified_cli.py ;;
    grafana_utils/dashboard_cli.py) json_array_from_lines tests/test_python_dashboard_cli.py tests/test_python_dashboard_inspection_cli.py ;;
    grafana_utils/alert_cli.py) json_array_from_lines tests/test_python_alert_cli.py tests/test_python_alert_sync_workbench.py ;;
    grafana_utils/access_cli.py|grafana_utils/access/parser.py|grafana_utils/access/workflows.py) json_array_from_lines tests/test_python_access_cli.py tests/test_python_access_pending_cli_staging.py ;;
    grafana_utils/http_transport.py) json_array_from_lines tests/test_python_datasource_client.py tests/test_python_access_cli.py tests/test_python_alert_cli.py tests/test_python_dashboard_cli.py ;;
    rust/src/access_org.rs|rust/src/access.rs|rust/src/access_cli_defs.rs) json_array_from_lines rust/src/access_rust_tests.rs ;;
    *) json_array_from_lines ;;
  esac
}

tags_for() {
  case "$1" in
    grafana_utils/unified_cli.py|grafana_utils/__main__.py) json_array_from_lines python cli entrypoint ;;
    grafana_utils/dashboard_cli.py) json_array_from_lines python cli dashboard ;;
    grafana_utils/alert_cli.py) json_array_from_lines python cli alert ;;
    grafana_utils/access_cli.py|grafana_utils/access/parser.py|grafana_utils/access/workflows.py) json_array_from_lines python cli access ;;
    grafana_utils/http_transport.py) json_array_from_lines python transport http ;;
    grafana_utils/clients/*.py) json_array_from_lines python client http ;;
    rust/src/*.rs) json_array_from_lines rust cli ;;
    tests/test_python_*.py) json_array_from_lines test python unittest ;;
    Makefile) json_array_from_lines build test tooling ;;
    pyproject.toml) json_array_from_lines packaging python ;;
    README.md|docs/DEVELOPER.md|docs/context/*.md) json_array_from_lines docs ;;
    *) json_array_from_lines ;;
  esac
}

mkdir -p "$repo_root/docs/context"

tmp_file=$(mktemp)
trap 'rm -f "$tmp_file"' EXIT INT HUP TERM

{
  printf '{\n'
  printf '  "metadata": {\n'
  printf '    "schema_version": 1,\n'
  printf '    "generated_at": "%s",\n' "$timestamp"
  printf '    "generator": "scripts/update_context_index.sh"\n'
  printf '  },\n'
  printf '  "entries": [\n'

  first=1
  printf '%s\n' "$core_files" | LC_ALL=C sort | while IFS= read -r path; do
    [ -n "$path" ] || continue

    purpose=$(purpose_for "$path" | sed 's/\\/\\\\/g; s/"/\\"/g')
    public_apis=$(extract_public_apis "$path")
    depends_on=$(depends_on_for "$path")
    owned_tests=$(owned_tests_for "$path")
    tags=$(tags_for "$path")

    if [ "$first" -eq 1 ]; then
      first=0
    else
      printf ',\n'
    fi

    path_esc=$(printf '%s' "$path" | sed 's/\\/\\\\/g; s/"/\\"/g')
    printf '    {\n'
    printf '      "path": "%s",\n' "$path_esc"
    printf '      "purpose": "%s",\n' "$purpose"
    printf '      "public_apis": %s,\n' "$public_apis"
    printf '      "depends_on": %s,\n' "$depends_on"
    printf '      "owned_tests": %s,\n' "$owned_tests"
    printf '      "tags": %s\n' "$tags"
    printf '    }'
  done

  printf '\n'
  printf '  ]\n'
  printf '}\n'
} > "$tmp_file"

mv "$tmp_file" "$out_file"
