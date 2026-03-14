#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$ROOT_DIR/rust"
CARGO_BIN="${CARGO_BIN:-cargo}"
GRAFANA_IMAGE="${GRAFANA_IMAGE:-grafana/grafana:12.4.1}"
GRAFANA_PORT="${GRAFANA_PORT:-}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-admin}"
GRAFANA_API_TOKEN="${GRAFANA_API_TOKEN:-}"
GRAFANA_URL=""
CONTAINER_NAME="${GRAFANA_CONTAINER_NAME:-grafana-util-rust-live-$$}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/grafana-util-rust-live.XXXXXX")"
DASHBOARD_EXPORT_DIR="${WORK_DIR}/dashboards"
DASHBOARD_DRY_RUN_DIR="${WORK_DIR}/dashboards-dry-run"
ALERT_EXPORT_DIR="${WORK_DIR}/alerts"
MULTI_ORG_EXPORT_DIR="${WORK_DIR}/dashboards-all-orgs"

cleanup() {
  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
  rm -rf "${WORK_DIR}"
}

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

api() {
  local method="$1"
  local path="$2"
  local payload="${3:-}"

  if [[ -n "${payload}" ]]; then
    curl --silent --show-error --fail-with-body \
      -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
      -H 'Content-Type: application/json' \
      -X "${method}" \
      "${GRAFANA_URL}${path}" \
      --data-binary "${payload}"
    return
  fi

  curl --silent --show-error --fail-with-body \
    -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
    -X "${method}" \
    "${GRAFANA_URL}${path}"
}

api_org() {
  local org_id="$1"
  local method="$2"
  local path="$3"
  local payload="${4:-}"

  if [[ -n "${payload}" ]]; then
    curl --silent --show-error --fail-with-body \
      -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
      -H "X-Grafana-Org-Id: ${org_id}" \
      -H 'Content-Type: application/json' \
      -X "${method}" \
      "${GRAFANA_URL}${path}" \
      --data-binary "${payload}"
    return
  fi

  curl --silent --show-error --fail-with-body \
    -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
    -H "X-Grafana-Org-Id: ${org_id}" \
    -X "${method}" \
    "${GRAFANA_URL}${path}"
}

wait_for_grafana() {
  local attempts=0
  until curl --silent --show-error --fail \
    -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
    "${GRAFANA_URL}/api/health" >/dev/null; do
    attempts=$((attempts + 1))
    if [[ "${attempts}" -ge 60 ]]; then
      fail "Grafana did not become ready at ${GRAFANA_URL}"
    fi
    sleep 2
  done
}

json_field() {
  local field="$1"
  jq -r --arg field "${field}" '.[$field] // empty'
}

rewrite_contact_point_url() {
  local path="$1"
  local url="$2"
  local tmp_path="${path}.tmp"

  jq --arg url "${url}" '.spec.settings.url = $url' "${path}" >"${tmp_path}" \
    || fail "failed to rewrite contact point URL in ${path}"
  mv "${tmp_path}" "${path}"
}

create_api_token() {
  local response=""
  local service_account_id=""

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  if response="$(api POST "/api/auth/keys" '{
    "name": "grafana-util-rust-live",
    "role": "Admin",
    "secondsToLive": 3600
  }' 2>/dev/null)"; then
    GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  fi

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  response="$(api POST "/api/serviceaccounts" '{
    "name": "grafana-util-rust-live",
    "role": "Admin",
    "isDisabled": false
  }')"
  service_account_id="$(printf '%s' "${response}" | json_field id)"
  [[ -n "${service_account_id}" ]] || fail "failed to create Grafana service account for token auth"

  response="$(api POST "/api/serviceaccounts/${service_account_id}/tokens" '{
    "name": "grafana-util-rust-live",
    "secondsToLive": 3600
  }')"
  GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  [[ -n "${GRAFANA_API_TOKEN}" ]] || fail "failed to create Grafana API token"
}

start_grafana() {
  local publish_args=()

  if [[ -n "${GRAFANA_PORT}" ]]; then
    publish_args=(-p "127.0.0.1:${GRAFANA_PORT}:3000")
  else
    publish_args=(-p "127.0.0.1::3000")
  fi

  docker run -d \
    --name "${CONTAINER_NAME}" \
    "${publish_args[@]}" \
    -e "GF_SECURITY_ADMIN_USER=${GRAFANA_USER}" \
    -e "GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD}" \
    -e "GF_USERS_ALLOW_SIGN_UP=false" \
    "${GRAFANA_IMAGE}" >/dev/null

  if [[ -z "${GRAFANA_PORT}" ]]; then
    GRAFANA_PORT="$(docker port "${CONTAINER_NAME}" 3000/tcp | awk -F: 'END {print $NF}')"
  fi
  GRAFANA_URL="http://127.0.0.1:${GRAFANA_PORT}"
  wait_for_grafana
}

build_rust_bins() {
  "${CARGO_BIN}" build --quiet \
    --manifest-path "${RUST_DIR}/Cargo.toml" \
    --bin grafana-util \
    --bin grafana-util
}

seed_datasource() {
  api POST "/api/datasources" '{
    "name": "Smoke Prometheus",
    "type": "prometheus",
    "access": "proxy",
    "url": "http://prometheus.invalid",
    "isDefault": true
  }' >/dev/null
}

seed_dashboard() {
  local title="$1"
  local uid="${2:-smoke-dashboard}"
  local org_id="${3:-}"
  local api_runner="api"
  if [[ -n "${org_id}" ]]; then
    api_runner="api_org ${org_id}"
  fi
  ${api_runner} POST "/api/dashboards/db" "{
    \"dashboard\": {
      \"id\": null,
      \"uid\": \"${uid}\",
      \"title\": \"${title}\",
      \"tags\": [\"smoke\"],
      \"timezone\": \"browser\",
      \"schemaVersion\": 39,
      \"version\": 0,
      \"templating\": {
        \"list\": [
          {
            \"name\": \"datasource\",
            \"label\": \"Data source\",
            \"type\": \"datasource\",
            \"query\": \"prometheus\",
            \"current\": {
              \"text\": \"Smoke Prometheus\",
              \"value\": \"Smoke Prometheus\"
            },
            \"options\": []
          }
        ]
      },
      \"panels\": [
        {
          \"id\": 1,
          \"title\": \"Smoke Panel\",
          \"type\": \"timeseries\",
          \"datasource\": \"\$datasource\",
          \"targets\": [
            {
              \"refId\": \"A\",
              \"expr\": \"vector(1)\"
            }
          ],
          \"gridPos\": {\"h\": 8, \"w\": 12, \"x\": 0, \"y\": 0}
        }
      ]
    },
    \"folderUid\": \"\",
    \"overwrite\": true,
    \"message\": \"smoke test seed\"
  }" >/dev/null
}

create_org() {
  local name="$1"
  api POST "/api/orgs" "{
    \"name\": \"${name}\"
  }" | json_field orgId
}

delete_org() {
  local org_id="$1"
  api DELETE "/api/orgs/${org_id}" >/dev/null
}

find_org_id_by_name() {
  local name="$1"
  api GET "/api/orgs" | jq -r --arg name "${name}" '.[] | select(.name == $name) | .id' | tail -n 1
}

seed_contact_point() {
  api POST "/api/v1/provisioning/contact-points" '{
    "uid": "smoke-webhook",
    "name": "Smoke Webhook",
    "type": "webhook",
    "settings": {
      "url": "http://127.0.0.1/notify"
    }
  }' >/dev/null
}

dashboard_bin() {
  printf '%s\n' "${RUST_DIR}/target/debug/grafana-util"
}

alert_bin() {
  printf '%s\n' "${RUST_DIR}/target/debug/grafana-util"
}

run_dashboard_smoke() {
  local diff_log="${WORK_DIR}/dashboard-diff.log"
  local dry_run_log="${WORK_DIR}/dashboard-import-dry-run.log"
  local routed_dry_run_log="${WORK_DIR}/dashboard-routed-import-dry-run.log"
  local recreate_dry_run_log="${WORK_DIR}/dashboard-routed-recreate-dry-run.log"
  local multi_org_org_two_id=""
  local recreated_org_id=""
  local prompt_file

  "$(dashboard_bin)" export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DASHBOARD_EXPORT_DIR}" \
    --overwrite

  [[ -f "${DASHBOARD_EXPORT_DIR}/raw/index.json" ]] || fail "dashboard raw index was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/raw/export-metadata.json" ]] || fail "dashboard raw metadata was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/prompt/index.json" ]] || fail "dashboard prompt index was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/prompt/export-metadata.json" ]] || fail "dashboard prompt metadata was not written"

  prompt_file="$(find "${DASHBOARD_EXPORT_DIR}/prompt" -type f -name '*.json' ! -name 'index.json' ! -name 'export-metadata.json' | head -n 1)"
  [[ -n "${prompt_file}" ]] || fail "dashboard prompt export did not produce a dashboard file"
  grep -q '"__inputs"' "${prompt_file}" || fail "dashboard prompt export did not include __inputs"
  grep -q 'DS_PROMETHEUS_' "${prompt_file}" || fail "dashboard prompt export did not rewrite datasource inputs"

  "$(dashboard_bin)" diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw"

  "$(dashboard_bin)" export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DASHBOARD_DRY_RUN_DIR}" \
    --overwrite \
    --dry-run

  [[ ! -e "${DASHBOARD_DRY_RUN_DIR}" ]] || fail "dashboard dry-run export created output files"

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run | tee "${dry_run_log}" >/dev/null
  grep -q 'Dry-run checked 1 dashboard(s)' "${dry_run_log}" || fail "dashboard dry-run import summary was not printed"

  seed_dashboard "Smoke Dashboard Drifted"
  if "$(dashboard_bin)" diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" >"${diff_log}" 2>&1; then
    fail "dashboard diff should have failed after live drift"
  fi
  grep -q 'Dashboard diff found 1 differing item(s).' "${diff_log}" || fail "dashboard diff drift summary was not printed"

  api DELETE "/api/dashboards/uid/smoke-dashboard" >/dev/null

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --replace-existing >/dev/null

  api GET "/api/dashboards/uid/smoke-dashboard" | grep -q '"uid":"smoke-dashboard"' \
    || fail "dashboard import did not recreate the exported dashboard"

  multi_org_org_two_id="$(create_org "Org Two")"
  [[ -n "${multi_org_org_two_id}" ]] || fail "failed to create Org Two for routed import smoke"
  seed_dashboard "Org Two Smoke Dashboard" "org-two-smoke-dashboard" "${multi_org_org_two_id}"

  "$(dashboard_bin)" export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${MULTI_ORG_EXPORT_DIR}" \
    --overwrite \
    --all-orgs \
    --without-dashboard-prompt >/dev/null

  [[ -d "${MULTI_ORG_EXPORT_DIR}/org_1_Main_Org/raw" ]] || fail "multi-org export did not include org 1 raw export"
  [[ -d "${MULTI_ORG_EXPORT_DIR}/org_${multi_org_org_two_id}_Org_Two/raw" ]] || fail "multi-org export did not include org 2 raw export"

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --dry-run | tee "${routed_dry_run_log}" >/dev/null
  grep -q "orgAction=exists" "${routed_dry_run_log}" || fail "routed dashboard dry-run did not report an existing org"
  grep -q "org-two-smoke-dashboard" "${routed_dry_run_log}" || fail "routed dashboard dry-run did not preview the selected org dashboard"

  delete_org "${multi_org_org_two_id}"

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --create-missing-orgs \
    --dry-run | tee "${recreate_dry_run_log}" >/dev/null
  grep -q "orgAction=would-create" "${recreate_dry_run_log}" || fail "routed dashboard dry-run did not preview missing-org creation"

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --create-missing-orgs >/dev/null

  recreated_org_id="$(find_org_id_by_name "Org Two")"
  [[ -n "${recreated_org_id}" ]] || fail "routed dashboard import did not recreate Org Two"
  api_org "${recreated_org_id}" GET "/api/dashboards/uid/org-two-smoke-dashboard" | grep -q '"uid":"org-two-smoke-dashboard"' \
    || fail "routed dashboard import did not restore the org-two dashboard"
}

run_alert_smoke() {
  local diff_log="${WORK_DIR}/alert-diff.log"
  local dry_run_log="${WORK_DIR}/alert-import-dry-run.log"
  local contact_file

  "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-dir "${ALERT_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${ALERT_EXPORT_DIR}/index.json" ]] || fail "alert export root index was not written"

  contact_file="$(find "${ALERT_EXPORT_DIR}/raw/contact-points" -type f -name '*Smoke_Webhook*.json' | head -n 1)"
  [[ -n "${contact_file}" ]] || fail "alert export did not write the seeded contact point"

  "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >/dev/null

  rewrite_contact_point_url "${contact_file}" "http://127.0.0.1/updated"

  if "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_log}" 2>&1; then
    fail "alert diff should have failed after local drift"
  fi
  grep -q 'Diff different' "${diff_log}" || fail "alert diff did not report a changed resource"

  "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run | tee "${dry_run_log}" >/dev/null
  grep -q 'action=would-update' "${dry_run_log}" || fail "alert dry-run import did not predict an update"

  "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing >/dev/null

  "$(alert_bin)" \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >/dev/null
}

main() {
  command -v docker >/dev/null || fail "docker is required"
  command -v curl >/dev/null || fail "curl is required"
  command -v jq >/dev/null || fail "jq is required"

  build_rust_bins
  start_grafana
  seed_datasource
  seed_dashboard "Smoke Dashboard"
  seed_contact_point
  create_api_token
  run_dashboard_smoke
  run_alert_smoke
  printf 'Rust live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
}

main "$@"
