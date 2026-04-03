#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${PYTHON_BIN:-python3}"
GRAFANA_IMAGE="${GRAFANA_IMAGE:-grafana/grafana:12.4.1}"
GRAFANA_PORT="${GRAFANA_PORT:-43011}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-admin}"
GRAFANA_API_TOKEN="${GRAFANA_API_TOKEN:-}"
GRAFANA_URL=""
CONTAINER_NAME="${GRAFANA_CONTAINER_NAME:-grafana-util-live-grafana}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/grafana-util-python-datasource-live.XXXXXX")"
DATASOURCE_EXPORT_DIR="${WORK_DIR}/datasources"
DATASOURCE_MULTI_ORG_EXPORT_DIR="${WORK_DIR}/datasources-all-orgs"

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

start_grafana() {
  local publish_args=()

  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
  publish_args=(-p "127.0.0.1:${GRAFANA_PORT}:3000")

  docker run -d \
    --name "${CONTAINER_NAME}" \
    "${publish_args[@]}" \
    -e "GF_SECURITY_ADMIN_USER=${GRAFANA_USER}" \
    -e "GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD}" \
    -e "GF_USERS_ALLOW_SIGN_UP=false" \
    "${GRAFANA_IMAGE}" >/dev/null

  GRAFANA_URL="http://127.0.0.1:${GRAFANA_PORT}"
  wait_for_grafana
}

create_api_token() {
  local response=""
  local service_account_id=""

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  if response="$(api POST "/api/auth/keys" '{
    "name": "grafana-util-python-datasource-live",
    "role": "Admin",
    "secondsToLive": 3600
  }' 2>/dev/null)"; then
    GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  fi

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  response="$(api POST "/api/serviceaccounts" '{
    "name": "grafana-util-python-datasource-live",
    "role": "Admin",
    "isDisabled": false
  }')"
  service_account_id="$(printf '%s' "${response}" | json_field id)"
  [[ -n "${service_account_id}" ]] || fail "failed to create Grafana service account for token auth"

  response="$(api POST "/api/serviceaccounts/${service_account_id}/tokens" '{
    "name": "grafana-util-python-datasource-live",
    "secondsToLive": 3600
  }')"
  GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  [[ -n "${GRAFANA_API_TOKEN}" ]] || fail "failed to create Grafana API token"
}

datasource_cli() {
  PYTHONPATH="${ROOT_DIR}/python${PYTHONPATH:+:${PYTHONPATH}}" \
    "${PYTHON_BIN}" -m grafana_utils datasource "$@"
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

run_datasource_smoke() {
  local add_dry_run_log="${WORK_DIR}/python-datasource-add-dry-run.json"
  local delete_dry_run_log="${WORK_DIR}/python-datasource-delete-dry-run.json"
  local import_dry_run_log="${WORK_DIR}/python-datasource-import-dry-run.json"
  local routed_dry_run_log="${WORK_DIR}/python-datasource-routed-import-dry-run.json"
  local recreate_dry_run_log="${WORK_DIR}/python-datasource-routed-recreate-dry-run.json"
  local secret_uid="py-smoke-prometheus-secret"
  local secret_after_add=""
  local secret_after_modify=""
  local org_two_id=""
  local recreated_org_id=""

  datasource_cli add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid py-smoke-prometheus-extra \
    --name "Py Smoke Prometheus Extra" \
    --type prometheus \
    --access proxy \
    --datasource-url "http://prometheus-extra.invalid" \
    --dry-run \
    --json | tee "${add_dry_run_log}" >/dev/null

  jq -e '.summary.createCount == 1' "${add_dry_run_log}" >/dev/null \
    || fail "python datasource add dry-run did not predict one create"

  datasource_cli add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid py-smoke-prometheus-extra \
    --name "Py Smoke Prometheus Extra" \
    --type prometheus \
    --access proxy \
    --datasource-url "http://prometheus-extra.invalid" >/dev/null

  api GET "/api/datasources" | jq -e '.[] | select(.uid == "py-smoke-prometheus-extra")' >/dev/null \
    || fail "python datasource add did not create the py-smoke-prometheus-extra datasource"

  datasource_cli add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid "${secret_uid}" \
    --name "Py Smoke Prometheus Secret" \
    --type prometheus \
    --datasource-url "http://prometheus-secret.invalid" \
    --apply-supported-defaults \
    --basic-auth \
    --basic-auth-user "metrics-user" \
    --basic-auth-password "metrics-pass" \
    --http-header "X-Scope-OrgID=tenant-a" >/dev/null

  secret_after_add="$(api GET "/api/datasources/uid/${secret_uid}")"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.basicAuthUser')" == "metrics-user" ]] \
    || fail "python datasource secret add did not persist basicAuthUser"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.jsonData.httpMethod')" == "POST" ]] \
    || fail "python datasource secret add did not keep prometheus preset httpMethod"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.jsonData.httpHeaderName1')" == "X-Scope-OrgID" ]] \
    || fail "python datasource secret add did not persist httpHeaderName1"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.secureJsonFields.basicAuthPassword')" == "true" ]] \
    || fail "python datasource secret add did not mark basicAuthPassword as server-managed secret"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.secureJsonFields.httpHeaderValue1')" == "true" ]] \
    || fail "python datasource secret add did not mark httpHeaderValue1 as server-managed secret"

  datasource_cli modify \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid "${secret_uid}" \
    --basic-auth-password "override-pass" >/dev/null

  secret_after_modify="$(api GET "/api/datasources/uid/${secret_uid}")"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.basicAuthUser')" == "metrics-user" ]] \
    || fail "python datasource secret modify did not preserve basicAuthUser"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.jsonData.httpHeaderName1')" == "X-Scope-OrgID" ]] \
    || fail "python datasource secret modify did not preserve httpHeaderName1"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.secureJsonFields.basicAuthPassword')" == "true" ]] \
    || fail "python datasource secret modify did not keep basicAuthPassword server-managed secret flag"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.secureJsonFields.httpHeaderValue1')" == "true" ]] \
    || fail "python datasource secret modify unexpectedly cleared the existing httpHeaderValue1 secret flag"

  datasource_cli delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid py-smoke-prometheus-extra \
    --dry-run \
    --json | tee "${delete_dry_run_log}" >/dev/null

  jq -e '.summary.deleteCount == 1' "${delete_dry_run_log}" >/dev/null \
    || fail "python datasource delete dry-run did not predict one delete"

  datasource_cli delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid py-smoke-prometheus-extra >/dev/null

  datasource_cli delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid "${secret_uid}" >/dev/null

  if api GET "/api/datasources" | jq -e '.[] | select(.uid == "py-smoke-prometheus-extra")' >/dev/null; then
    fail "python datasource delete did not remove the py-smoke-prometheus-extra datasource"
  fi
  if api GET "/api/datasources" | jq -e --arg uid "${secret_uid}" '.[] | select(.uid == $uid)' >/dev/null; then
    fail "python datasource delete did not remove the ${secret_uid} datasource"
  fi

  datasource_cli export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DATASOURCE_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${DATASOURCE_EXPORT_DIR}/datasources.json" ]] || fail "python datasource export did not write datasources.json"
  [[ -f "${DATASOURCE_EXPORT_DIR}/index.json" ]] || fail "python datasource export did not write index.json"
  [[ -f "${DATASOURCE_EXPORT_DIR}/export-metadata.json" ]] || fail "python datasource export did not write export-metadata.json"

  datasource_cli import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DATASOURCE_EXPORT_DIR}" \
    --replace-existing \
    --dry-run \
    --json | tee "${import_dry_run_log}" >/dev/null

  jq -e '.summary.updateCount >= 1' "${import_dry_run_log}" >/dev/null \
    || fail "python datasource import dry-run did not predict an update"

  org_two_id="$(create_org "Python Datasource Org Two")"
  [[ -n "${org_two_id}" ]] || fail "failed to create python datasource org two"
  seed_datasource "${org_two_id}" "Py Org Two Prometheus" "py-org-two-prometheus"

  datasource_cli export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --overwrite \
    --all-orgs >/dev/null

  [[ -f "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/index.json" ]] || fail "python datasource multi-org export did not write root index"
  [[ -f "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/export-metadata.json" ]] || fail "python datasource multi-org export did not write root metadata"
  [[ -d "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/org_1_Main_Org" ]] || fail "python datasource multi-org export did not include main org bundle"
  [[ -d "${DATASOURCE_MULTI_ORG_EXPORT_DIR}/org_${org_two_id}_Python_Datasource_Org_Two" ]] || fail "python datasource multi-org export did not include org two bundle"

  datasource_cli import \
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
    || fail "python datasource routed dry-run did not report an existing org"
  jq -e '.imports | any(.datasources[]?.uid == "py-org-two-prometheus")' "${routed_dry_run_log}" >/dev/null \
    || fail "python datasource routed dry-run did not preview the selected org datasource"

  delete_org "${org_two_id}"

  datasource_cli import \
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

  jq -e '.orgs | any(.orgAction == "would-create-org")' "${recreate_dry_run_log}" >/dev/null \
    || fail "python datasource routed dry-run did not preview missing-org creation"

  datasource_cli import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${org_two_id}" \
    --replace-existing \
    --create-missing-orgs >/dev/null

  recreated_org_id="$(find_org_id_by_name "Python Datasource Org Two")"
  [[ -n "${recreated_org_id}" ]] || fail "python datasource routed import did not recreate org two"
  api_org "${recreated_org_id}" GET "/api/datasources" | jq -e '.[] | select(.uid == "py-org-two-prometheus")' >/dev/null \
    || fail "python datasource routed import did not restore the org-two datasource"
}

main() {
  trap cleanup EXIT

  command -v docker >/dev/null || fail "docker is required"
  command -v curl >/dev/null || fail "curl is required"
  command -v jq >/dev/null || fail "jq is required"
  command -v "${PYTHON_BIN}" >/dev/null || fail "${PYTHON_BIN} is required"

  start_grafana
  create_api_token
  seed_datasource
  run_datasource_smoke
  printf 'Python datasource live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
}

main "$@"
