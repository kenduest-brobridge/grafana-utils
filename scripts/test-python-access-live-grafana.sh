#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="${PYTHON_BIN:-python3}"
GRAFANA_IMAGE="${GRAFANA_IMAGE:-grafana/grafana:12.4.1}"
GRAFANA_PORT="${GRAFANA_PORT:-}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-admin}"
GRAFANA_API_TOKEN="${GRAFANA_API_TOKEN:-}"
GRAFANA_URL=""
CONTAINER_NAME="${GRAFANA_CONTAINER_NAME:-grafana-util-python-access-live-$$}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/grafana-util-python-access-live.XXXXXX")"
SERVICE_ACCOUNT_EXPORT_DIR="${WORK_DIR}/access-service-accounts"

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

create_api_token() {
  local response=""
  local service_account_id=""

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  if response="$(api POST "/api/auth/keys" '{
    "name": "grafana-util-python-access-live",
    "role": "Admin",
    "secondsToLive": 3600
  }' 2>/dev/null)"; then
    GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  fi

  if [[ -n "${GRAFANA_API_TOKEN}" ]]; then
    return
  fi

  response="$(api POST "/api/serviceaccounts" '{
    "name": "grafana-util-python-access-live",
    "role": "Admin",
    "isDisabled": false
  }')"
  service_account_id="$(printf '%s' "${response}" | json_field id)"
  [[ -n "${service_account_id}" ]] || fail "failed to create Grafana service account for token auth"

  response="$(api POST "/api/serviceaccounts/${service_account_id}/tokens" '{
    "name": "grafana-util-python-access-live",
    "secondsToLive": 3600
  }')"
  GRAFANA_API_TOKEN="$(printf '%s' "${response}" | json_field key)"
  [[ -n "${GRAFANA_API_TOKEN}" ]] || fail "failed to create Grafana API token"
}

access_cli() {
  "${PYTHON_BIN}" -m grafana_utils access "$@"
}

run_user_smoke() {
  local list_json org_json modify_json

  access_cli user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login access-modify \
    --email access-modify@example.com \
    --name "Access Modify" \
    --password secret123 \
    --org-role Editor \
    --grafana-admin true >/dev/null

  list_json="$(
    access_cli user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope global \
      --login access-modify \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq -r '.[0].grafanaAdmin')" == "true" ]] \
    || fail "global user list did not show grafanaAdmin=true for created user"

  org_json="$(
    access_cli user list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --scope org \
      --login access-modify \
      --json
  )"
  [[ "$(printf '%s' "${org_json}" | jq -r '.[0].orgRole')" == "Editor" ]] \
    || fail "org user list did not show orgRole=Editor for created user"

  modify_json="$(
    access_cli user modify \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --login access-modify \
      --set-login access-modified \
      --set-email access-modified@example.com \
      --set-name "Access Modified" \
      --set-password secret456 \
      --set-org-role Admin \
      --set-grafana-admin false \
      --json
  )"
  [[ "$(printf '%s' "${modify_json}" | jq -r '.login')" == "access-modified" ]] \
    || fail "user modify did not return updated login"

  access_cli user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login access-org-delete \
    --email access-org-delete@example.com \
    --name "Access Org Delete" \
    --password secret123 >/dev/null

  access_cli user delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --token "${GRAFANA_API_TOKEN}" \
    --scope org \
    --login access-org-delete \
    --yes >/dev/null

  org_json="$(
    access_cli user list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --scope org \
      --login access-org-delete \
      --json
  )"
  [[ "$(printf '%s' "${org_json}" | jq 'length')" == "0" ]] \
    || fail "org-scoped delete did not remove the target user"

  access_cli user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login access-global-delete \
    --email access-global-delete@example.com \
    --name "Access Global Delete" \
    --password secret123 >/dev/null

  access_cli user delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --login access-global-delete \
    --yes >/dev/null

  list_json="$(
    access_cli user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope global \
      --login access-global-delete \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq 'length')" == "0" ]] \
    || fail "global delete did not remove the target user"
}

run_team_smoke() {
  local team_json delete_json

  access_cli user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login access-team-member \
    --email access-team-member@example.com \
    --name "Access Team Member" \
    --password secret123 >/dev/null

  access_cli user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login access-team-admin \
    --email access-team-admin@example.com \
    --name "Access Team Admin" \
    --password secret123 >/dev/null

  access_cli team add \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --name access-ops \
    --email access-ops@example.com \
    --member access-team-member \
    --admin access-team-admin@example.com >/dev/null

  team_json="$(
    access_cli team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name access-ops \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].name')" == "access-ops" ]] \
    || fail "team list did not return the created team"

  access_cli team modify \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name access-ops \
    --remove-member access-team-member \
    --remove-admin access-team-admin@example.com \
    --remove-member access-team-admin@example.com >/dev/null

  team_json="$(
    access_cli team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name access-ops \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].members | length')" == "0" ]] \
    || fail "team modify did not remove seeded members/admins"

  delete_json="$(
    access_cli team delete \
      --url "${GRAFANA_URL}" \
      --insecure \
      --token "${GRAFANA_API_TOKEN}" \
      --name access-ops \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${delete_json}" | jq -r '.name')" == "access-ops" ]] \
    || fail "team delete did not remove the created team"

  team_json="$(
    access_cli team list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --name access-ops \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq 'length')" == "0" ]] \
    || fail "team delete did not remove the target team from list output"
}

run_org_smoke() {
  local org_json list_json

  org_json="$(
    access_cli org add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-live-delete-target \
      --json
  )"
  [[ "$(printf '%s' "${org_json}" | jq -r '.name')" == "access-live-delete-target" ]] \
    || fail "org add did not create the live delete target"

  access_cli org delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name access-live-delete-target \
    --yes >/dev/null

  list_json="$(
    access_cli org list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-live-delete-target \
      --json
  )"
  [[ "$(printf '%s' "${list_json}" | jq 'length')" == "0" ]] \
    || fail "org delete did not remove the target organization"
}

run_service_account_smoke() {
  local service_account_json token_json diff_log delete_json token_delete_json

  service_account_json="$(
    access_cli service-account add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-cli-service-account \
      --role Admin \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.name')" == "access-cli-service-account" ]] \
    || fail "service-account add did not return the created item"

  token_json="$(
    access_cli service-account token add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-cli-service-account \
      --token-name access-cli-token \
      --seconds-to-live 3600 \
      --json
  )"
  [[ -n "$(printf '%s' "${token_json}" | jq -r '.key')" ]] \
    || fail "service-account token add did not return a token key"

  service_account_json="$(
    access_cli service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[] | select(.name=="access-cli-service-account") | .role')" == "Admin" ]] \
    || fail "service-account list did not show the created service account"

  access_cli service-account export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" \
    --overwrite >/dev/null

  [[ -f "${SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json" ]] \
    || fail "service-account export did not write service-accounts.json"
  [[ -f "${SERVICE_ACCOUNT_EXPORT_DIR}/export-metadata.json" ]] \
    || fail "service-account export did not write export-metadata.json"

  token_delete_json="$(
    access_cli service-account token delete \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-cli-service-account \
      --token-name access-cli-token \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${token_delete_json}" | jq -r '.tokenName')" == "access-cli-token" ]] \
    || fail "service-account token delete did not remove the created token"

  delete_json="$(
    access_cli service-account delete \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name access-cli-service-account \
      --yes \
      --json
  )"
  [[ "$(printf '%s' "${delete_json}" | jq -r '.name')" == "access-cli-service-account" ]] \
    || fail "service-account delete did not remove the created service account"

  service_account_json="$(
    access_cli service-account import \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --import-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" \
      --replace-existing \
      --dry-run \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.summary.created')" == "1" ]] \
    || fail "service-account dry-run import did not predict one create"

  access_cli service-account import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" \
    --replace-existing >/dev/null

  service_account_json="$(
    access_cli service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[] | select(.name=="access-cli-service-account") | .role')" == "Admin" ]] \
    || fail "service-account import did not recreate the exported service account"

  jq '
    .records = (
      .records
      | map(
          if .name == "access-cli-service-account"
          then .role = "Viewer"
          else .
          end
        )
    )
  ' "${SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json" > "${SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json.tmp" \
    || fail "failed to rewrite service-account export role for diff/import smoke"
  mv "${SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json.tmp" "${SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json"

  diff_log="${WORK_DIR}/service-account-diff.log"
  if access_cli service-account diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" >"${diff_log}" 2>&1; then
    fail "service-account diff should have detected the rewritten export role"
  fi
  grep -q 'Diff different service-account access-cli-service-account fields=role' "${diff_log}" \
    || fail "service-account diff did not report the changed role"

  service_account_json="$(
    access_cli service-account import \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --import-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" \
      --replace-existing \
      --dry-run \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.summary.updated')" == "1" ]] \
    || fail "service-account dry-run import did not predict one update"

  access_cli service-account import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" \
    --replace-existing >/dev/null

  service_account_json="$(
    access_cli service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[] | select(.name=="access-cli-service-account") | .role')" == "Viewer" ]] \
    || fail "service-account import did not apply the rewritten role"

  access_cli service-account diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${SERVICE_ACCOUNT_EXPORT_DIR}" >/dev/null
}

main() {
  trap cleanup EXIT

  command -v docker >/dev/null || fail "docker is required"
  command -v curl >/dev/null || fail "curl is required"
  command -v jq >/dev/null || fail "jq is required"
  command -v "${PYTHON_BIN}" >/dev/null || fail "${PYTHON_BIN} is required"

  start_grafana
  create_api_token
  run_user_smoke
  run_team_smoke
  run_org_smoke
  run_service_account_smoke
  printf 'Python access live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
}

main "$@"
