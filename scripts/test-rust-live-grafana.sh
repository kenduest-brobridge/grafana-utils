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
DATASOURCE_EXPORT_DIR="${WORK_DIR}/datasources"
DATASOURCE_MULTI_ORG_EXPORT_DIR="${WORK_DIR}/datasources-all-orgs"
ALERT_EXPORT_DIR="${WORK_DIR}/alerts"
MULTI_ORG_EXPORT_DIR="${WORK_DIR}/dashboards-all-orgs"
ACCESS_SERVICE_ACCOUNT_EXPORT_DIR="${WORK_DIR}/access-service-accounts"
SYNC_BUNDLE_FILE="${WORK_DIR}/sync-source-bundle.json"
SYNC_TARGET_INVENTORY_FILE="${WORK_DIR}/sync-target-inventory.json"
SYNC_BUNDLE_PREFLIGHT_FILE="${WORK_DIR}/sync-bundle-preflight.json"

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

check_requested_grafana_port() {
  if [[ -z "${GRAFANA_PORT}" ]]; then
    return
  fi

  if command -v lsof >/dev/null 2>&1; then
    if lsof -nP -iTCP:"${GRAFANA_PORT}" -sTCP:LISTEN >/dev/null 2>&1; then
      fail "GRAFANA_PORT ${GRAFANA_PORT} is already in use by another listening service"
    fi
    return
  fi

  if command -v ss >/dev/null 2>&1; then
    if ss -ltn "sport = :${GRAFANA_PORT}" | tail -n +2 | grep -q .; then
      fail "GRAFANA_PORT ${GRAFANA_PORT} is already in use by another listening service"
    fi
    return
  fi

  if command -v netstat >/dev/null 2>&1; then
    if netstat -an | grep -E "[\\.:]${GRAFANA_PORT}[[:space:]].*LISTEN" >/dev/null 2>&1; then
      fail "GRAFANA_PORT ${GRAFANA_PORT} is already in use by another listening service"
    fi
  fi
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
    check_requested_grafana_port
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
  local org_id="${1:-}"
  local name="${2:-Smoke Prometheus}"
  local uid="${3:-smoke-prometheus}"
  local api_runner="api"
  if [[ -n "${org_id}" ]]; then
    api_runner="api_org ${org_id}"
  fi
  ${api_runner} POST "/api/datasources" "{
    \"uid\": \"${uid}\",
    \"name\": \"${name}\",
    \"type\": \"prometheus\",
    \"access\": \"proxy\",
    \"url\": \"http://prometheus.invalid\",
    \"isDefault\": true
  }" >/dev/null
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

datasource_bin() {
  printf '%s\n' "${RUST_DIR}/target/debug/grafana-util"
}

access_bin() {
  printf '%s\n' "${RUST_DIR}/target/debug/grafana-util"
}

sync_bin() {
  printf '%s\n' "${RUST_DIR}/target/debug/grafana-util"
}

run_access_smoke() {
  local list_json team_json org_json service_account_json token_json token_delete_json delete_json

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login rust-access-org-delete \
    --email rust-access-org-delete@example.com \
    --name "Rust Access Org Delete" \
    --password secret123 >/dev/null

  "$(access_bin)" access user delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --token "${GRAFANA_API_TOKEN}" \
    --scope org \
    --login rust-access-org-delete \
    --yes >/dev/null

  list_json="$(
    "$(access_bin)" access user list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --scope org \
      --login rust-access-org-delete \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq 'length')" == "0" ]] \
    || fail "rust access org-scoped delete did not remove the target user"

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login rust-access-global-delete \
    --email rust-access-global-delete@example.com \
    --name "Rust Access Global Delete" \
    --password secret123 >/dev/null

  "$(access_bin)" access user delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --login rust-access-global-delete \
    --yes >/dev/null

  list_json="$(
    "$(access_bin)" access user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope global \
      --login rust-access-global-delete \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq 'length')" == "0" ]] \
    || fail "rust access global delete did not remove the target user"

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login rust-access-team-member \
    --email rust-access-team-member@example.com \
    --name "Rust Access Team Member" \
    --password secret123 >/dev/null

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login rust-access-team-admin \
    --email rust-access-team-admin@example.com \
    --name "Rust Access Team Admin" \
    --password secret123 >/dev/null

  "$(access_bin)" access team add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --name rust-access-ops \
    --email rust-access-ops@example.com \
    --member rust-access-team-member \
    --admin rust-access-team-admin@example.com >/dev/null

  team_json="$(
    "$(access_bin)" access team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name rust-access-ops \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].name')" == "rust-access-ops" ]] \
    || fail "rust access team list did not return the created team"

  "$(access_bin)" access team modify \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-ops \
    --remove-member rust-access-team-member \
    --remove-admin rust-access-team-admin@example.com \
    --remove-member rust-access-team-admin@example.com >/dev/null

  team_json="$(
    "$(access_bin)" access team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name rust-access-ops \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].members | length')" == "0" ]] \
    || fail "rust access team modify did not remove seeded members/admins"

  delete_json="$(
    "$(access_bin)" access team delete \
      --url "${GRAFANA_URL}" \
      --insecure \
      --token "${GRAFANA_API_TOKEN}" \
      --name rust-access-ops \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${delete_json}" | jq -r '.name')" == "rust-access-ops" ]] \
    || fail "rust access team delete did not remove the created team"

  team_json="$(
    "$(access_bin)" access team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name rust-access-ops \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq 'length')" == "0" ]] \
    || fail "rust access team delete did not remove the target team from list output"

  org_json="$(
    "$(access_bin)" access org add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-live-delete-target \
      --json
  )"
  [[ "$(printf '%s' "${org_json}" | jq -r '.name')" == "rust-access-live-delete-target" ]] \
    || fail "rust access org add did not create the delete target"

  "$(access_bin)" access org delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-live-delete-target \
    --yes >/dev/null

  list_json="$(
    "$(access_bin)" access org list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-live-delete-target \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq 'length')" == "0" ]] \
    || fail "rust access org delete did not remove the target organization"

  service_account_json="$(
    "$(access_bin)" access service-account add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-service-account \
      --role Admin \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.name')" == "rust-access-service-account" ]] \
    || fail "rust access service-account add did not return the created item"

  token_json="$(
    "$(access_bin)" access service-account token add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-service-account \
      --token-name rust-access-token \
      --seconds-to-live 3600 \
      --json
  )"
  [[ -n "$(printf '%s' "${token_json}" | jq -r '.key')" ]] \
    || fail "rust access service-account token add did not return a token key"

  "$(access_bin)" access service-account export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json" ]] \
    || fail "rust access service-account export did not write service-accounts.json"
  [[ -f "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}/export-metadata.json" ]] \
    || fail "rust access service-account export did not write export-metadata.json"

  token_delete_json="$(
    "$(access_bin)" access service-account token delete \
      --url "${GRAFANA_URL}" \
      --insecure \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-service-account \
      --token-name rust-access-token \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${token_delete_json}" | jq -r '.tokenName')" == "rust-access-token" ]] \
    || fail "rust access service-account token delete did not remove the created token"

  delete_json="$(
    "$(access_bin)" access service-account delete \
      --url "${GRAFANA_URL}" \
      --insecure \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-service-account \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${delete_json}" | jq -r '.name')" == "rust-access-service-account" ]] \
    || fail "rust access service-account delete did not remove the created service account"

  service_account_json="$(
    "$(access_bin)" access service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq '[.[] | select(.name == "rust-access-service-account")] | length')" == "0" ]] \
    || fail "rust access service-account delete did not remove the target service account"
}

run_datasource_smoke() {
  local add_dry_run_log="${WORK_DIR}/datasource-add-dry-run.json"
  local delete_dry_run_log="${WORK_DIR}/datasource-delete-dry-run.json"
  local dry_run_log="${WORK_DIR}/datasource-import-dry-run.json"
  local routed_dry_run_log="${WORK_DIR}/datasource-routed-import-dry-run.json"
  local recreate_dry_run_log="${WORK_DIR}/datasource-routed-recreate-dry-run.json"
  local org_two_id=""
  local recreated_org_id=""

  "$(datasource_bin)" datasource add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid smoke-prometheus-extra \
    --name "Smoke Prometheus Extra" \
    --type prometheus \
    --access proxy \
    --datasource-url "http://prometheus-extra.invalid" \
    --dry-run \
    --json | tee "${add_dry_run_log}" >/dev/null

  jq -e '.summary.createCount == 1' "${add_dry_run_log}" >/dev/null \
    || fail "datasource add dry-run did not predict one create"

  "$(datasource_bin)" datasource add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid smoke-prometheus-extra \
    --name "Smoke Prometheus Extra" \
    --type prometheus \
    --access proxy \
    --datasource-url "http://prometheus-extra.invalid" >/dev/null

  api GET "/api/datasources" | jq -e '.[] | select(.uid == "smoke-prometheus-extra")' >/dev/null \
    || fail "datasource add did not create the smoke-prometheus-extra datasource"

  "$(datasource_bin)" datasource delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid smoke-prometheus-extra \
    --dry-run \
    --json | tee "${delete_dry_run_log}" >/dev/null

  jq -e '.summary.deleteCount == 1' "${delete_dry_run_log}" >/dev/null \
    || fail "datasource delete dry-run did not predict one delete"

  "$(datasource_bin)" datasource delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid smoke-prometheus-extra >/dev/null

  if api GET "/api/datasources" | jq -e '.[] | select(.uid == "smoke-prometheus-extra")' >/dev/null; then
    fail "datasource delete did not remove the smoke-prometheus-extra datasource"
  fi

  "$(datasource_bin)" datasource export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DATASOURCE_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${DATASOURCE_EXPORT_DIR}/datasources.json" ]] || fail "datasource export did not write datasources.json"
  [[ -f "${DATASOURCE_EXPORT_DIR}/index.json" ]] || fail "datasource export did not write index.json"
  [[ -f "${DATASOURCE_EXPORT_DIR}/export-metadata.json" ]] || fail "datasource export did not write export-metadata.json"

  "$(datasource_bin)" datasource import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DATASOURCE_EXPORT_DIR}" \
    --replace-existing \
    --dry-run \
    --json | tee "${dry_run_log}" >/dev/null

  jq -e '.summary.wouldUpdate >= 1' "${dry_run_log}" >/dev/null \
    || fail "datasource dry-run import did not predict an update"

  org_two_id="$(create_org "Datasource Org Two")"
  [[ -n "${org_two_id}" ]] || fail "failed to create datasource org two"
  seed_datasource "${org_two_id}" "Org Two Prometheus" "org-two-prometheus"

  "$(datasource_bin)" datasource export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --overwrite \
    --all-orgs >/dev/null

  [[ -f "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/index.json" ]] || fail "datasource multi-org export did not write root index"
  [[ -f "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/export-metadata.json" ]] || fail "datasource multi-org export did not write root metadata"
  [[ -d "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/org_1_Main_Org" ]] || fail "datasource multi-org export did not include main org bundle"
  [[ -d "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/org_${org_two_id}_Datasource_Org_Two" ]] || fail "datasource multi-org export did not include org two bundle"

  "$(datasource_bin)" datasource import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${org_two_id}" \
    --replace-existing \
    --dry-run \
    --json | tee "${routed_dry_run_log}" >/dev/null

  jq -e '.orgs | any(.orgAction == "exists")' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not report an existing org"
  jq -e '.imports | any(.datasources[]?.uid == "org-two-prometheus")' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not preview the selected org datasource"

  delete_org "${org_two_id}"

  "$(datasource_bin)" datasource import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${org_two_id}" \
    --replace-existing \
    --create-missing-orgs \
    --dry-run \
    --json | tee "${recreate_dry_run_log}" >/dev/null

  jq -e '.orgs | any(.orgAction == "would-create")' "${recreate_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not preview missing-org creation"

  "$(datasource_bin)" datasource import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${org_two_id}" \
    --replace-existing \
    --create-missing-orgs >/dev/null

  recreated_org_id="$(find_org_id_by_name "Datasource Org Two")"
  [[ -n "${recreated_org_id}" ]] || fail "datasource routed import did not recreate org two"
  api_org "${recreated_org_id}" GET "/api/datasources" | jq -e '.[] | select(.uid == "org-two-prometheus")' >/dev/null \
    || fail "datasource routed import did not restore the org-two datasource"
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
  jq -e '.__inputs | length > 0' "${prompt_file}" >/dev/null \
    || fail "dashboard prompt export did not include __inputs"
  jq -e '.__inputs | map(.name) | any(startswith("DS_PROMETHEUS"))' "${prompt_file}" >/dev/null \
    || fail "dashboard prompt export did not rewrite datasource inputs"

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
    --dry-run \
    --json | tee "${routed_dry_run_log}" >/dev/null
  jq -e '.orgs | any(.orgAction == "exists")' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not report an existing org"
  jq -e '.imports | any(.dashboards[]?.uid == "org-two-smoke-dashboard")' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not preview the selected org dashboard"

  delete_org "${multi_org_org_two_id}"

  "$(dashboard_bin)" import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --create-missing-orgs \
    --dry-run \
    --json | tee "${recreate_dry_run_log}" >/dev/null
  jq -e '.orgs | any(.orgAction == "would-create")' "${recreate_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not preview missing-org creation"

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
    alert export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-dir "${ALERT_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${ALERT_EXPORT_DIR}/index.json" ]] || fail "alert export root index was not written"

  contact_file="$(find "${ALERT_EXPORT_DIR}/raw/contact-points" -type f -name '*Smoke_Webhook*.json' | head -n 1)"
  [[ -n "${contact_file}" ]] || fail "alert export did not write the seeded contact point"

  "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >/dev/null

  rewrite_contact_point_url "${contact_file}" "http://127.0.0.1/updated"

  if "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_log}" 2>&1; then
    fail "alert diff should have failed after local drift"
  fi
  grep -q 'Diff different' "${diff_log}" || fail "alert diff did not report a changed resource"

  "$(alert_bin)" \
    alert import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run | tee "${dry_run_log}" >/dev/null
  grep -q 'action=would-update' "${dry_run_log}" || fail "alert dry-run import did not predict an update"

  "$(alert_bin)" \
    alert import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing >/dev/null

  "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >/dev/null
}

run_sync_smoke() {
  "$(sync_bin)" sync bundle \
    --dashboard-export-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --alert-export-dir "${ALERT_EXPORT_DIR}/raw" \
    --output-file "${SYNC_BUNDLE_FILE}" \
    --output json >/dev/null

  [[ -f "${SYNC_BUNDLE_FILE}" ]] || fail "sync bundle did not write source bundle output"
  jq -e '.kind == "grafana-utils-sync-source-bundle"' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not emit the expected source bundle kind"
  jq -e '.summary.alertRuleCount >= 1' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not record exported alert rule count"
  jq -e '.alerts | length >= 1' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not normalize top-level alert specs"
  jq -e '.alerts[] | select(.kind == "alert" and .uid == "smoke-alert-rule" and (.managedFields | index("condition")) != null)' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not preserve normalized alert spec fields"

  printf '{}\n' >"${SYNC_TARGET_INVENTORY_FILE}"

  "$(sync_bin)" sync bundle-preflight \
    --source-bundle "${SYNC_BUNDLE_FILE}" \
    --target-inventory "${SYNC_TARGET_INVENTORY_FILE}" \
    --output json >"${SYNC_BUNDLE_PREFLIGHT_FILE}"

  jq -e '.kind == "grafana-utils-sync-bundle-preflight"' "${SYNC_BUNDLE_PREFLIGHT_FILE}" >/dev/null \
    || fail "sync bundle-preflight did not emit the expected document kind"
  jq -e '.summary.resourceCount >= 3' "${SYNC_BUNDLE_PREFLIGHT_FILE}" >/dev/null \
    || fail "sync bundle-preflight did not count bundled dashboard, datasource, and alert specs"
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
  run_access_smoke
  run_dashboard_smoke
  run_alert_smoke
  run_sync_smoke
  run_datasource_smoke
  printf 'Rust live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
}

main "$@"
