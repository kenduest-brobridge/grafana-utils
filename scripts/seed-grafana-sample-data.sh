#!/usr/bin/env bash
set -euo pipefail

GRAFANA_URL="${GRAFANA_URL:-http://localhost:3000}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-admin}"
DESTROY_MODE=false
RESET_ALL_DATA_MODE=false
VERIFY_MODE=false
CONFIRMED_RESET=false

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage: seed-grafana-sample-data.sh [OPTIONS]

Seed or destroy reusable developer sample data in a Grafana instance.

Options:
  --url URL           Grafana base URL (default: http://localhost:3000)
  --basic-user USER   Grafana admin username (default: admin)
  --basic-password PW Grafana admin password (default: admin)
  --verify            Verify that the expected sample data already exists
  --destroy           Delete the sample data created by this script
  --reset-all-data    Delete all repo-relevant developer test data from Grafana
  --yes               Required with --reset-all-data
  -h, --help          Show this help text

Environment overrides:
  GRAFANA_URL
  GRAFANA_USER
  GRAFANA_PASSWORD

The script is idempotent:
- reuses existing orgs, users, teams, service accounts, folders, and datasources by fixed uid or name
- upserts dashboards with overwrite=true
- `--destroy` removes only the known sample resources and extra sample orgs
- `--reset-all-data --yes` is a destructive developer reset for disposable Grafana instances

Seeded sample layout:
- Org 1 Main Org.
- Users: sample-platform-admin, sample-editor, sample-viewer, sample-auditor
- Teams: platform-ops, api-dev
- Service accounts: deploy-bot, report-bot
  - Datasources: Smoke Prometheus, Smoke Prometheus 2, Smoke Loki
  - Folders: Platform, Platform / Infra, Platform / Team / Apps / Prod, Platform / Team / Apps / Stage, Platform / Team / Apps / API
  - Dashboards: smoke-main, smoke-prom-only, query-smoke, mixed-query-smoke, two-prom-query-smoke, subfolder-main, subfolder-chain-smoke, variable-chain-smoke, api-mixed-overview, stage-rollup
- Org 2 Org Two
- Team: org-two-editors
- Service account: org-two-bot
  - Dashboard: org-two-main
- Org 3 QA Org
- Team: qa-team
- Service account: qa-bot
  - Dashboard: qa-overview
- Org 4 Audit Org
- Team: audit-reviewers
- Service account: audit-bot
  - Dashboard: audit-home

Reset-all-data scope:
- deletes all non-default orgs
- clears dashboards, folders, datasources, teams, service accounts, and alert rules in org 1
- deletes non-admin global users except the current login user
EOF
}

require_tool() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required tool: $1"
}

urlencode() {
  jq -rn --arg value "$1" '$value|@uri'
}

request_raw() {
  local method="$1"
  local path="$2"
  local payload="${3:-}"
  local org_id="${4:-}"
  local response
  local headers=(-u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" -X "${method}")

  if [[ -n "${org_id}" ]]; then
    headers+=(-H "X-Grafana-Org-Id: ${org_id}")
  fi
  if [[ -n "${payload}" ]]; then
    headers+=(-H 'Content-Type: application/json' --data-binary "${payload}")
  fi

  response="$(
    curl --silent --show-error \
      "${headers[@]}" \
      "${GRAFANA_URL}${path}" \
      -w $'\n%{http_code}'
  )"
  HTTP_STATUS="${response##*$'\n'}"
  HTTP_BODY="${response%$'\n'*}"
}

request_json() {
  request_raw "$@"
  if [[ "${HTTP_STATUS}" != 2* ]]; then
    fail "request failed: $1 $2 -> HTTP ${HTTP_STATUS} ${HTTP_BODY}"
  fi
}

request_optional() {
  request_raw "$@"
  if [[ "${HTTP_STATUS}" == "404" ]]; then
    return 1
  fi
  if [[ "${HTTP_STATUS}" != 2* ]]; then
    fail "request failed: $1 $2 -> HTTP ${HTTP_STATUS} ${HTTP_BODY}"
  fi
  return 0
}

current_admin_login() {
  request_json GET "/api/user"
  printf '%s' "${HTTP_BODY}" | jq -r '.login // empty'
}

ensure_health() {
  request_json GET "/api/health"
}

lookup_org_id_by_name() {
  request_json GET "/api/orgs"
  printf '%s' "${HTTP_BODY}" | jq -r --arg name "$1" '.[] | select(.name == $name) | .id' | head -n 1
}

list_org_ids() {
  request_json GET "/api/orgs"
  printf '%s' "${HTTP_BODY}" | jq -r '.[].id'
}

ensure_org() {
  local name="$1"
  local org_id

  org_id="$(lookup_org_id_by_name "${name}")"
  if [[ -n "${org_id}" ]]; then
    printf '%s\n' "${org_id}"
    return
  fi

  request_json POST "/api/orgs" "$(jq -cn --arg name "${name}" '{name: $name}')"
  org_id="$(printf '%s' "${HTTP_BODY}" | jq -r '.orgId // .id // empty')"
  [[ -n "${org_id}" ]] || fail "failed to create org ${name}"
  printf '%s\n' "${org_id}"
}

lookup_global_user_id_by_login() {
  local login="$1"
  request_json GET "/api/users?perpage=1000&page=1"
  printf '%s' "${HTTP_BODY}" | jq -r --arg login "${login}" '.[] | select(.login == $login) | .id' | head -n 1
}

lookup_global_user_email_by_login() {
  local login="$1"
  request_json GET "/api/users?perpage=1000&page=1"
  printf '%s' "${HTTP_BODY}" | jq -r --arg login "${login}" '.[] | select(.login == $login) | .email // empty' | head -n 1
}

resolve_identity_array_to_emails() {
  local identities_json="$1"
  local identity=""
  local resolved_email=""
  local resolved_json='[]'

  while IFS= read -r identity; do
    [[ -n "${identity}" ]] || continue
    if [[ "${identity}" == *"@"* ]]; then
      resolved_email="${identity}"
    else
      resolved_email="$(lookup_global_user_email_by_login "${identity}")"
    fi
    [[ -n "${resolved_email}" ]] || fail "failed to resolve user identity ${identity} to email"
    resolved_json="$(printf '%s' "${resolved_json}" | jq -c --arg value "${resolved_email}" '. + [$value]')"
  done < <(printf '%s' "${identities_json}" | jq -r '.[]')

  printf '%s\n' "${resolved_json}"
}

ensure_global_user() {
  local login="$1"
  local email="$2"
  local name="$3"
  local password="$4"
  local org_role="$5"
  local is_grafana_admin="$6"
  local user_id

  user_id="$(lookup_global_user_id_by_login "${login}")"
  if [[ -z "${user_id}" ]]; then
    request_json POST "/api/admin/users" "$(
      jq -cn \
        --arg login "${login}" \
        --arg email "${email}" \
        --arg name "${name}" \
        --arg password "${password}" \
        '{login: $login, email: $email, name: $name, password: $password}'
    )"
    user_id="$(printf '%s' "${HTTP_BODY}" | jq -r '.id // empty')"
    [[ -n "${user_id}" ]] || fail "failed to create global user ${login}"
    printf 'Created user %s\n' "${login}"
  else
    request_json PUT "/api/users/${user_id}" "$(
      jq -cn \
        --arg login "${login}" \
        --arg email "${email}" \
        --arg name "${name}" \
        '{login: $login, email: $email, name: $name}'
    )"
    printf 'Updated user %s profile\n' "${login}"
  fi

  request_json PUT "/api/admin/users/${user_id}/password" "$(
    jq -cn --arg password "${password}" '{password: $password}'
  )"
  request_json PATCH "/api/org/users/${user_id}" "$(
    jq -cn --arg role "${org_role}" '{role: $role}'
  )"
  request_json PUT "/api/admin/users/${user_id}/permissions" "$(
    jq -cn --argjson isGrafanaAdmin "${is_grafana_admin}" '{isGrafanaAdmin: $isGrafanaAdmin}'
  )"
}

delete_known_global_user() {
  local login="$1"
  local user_id

  user_id="$(lookup_global_user_id_by_login "${login}")"
  if [[ -z "${user_id}" ]]; then
    printf 'Skipped user %s: not found\n' "${login}"
    return
  fi

  request_json DELETE "/api/admin/users/${user_id}"
  printf 'Deleted user %s\n' "${login}"
}

lookup_org_user_id_by_login() {
  local org_id="$1"
  local login="$2"
  request_json GET "/api/orgs/${org_id}/users" "" "${org_id}"
  printf '%s' "${HTTP_BODY}" | jq -r --arg login "${login}" '.[] | select(.login == $login) | .userId // .userID // .id // empty' | head -n 1
}

ensure_org_membership() {
  local org_id="$1"
  local login="$2"
  local role="$3"
  local user_id
  local existing_user_id

  user_id="$(lookup_global_user_id_by_login "${login}")"
  [[ -n "${user_id}" ]] || fail "cannot add unknown user ${login} to org ${org_id}"

  existing_user_id="$(lookup_org_user_id_by_login "${org_id}" "${login}")"
  if [[ -z "${existing_user_id}" ]]; then
    request_json POST "/api/orgs/${org_id}/users" "$(
      jq -cn --arg loginOrEmail "${login}" --arg role "${role}" '{loginOrEmail: $loginOrEmail, role: $role}'
    )"
    printf 'Added user %s to org %s as %s\n' "${login}" "${org_id}" "${role}"
    return
  fi

  request_json PATCH "/api/orgs/${org_id}/users/${existing_user_id}" "$(
    jq -cn --arg role "${role}" '{role: $role}'
  )"
  printf 'Updated user %s role in org %s -> %s\n' "${login}" "${org_id}" "${role}"
}

lookup_team_id_by_name() {
  local org_id="$1"
  local name="$2"
  request_json GET "/api/teams/search?perpage=1000&page=1&query=$(urlencode "${name}")" "" "${org_id}"
  printf '%s' "${HTTP_BODY}" | jq -r --arg name "${name}" '.teams[]? | select(.name == $name) | .id' | head -n 1
}

ensure_team() {
  local org_id="$1"
  local name="$2"
  local email="$3"
  local members_json="$4"
  local admins_json="$5"
  local team_id
  local resolved_members_json=""
  local resolved_admins_json=""

  team_id="$(lookup_team_id_by_name "${org_id}" "${name}")"
  if [[ -z "${team_id}" ]]; then
    request_json POST "/api/teams" "$(
      jq -cn --arg name "${name}" --arg email "${email}" '{name: $name, email: $email}'
    )" "${org_id}"
    team_id="$(printf '%s' "${HTTP_BODY}" | jq -r '.teamId // .id // empty')"
    [[ -n "${team_id}" ]] || fail "failed to create team ${name} in org ${org_id}"
    printf 'Created team %s (org %s)\n' "${name}" "${org_id}"
  else
    printf 'Reused team %s (org %s)\n' "${name}" "${org_id}"
  fi

  resolved_members_json="$(resolve_identity_array_to_emails "${members_json}")"
  resolved_admins_json="$(resolve_identity_array_to_emails "${admins_json}")"

  request_json PUT "/api/teams/${team_id}/members" "$(
    jq -cn --argjson members "${resolved_members_json}" --argjson admins "${resolved_admins_json}" '{members: $members, admins: $admins}'
  )" "${org_id}"
  printf 'Synchronized team %s members/admins (org %s)\n' "${name}" "${org_id}"
}

delete_team_by_name() {
  local org_id="$1"
  local name="$2"
  local team_id

  team_id="$(lookup_team_id_by_name "${org_id}" "${name}")"
  if [[ -z "${team_id}" ]]; then
    printf 'Skipped team %s (org %s): not found\n' "${name}" "${org_id}"
    return
  fi

  request_json DELETE "/api/teams/${team_id}" "" "${org_id}"
  printf 'Deleted team %s (org %s)\n' "${name}" "${org_id}"
}

lookup_service_account_id_by_name() {
  local org_id="$1"
  local name="$2"
  request_json GET "/api/serviceaccounts/search?perpage=1000&page=1&query=$(urlencode "${name}")" "" "${org_id}"
  printf '%s' "${HTTP_BODY}" | jq -r --arg name "${name}" '.serviceAccounts[]? | select(.name == $name) | .id' | head -n 1
}

ensure_service_account() {
  local org_id="$1"
  local name="$2"
  local role="$3"
  local disabled="$4"
  local service_account_id

  service_account_id="$(lookup_service_account_id_by_name "${org_id}" "${name}")"
  if [[ -z "${service_account_id}" ]]; then
    request_json POST "/api/serviceaccounts" "$(
      jq -cn --arg name "${name}" --arg role "${role}" --argjson isDisabled "${disabled}" '{name: $name, role: $role, isDisabled: $isDisabled}'
    )" "${org_id}"
    printf 'Created service account %s (org %s)\n' "${name}" "${org_id}"
    return
  fi

  request_json PATCH "/api/serviceaccounts/${service_account_id}" "$(
    jq -cn --arg name "${name}" --arg role "${role}" --argjson isDisabled "${disabled}" '{name: $name, role: $role, isDisabled: $isDisabled}'
  )" "${org_id}"
  printf 'Updated service account %s (org %s)\n' "${name}" "${org_id}"
}

delete_service_account_by_name() {
  local org_id="$1"
  local name="$2"
  local service_account_id

  service_account_id="$(lookup_service_account_id_by_name "${org_id}" "${name}")"
  if [[ -z "${service_account_id}" ]]; then
    printf 'Skipped service account %s (org %s): not found\n' "${name}" "${org_id}"
    return
  fi

  request_json DELETE "/api/serviceaccounts/${service_account_id}" "" "${org_id}"
  printf 'Deleted service account %s (org %s)\n' "${name}" "${org_id}"
}

lookup_datasource_uid() {
  local org_id="$1"
  local uid="$2"
  local name="$3"
  request_json GET "/api/datasources" "" "${org_id}"
  printf '%s' "${HTTP_BODY}" |
    jq -r --arg uid "${uid}" --arg name "${name}" \
      '.[] | select(.uid == $uid or .name == $name) | .uid' | head -n 1
}

ensure_datasource() {
  local org_id="$1"
  local uid="$2"
  local name="$3"
  local ds_type="$4"
  local url="$5"
  local is_default="$6"
  local existing_uid
  local recreated=false

  request_json GET "/api/datasources" "" "${org_id}"
  existing_uid="$(
    printf '%s' "${HTTP_BODY}" |
      jq -r --arg uid "${uid}" --arg name "${name}" \
        '.[] | select(.uid == $uid or .name == $name) | .uid' | head -n 1
  )"
  if [[ -n "${existing_uid}" ]]; then
    if [[ "${existing_uid}" != "${uid}" ]]; then
      request_json DELETE "/api/datasources/uid/${existing_uid}" "" "${org_id}"
      recreated=true
    else
      printf 'Reused datasource %s (org %s)\n' "${name}" "${org_id}"
      return
    fi
  fi
  request_json POST "/api/datasources" "$(
    jq -cn \
      --arg uid "${uid}" \
      --arg name "${name}" \
      --arg type "${ds_type}" \
      --arg url "${url}" \
      --argjson isDefault "${is_default}" \
      '{
        uid: $uid,
        name: $name,
        type: $type,
        access: "proxy",
        url: $url,
        isDefault: $isDefault
      }'
  )" "${org_id}"
  if [[ "${recreated}" == true ]]; then
    printf 'Recreated datasource %s (org %s): replaced uid %s with %s\n' "${name}" "${org_id}" "${existing_uid}" "${uid}"
  else
    printf 'Created datasource %s (org %s)\n' "${name}" "${org_id}"
  fi
}

delete_datasource() {
  local org_id="$1"
  local uid="$2"
  local name="$3"
  local existing_uid

  existing_uid="$(lookup_datasource_uid "${org_id}" "${uid}" "${name}")"
  if [[ -z "${existing_uid}" ]]; then
    printf 'Skipped datasource %s (org %s): not found\n' "${name}" "${org_id}"
    return
  fi

  request_json DELETE "/api/datasources/uid/${existing_uid}" "" "${org_id}"
  printf 'Deleted datasource %s (org %s)\n' "${name}" "${org_id}"
}

require_global_user() {
  local login="$1"
  local expected_admin="$2"
  local user_id

  user_id="$(lookup_global_user_id_by_login "${login}")"
  [[ -n "${user_id}" ]] || fail "missing expected global user ${login}"
  request_json GET "/api/users/${user_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r '.isGrafanaAdmin // false')" == "${expected_admin}" ]] \
    || fail "global user ${login} grafana admin flag mismatch"
  printf 'Verified user %s\n' "${login}"
}

require_org_membership() {
  local org_id="$1"
  local login="$2"
  local expected_role="$3"

  request_json GET "/api/orgs/${org_id}/users" "" "${org_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg login "${login}" --arg role "${expected_role}" 'any(.[]; .login == $login and ((.role // .orgRole // "") == $role))')" == "true" ]] \
    || fail "missing expected org membership ${login}=${expected_role} in org ${org_id}"
  printf 'Verified org membership %s in org %s\n' "${login}" "${org_id}"
}

require_team() {
  local org_id="$1"
  local name="$2"
  local expected_member="$3"
  local expected_admin="$4"
  local team_id

  team_id="$(lookup_team_id_by_name "${org_id}" "${name}")"
  [[ -n "${team_id}" ]] || fail "missing expected team ${name} in org ${org_id}"
  request_json GET "/api/teams/${team_id}/members" "" "${org_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg identity "${expected_member}" 'any(.[]; (.login // .email // "") == $identity or (.email // "") == $identity)')" == "true" ]] \
    || fail "team ${name} in org ${org_id} missing member ${expected_member}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg identity "${expected_admin}" 'any(.[]; ((.login // .email // "") == $identity or (.email // "") == $identity) and (.permission == 4 or .role == "Admin" or .teamRole == "Admin"))')" == "true" ]] \
    || fail "team ${name} in org ${org_id} missing admin ${expected_admin}"
  printf 'Verified team %s (org %s)\n' "${name}" "${org_id}"
}

require_service_account() {
  local org_id="$1"
  local name="$2"
  local expected_role="$3"

  request_json GET "/api/serviceaccounts/search?perpage=1000&page=1&query=$(urlencode "${name}")" "" "${org_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg name "${name}" --arg role "${expected_role}" 'any(.serviceAccounts[]?; .name == $name and .role == $role)')" == "true" ]] \
    || fail "missing expected service account ${name}=${expected_role} in org ${org_id}"
  printf 'Verified service account %s (org %s)\n' "${name}" "${org_id}"
}

require_datasource() {
  local org_id="$1"
  local uid="$2"
  local name="$3"

  [[ -n "$(lookup_datasource_uid "${org_id}" "${uid}" "${name}")" ]] \
    || fail "missing expected datasource ${name} (${uid}) in org ${org_id}"
  printf 'Verified datasource %s (org %s)\n' "${name}" "${org_id}"
}

require_folder() {
  local org_id="$1"
  local uid="$2"
  local title="$3"

  request_json GET "/api/folders/${uid}" "" "${org_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg uid "${uid}" --arg title "${title}" '.uid == $uid and .title == $title')" == "true" ]] \
    || fail "missing expected folder ${title} (${uid}) in org ${org_id}"
  printf 'Verified folder %s (org %s)\n' "${title}" "${org_id}"
}

require_dashboard() {
  local org_id="$1"
  local uid="$2"

  request_json GET "/api/search?type=dash-db&limit=500&page=1" "" "${org_id}"
  [[ "$(printf '%s' "${HTTP_BODY}" | jq -r --arg uid "${uid}" 'any(.[]; .uid == $uid)')" == "true" ]] \
    || fail "missing expected dashboard ${uid} in org ${org_id}"
  printf 'Verified dashboard %s (org %s)\n' "${uid}" "${org_id}"
}

verify_main_org() {
  local org_id="$1"
  require_global_user "sample-platform-admin" "true"
  require_global_user "sample-editor" "false"
  require_global_user "sample-viewer" "false"
  require_global_user "sample-auditor" "false"
  require_service_account "${org_id}" "deploy-bot" "Editor"
  require_service_account "${org_id}" "report-bot" "Viewer"
  require_team "${org_id}" "platform-ops" "sample-editor@example.com" "sample-platform-admin@example.com"
  require_team "${org_id}" "api-dev" "sample-viewer@example.com" "sample-editor@example.com"
  require_datasource "${org_id}" "smoke-prom" "Smoke Prometheus"
  require_datasource "${org_id}" "smoke-prom-2" "Smoke Prometheus 2"
  require_datasource "${org_id}" "smoke-loki" "Smoke Loki"
  require_folder "${org_id}" "platform" "Platform"
  require_folder "${org_id}" "infra" "Infra"
  require_folder "${org_id}" "team" "Team"
  require_folder "${org_id}" "apps" "Apps"
  require_folder "${org_id}" "prod" "Prod"
  require_folder "${org_id}" "stage" "Stage"
  require_folder "${org_id}" "api" "API"
  require_dashboard "${org_id}" "smoke-main"
  require_dashboard "${org_id}" "smoke-prom-only"
  require_dashboard "${org_id}" "query-smoke"
  require_dashboard "${org_id}" "mixed-query-smoke"
  require_dashboard "${org_id}" "two-prom-query-smoke"
  require_dashboard "${org_id}" "subfolder-main"
  require_dashboard "${org_id}" "subfolder-chain-smoke"
  require_dashboard "${org_id}" "variable-chain-smoke"
  require_dashboard "${org_id}" "api-mixed-overview"
  require_dashboard "${org_id}" "stage-rollup"
}

verify_extra_org() {
  local org_name="$1"
  local dashboard_uid="$2"
  local datasource_uid="$3"
  local datasource_name="$4"
  local team_name="$5"
  local team_member="$6"
  local team_admin="$7"
  local service_account_name="$8"
  local service_account_role="$9"
  local membership_login="${10}"
  local membership_role="${11}"
  local admin_login="${12}"
  local admin_role="${13}"
  local org_id

  org_id="$(lookup_org_id_by_name "${org_name}")"
  [[ -n "${org_id}" ]] || fail "missing expected org ${org_name}"
  require_org_membership "${org_id}" "${membership_login}" "${membership_role}"
  require_org_membership "${org_id}" "${admin_login}" "${admin_role}"
  require_team "${org_id}" "${team_name}" "${team_member}" "${team_admin}"
  require_service_account "${org_id}" "${service_account_name}" "${service_account_role}"
  require_datasource "${org_id}" "${datasource_uid}" "${datasource_name}"
  require_dashboard "${org_id}" "${dashboard_uid}"
}

verify_sample_data() {
  verify_main_org "1"
  verify_extra_org "Org Two" "org-two-main" "org-two-prom" "Org Two Prometheus" "org-two-editors" "sample-editor@example.com" "sample-platform-admin@example.com" "org-two-bot" "Editor" "sample-editor" "Editor" "sample-platform-admin" "Admin"
  verify_extra_org "QA Org" "qa-overview" "qa-prom" "QA Prometheus" "qa-team" "sample-viewer@example.com" "sample-editor@example.com" "qa-bot" "Editor" "sample-viewer" "Editor" "sample-editor" "Admin"
  verify_extra_org "Audit Org" "audit-home" "audit-prom" "Audit Prometheus" "audit-reviewers" "sample-auditor@example.com" "sample-platform-admin@example.com" "audit-bot" "Viewer" "sample-auditor" "Viewer" "sample-platform-admin" "Admin"
  printf 'Verified sample Grafana data at %s\n' "${GRAFANA_URL}"
}

lookup_folder_uid() {
  local org_id="$1"
  local uid="$2"
  request_raw GET "/api/folders/${uid}" "" "${org_id}"
  if [[ "${HTTP_STATUS}" == "200" ]]; then
    printf '%s\n' "${uid}"
  fi
}

ensure_folder() {
  local org_id="$1"
  local uid="$2"
  local title="$3"
  local parent_uid="${4:-}"
  local existing_uid
  local payload

  existing_uid="$(lookup_folder_uid "${org_id}" "${uid}")"
  if [[ -n "${existing_uid}" ]]; then
    printf 'Reused folder %s (org %s)\n' "${title}" "${org_id}"
    return
  fi

  if [[ -n "${parent_uid}" ]]; then
    payload="$(jq -cn --arg uid "${uid}" --arg title "${title}" --arg parentUid "${parent_uid}" \
      '{uid: $uid, title: $title, parentUid: $parentUid}')"
  else
    payload="$(jq -cn --arg uid "${uid}" --arg title "${title}" '{uid: $uid, title: $title}')"
  fi
  request_json POST "/api/folders" "${payload}" "${org_id}"
  printf 'Created folder %s (org %s)\n' "${title}" "${org_id}"
}

delete_folder() {
  local org_id="$1"
  local uid="$2"
  local title="$3"

  if ! request_optional DELETE "/api/folders/${uid}" "" "${org_id}"; then
    printf 'Skipped folder %s (org %s): not found\n' "${title}" "${org_id}"
    return
  fi
  printf 'Deleted folder %s (org %s)\n' "${title}" "${org_id}"
}

upsert_dashboard() {
  local org_id="$1"
  local folder_uid="$2"
  local dashboard_json="$3"
  local uid
  local title
  local payload

  uid="$(printf '%s' "${dashboard_json}" | jq -r '.uid')"
  title="$(printf '%s' "${dashboard_json}" | jq -r '.title')"
  payload="$(jq -cn \
    --arg folderUid "${folder_uid}" \
    --argjson dashboard "${dashboard_json}" \
    '{dashboard: $dashboard, folderUid: $folderUid, overwrite: true, message: "developer sample seed"}'
  )"
  request_json POST "/api/dashboards/db" "${payload}" "${org_id}"
  printf 'Upserted dashboard %s (%s) in org %s\n' "${title}" "${uid}" "${org_id}"
}

delete_dashboard() {
  local org_id="$1"
  local uid="$2"

  if ! request_optional DELETE "/api/dashboards/uid/${uid}" "" "${org_id}"; then
    printf 'Skipped dashboard %s (org %s): not found\n' "${uid}" "${org_id}"
    return
  fi
  printf 'Deleted dashboard %s (org %s)\n' "${uid}" "${org_id}"
}

list_dashboard_uids() {
  local org_id="$1"
  local page=1
  local page_data

  while true; do
    request_json GET "/api/search?type=dash-db&limit=500&page=${page}" "" "${org_id}"
    page_data="$(printf '%s' "${HTTP_BODY}" | jq -r '.[].uid')"
    if [[ -z "${page_data}" ]]; then
      break
    fi
    printf '%s\n' "${page_data}"
    if [[ "$(printf '%s' "${HTTP_BODY}" | jq 'length')" -lt 500 ]]; then
      break
    fi
    page=$((page + 1))
  done
}

list_folder_uids() {
  local org_id="$1"
  request_json GET "/api/folders" "" "${org_id}"
  printf '%s' "${HTTP_BODY}" | jq -r '.[].uid'
}

delete_all_dashboards_in_org() {
  local org_id="$1"
  local uid
  while IFS= read -r uid; do
    [[ -n "${uid}" ]] || continue
    delete_dashboard "${org_id}" "${uid}"
  done < <(list_dashboard_uids "${org_id}")
}

delete_all_folders_in_org() {
  local org_id="$1"
  local uid
  while IFS= read -r uid; do
    [[ -n "${uid}" ]] || continue
    delete_folder "${org_id}" "${uid}" "${uid}"
  done < <(list_folder_uids "${org_id}")
}

delete_all_datasources_in_org() {
  local org_id="$1"
  local uid name
  request_json GET "/api/datasources" "" "${org_id}"
  while IFS=$'\t' read -r uid name; do
    [[ -n "${uid}" ]] || continue
    delete_datasource "${org_id}" "${uid}" "${name}"
  done < <(printf '%s' "${HTTP_BODY}" | jq -r '.[] | [.uid, .name] | @tsv')
}

delete_all_alert_rules_in_org() {
  local org_id="$1"
  local uid
  request_json GET "/api/v1/provisioning/alert-rules" "" "${org_id}"
  while IFS= read -r uid; do
    [[ -n "${uid}" ]] || continue
    request_json DELETE "/api/v1/provisioning/alert-rules/${uid}" "" "${org_id}"
    printf 'Deleted alert rule %s (org %s)\n' "${uid}" "${org_id}"
  done < <(printf '%s' "${HTTP_BODY}" | jq -r '.[]?.uid // empty')
}

delete_all_teams_in_org() {
  local org_id="$1"
  local team_id name
  request_json GET "/api/teams/search?perpage=1000&page=1" "" "${org_id}"
  while IFS=$'\t' read -r team_id name; do
    [[ -n "${team_id}" ]] || continue
    request_json DELETE "/api/teams/${team_id}" "" "${org_id}"
    printf 'Deleted team %s (org %s)\n' "${name}" "${org_id}"
  done < <(printf '%s' "${HTTP_BODY}" | jq -r '.teams[]? | [.id, .name] | @tsv')
}

delete_all_service_accounts_in_org() {
  local org_id="$1"
  local sa_id name
  request_json GET "/api/serviceaccounts/search?perpage=1000&page=1" "" "${org_id}"
  while IFS=$'\t' read -r sa_id name; do
    [[ -n "${sa_id}" ]] || continue
    request_json DELETE "/api/serviceaccounts/${sa_id}" "" "${org_id}"
    printf 'Deleted service account %s (org %s)\n' "${name}" "${org_id}"
  done < <(printf '%s' "${HTTP_BODY}" | jq -r '.serviceAccounts[]? | [.id, .name] | @tsv')
}

delete_non_admin_users() {
  local keep_login="$1"
  local user_id login is_admin
  request_json GET "/api/users?perpage=1000&page=1"
  while IFS=$'\t' read -r user_id login is_admin; do
    [[ -n "${user_id}" ]] || continue
    if [[ "${login}" == "${keep_login}" ]]; then
      continue
    fi
    if [[ "${is_admin}" == "true" ]]; then
      continue
    fi
    request_json DELETE "/api/admin/users/${user_id}"
    printf 'Deleted user %s\n' "${login}"
  done < <(printf '%s' "${HTTP_BODY}" | jq -r '.[]? | [.id, .login, (.isGrafanaAdmin // false)] | @tsv')
}

delete_non_default_orgs() {
  local org_id
  while IFS= read -r org_id; do
    [[ -n "${org_id}" ]] || continue
    if [[ "${org_id}" == "1" ]]; then
      continue
    fi
    request_json DELETE "/api/orgs/${org_id}"
    printf 'Deleted org %s\n' "${org_id}"
  done < <(list_org_ids)
}

reset_all_data() {
  local keep_login
  keep_login="$(current_admin_login)"
  [[ -n "${keep_login}" ]] || fail "failed to detect current Grafana login"

  delete_non_default_orgs
  delete_all_alert_rules_in_org "1"
  delete_all_dashboards_in_org "1"
  delete_all_folders_in_org "1"
  delete_all_datasources_in_org "1"
  delete_all_teams_in_org "1"
  delete_all_service_accounts_in_org "1"
  delete_non_admin_users "${keep_login}"
}

dashboard_smoke_main() {
  cat <<'EOF'
{
  "id": null,
  "uid": "smoke-main",
  "title": "Smoke Dashboard",
  "tags": ["sample", "smoke"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Up Query",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "up"}
      ],
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
    },
    {
      "id": 2,
      "title": "Recent Logs",
      "type": "logs",
      "datasource": {"type": "loki", "uid": "smoke-loki"},
      "targets": [
        {"refId": "A", "expr": "{job=\"smoke\"}"}
      ],
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
    }
  ]
}
EOF
}

dashboard_prom_only() {
  cat <<'EOF'
{
  "id": null,
  "uid": "smoke-prom-only",
  "title": "Prometheus Only",
  "tags": ["sample", "prometheus"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Only Prometheus",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "sum(up)"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_query_smoke() {
  cat <<'EOF'
{
  "id": null,
  "uid": "query-smoke",
  "title": "Query Smoke Dashboard",
  "tags": ["sample", "query"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Up Query",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "up{a=\"100\"}"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_mixed_query_smoke() {
  cat <<'EOF'
{
  "id": null,
  "uid": "mixed-query-smoke",
  "title": "Mixed Query Dashboard",
  "tags": ["sample", "mixed-datasource"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "refresh": "30s",
  "panels": [
    {
      "id": 1,
      "title": "Mixed Panel",
      "type": "timeseries",
      "datasource": {"type": "datasource", "uid": "-- Mixed --"},
      "targets": [
        {"refId": "A", "datasource": {"type": "prometheus", "uid": "smoke-prom"}, "expr": "up", "legendFormat": "prom"},
        {"refId": "B", "datasource": {"type": "loki", "uid": "smoke-loki"}, "expr": "{job=\"grafana\"}", "queryType": "range", "legendFormat": "loki"}
      ],
      "gridPos": {"h": 9, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_two_prom_query_smoke() {
  cat <<'EOF'
{
  "id": null,
  "uid": "two-prom-query-smoke",
  "title": "Two Prometheus Query Dashboard",
  "tags": ["sample", "two-prometheus"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "refresh": "30s",
  "panels": [
    {
      "id": 1,
      "title": "Two Prometheus Panel",
      "type": "timeseries",
      "datasource": {"type": "datasource", "uid": "-- Mixed --"},
      "targets": [
        {"refId": "A", "datasource": {"type": "prometheus", "uid": "smoke-prom"}, "expr": "up", "legendFormat": "prom-1"},
        {"refId": "B", "datasource": {"type": "prometheus", "uid": "smoke-prom-2"}, "expr": "up", "legendFormat": "prom-2"}
      ],
      "gridPos": {"h": 9, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_subfolder_main() {
  cat <<'EOF'
{
  "id": null,
  "uid": "subfolder-main",
  "title": "Subfolder Dashboard",
  "tags": ["sample", "folder"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Folder Query",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "rate(up[5m])"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_subfolder_chain_smoke() {
  cat <<'EOF'
{
  "id": null,
  "uid": "subfolder-chain-smoke",
  "title": "Subfolder Chain Dashboard",
  "tags": ["sample", "folder", "chain"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Prod Chain Query",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "sum(up)", "legendFormat": "prod"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_variable_chain_smoke() {
  cat <<'EOF'
{
  "id": null,
  "uid": "variable-chain-smoke",
  "title": "Variable Chain Dashboard",
  "tags": ["sample", "variables", "folder"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "templating": {
    "list": [
      {
        "name": "prom_ds",
        "label": "Prometheus DS",
        "type": "datasource",
        "query": "prometheus",
        "current": {"text": "Smoke Prometheus", "value": "smoke-prom"},
        "options": []
      },
      {
        "name": "env",
        "label": "Environment",
        "type": "custom",
        "query": "prod,stage",
        "current": {"text": "prod", "value": "prod"},
        "options": [
          {"text": "prod", "value": "prod", "selected": true},
          {"text": "stage", "value": "stage", "selected": false}
        ]
      },
      {
        "name": "job",
        "label": "Job",
        "type": "query",
        "datasource": {"type": "prometheus", "uid": "smoke-prom"},
        "query": "label_values(up, job)",
        "refresh": 1,
        "current": {"text": "grafana", "value": "grafana"},
        "options": []
      }
    ]
  },
  "panels": [
    {
      "id": 1,
      "title": "Selected Job Health",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "up{job=\"$job\"}", "legendFormat": "{{job}}"}
      ],
      "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
    },
    {
      "id": 2,
      "title": "Selected Environment Logs",
      "type": "logs",
      "datasource": {"type": "loki", "uid": "smoke-loki"},
      "targets": [
        {"refId": "A", "expr": "{env=\"$env\"}"}
      ],
      "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
    }
  ]
}
EOF
}

dashboard_api_mixed_overview() {
  cat <<'EOF'
{
  "id": null,
  "uid": "api-mixed-overview",
  "title": "API Mixed Overview",
  "tags": ["sample", "api", "mixed-datasource"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "API Error Rate",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "sum(rate(http_requests_total{job=\"api\",status=~\"5..\"}[5m]))"}
      ],
      "gridPos": {"h": 8, "w": 8, "x": 0, "y": 0}
    },
    {
      "id": 2,
      "title": "API Logs",
      "type": "logs",
      "datasource": {"type": "loki", "uid": "smoke-loki"},
      "targets": [
        {"refId": "A", "expr": "{app=\"api\"}"}
      ],
      "gridPos": {"h": 8, "w": 8, "x": 8, "y": 0}
    },
    {
      "id": 3,
      "title": "Dual Prom Compare",
      "type": "timeseries",
      "datasource": {"type": "datasource", "uid": "-- Mixed --"},
      "targets": [
        {"refId": "A", "datasource": {"type": "prometheus", "uid": "smoke-prom"}, "expr": "sum(up{job=\"api\"})", "legendFormat": "primary"},
        {"refId": "B", "datasource": {"type": "prometheus", "uid": "smoke-prom-2"}, "expr": "sum(up{job=\"api\"})", "legendFormat": "secondary"}
      ],
      "gridPos": {"h": 8, "w": 8, "x": 16, "y": 0}
    }
  ]
}
EOF
}

dashboard_stage_rollup() {
  cat <<'EOF'
{
  "id": null,
  "uid": "stage-rollup",
  "title": "Stage Rollup Dashboard",
  "tags": ["sample", "stage", "folder"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Stage Traffic",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "smoke-prom"},
      "targets": [
        {"refId": "A", "expr": "sum(rate(http_requests_total{env=\"stage\"}[5m]))"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_org_two() {
  cat <<'EOF'
{
  "id": null,
  "uid": "org-two-main",
  "title": "Org Two Dashboard",
  "tags": ["sample", "org-two"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Org Two Query",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "org-two-prom"},
      "targets": [
        {"refId": "A", "expr": "up"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_qa_overview() {
  cat <<'EOF'
{
  "id": null,
  "uid": "qa-overview",
  "title": "QA Overview",
  "tags": ["sample", "qa"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "QA Up",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "qa-prom"},
      "targets": [
        {"refId": "A", "expr": "up"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

dashboard_audit_home() {
  cat <<'EOF'
{
  "id": null,
  "uid": "audit-home",
  "title": "Audit Home",
  "tags": ["sample", "audit"],
  "timezone": "browser",
  "schemaVersion": 41,
  "version": 0,
  "panels": [
    {
      "id": 1,
      "title": "Audit Up",
      "type": "timeseries",
      "datasource": {"type": "prometheus", "uid": "audit-prom"},
      "targets": [
        {"refId": "A", "expr": "up"}
      ],
      "gridPos": {"h": 8, "w": 24, "x": 0, "y": 0}
    }
  ]
}
EOF
}

seed_main_org() {
  local org_id="$1"
  ensure_global_user "sample-platform-admin" "sample-platform-admin@example.com" "Sample Platform Admin" "secret123" "Admin" true
  ensure_global_user "sample-editor" "sample-editor@example.com" "Sample Editor" "secret123" "Editor" false
  ensure_global_user "sample-viewer" "sample-viewer@example.com" "Sample Viewer" "secret123" "Viewer" false
  ensure_global_user "sample-auditor" "sample-auditor@example.com" "Sample Auditor" "secret123" "Viewer" false
  ensure_service_account "${org_id}" "deploy-bot" "Editor" false
  ensure_service_account "${org_id}" "report-bot" "Viewer" true
  ensure_team "${org_id}" "platform-ops" "platform-ops@example.com" '["sample-editor","sample-viewer"]' '["sample-platform-admin"]'
  ensure_team "${org_id}" "api-dev" "api-dev@example.com" '["sample-viewer"]' '["sample-editor"]'
  ensure_datasource "${org_id}" "smoke-prom" "Smoke Prometheus" "prometheus" "http://prometheus:9090" true
  ensure_datasource "${org_id}" "smoke-prom-2" "Smoke Prometheus 2" "prometheus" "http://prometheus-two:9090" false
  ensure_datasource "${org_id}" "smoke-loki" "Smoke Loki" "loki" "http://loki:3100" false
  ensure_folder "${org_id}" "platform" "Platform"
  ensure_folder "${org_id}" "infra" "Infra" "platform"
  ensure_folder "${org_id}" "team" "Team" "platform"
  ensure_folder "${org_id}" "apps" "Apps" "team"
  ensure_folder "${org_id}" "prod" "Prod" "apps"
  ensure_folder "${org_id}" "stage" "Stage" "apps"
  ensure_folder "${org_id}" "api" "API" "apps"
  upsert_dashboard "${org_id}" "" "$(dashboard_smoke_main)"
  upsert_dashboard "${org_id}" "" "$(dashboard_prom_only)"
  upsert_dashboard "${org_id}" "" "$(dashboard_query_smoke)"
  upsert_dashboard "${org_id}" "" "$(dashboard_mixed_query_smoke)"
  upsert_dashboard "${org_id}" "" "$(dashboard_two_prom_query_smoke)"
  upsert_dashboard "${org_id}" "infra" "$(dashboard_subfolder_main)"
  upsert_dashboard "${org_id}" "prod" "$(dashboard_subfolder_chain_smoke)"
  upsert_dashboard "${org_id}" "api" "$(dashboard_variable_chain_smoke)"
  upsert_dashboard "${org_id}" "api" "$(dashboard_api_mixed_overview)"
  upsert_dashboard "${org_id}" "stage" "$(dashboard_stage_rollup)"
}

destroy_main_org() {
  local org_id="$1"
  delete_dashboard "${org_id}" "stage-rollup"
  delete_dashboard "${org_id}" "api-mixed-overview"
  delete_dashboard "${org_id}" "variable-chain-smoke"
  delete_dashboard "${org_id}" "subfolder-chain-smoke"
  delete_dashboard "${org_id}" "subfolder-main"
  delete_dashboard "${org_id}" "two-prom-query-smoke"
  delete_dashboard "${org_id}" "mixed-query-smoke"
  delete_dashboard "${org_id}" "query-smoke"
  delete_dashboard "${org_id}" "smoke-prom-only"
  delete_dashboard "${org_id}" "smoke-main"
  delete_folder "${org_id}" "api" "API"
  delete_folder "${org_id}" "stage" "Stage"
  delete_folder "${org_id}" "prod" "Prod"
  delete_folder "${org_id}" "apps" "Apps"
  delete_folder "${org_id}" "team" "Team"
  delete_folder "${org_id}" "infra" "Infra"
  delete_folder "${org_id}" "platform" "Platform"
  delete_datasource "${org_id}" "smoke-loki" "Smoke Loki"
  delete_datasource "${org_id}" "smoke-prom-2" "Smoke Prometheus 2"
  delete_datasource "${org_id}" "smoke-prom" "Smoke Prometheus"
  delete_team_by_name "${org_id}" "api-dev"
  delete_team_by_name "${org_id}" "platform-ops"
  delete_service_account_by_name "${org_id}" "report-bot"
  delete_service_account_by_name "${org_id}" "deploy-bot"
  delete_known_global_user "sample-auditor"
  delete_known_global_user "sample-viewer"
  delete_known_global_user "sample-editor"
  delete_known_global_user "sample-platform-admin"
}

seed_extra_org() {
  local org_name="$1"
  local datasource_uid="$2"
  local datasource_name="$3"
  local dashboard_json="$4"
  local team_name="$5"
  local team_members_json="$6"
  local team_admins_json="$7"
  local service_account_name="$8"
  local service_account_role="$9"
  local service_account_disabled="${10}"
  local org_id

  org_id="$(ensure_org "${org_name}")"
  ensure_datasource "${org_id}" "${datasource_uid}" "${datasource_name}" "prometheus" "http://prometheus:9090" true
  upsert_dashboard "${org_id}" "" "${dashboard_json}"
  ensure_team "${org_id}" "${team_name}" "${team_name}@example.com" "${team_members_json}" "${team_admins_json}"
  ensure_service_account "${org_id}" "${service_account_name}" "${service_account_role}" "${service_account_disabled}"
}

destroy_extra_org() {
  local org_name="$1"
  local datasource_uid="$2"
  local datasource_name="$3"
  local dashboard_uid="$4"
  local team_name="$5"
  local service_account_name="$6"
  local org_id

  org_id="$(lookup_org_id_by_name "${org_name}")"
  if [[ -z "${org_id}" ]]; then
    printf 'Skipped org %s: not found\n' "${org_name}"
    return
  fi

  delete_dashboard "${org_id}" "${dashboard_uid}"
  delete_team_by_name "${org_id}" "${team_name}"
  delete_service_account_by_name "${org_id}" "${service_account_name}"
  delete_datasource "${org_id}" "${datasource_uid}" "${datasource_name}"
  request_json DELETE "/api/orgs/${org_id}"
  printf 'Deleted org %s (%s)\n' "${org_name}" "${org_id}"
}

main() {
  local org_two_id=""
  local qa_org_id=""
  local audit_org_id=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --url)
        GRAFANA_URL="$2"
        shift 2
        ;;
      --basic-user)
        GRAFANA_USER="$2"
        shift 2
        ;;
      --basic-password)
        GRAFANA_PASSWORD="$2"
        shift 2
        ;;
      --verify)
        VERIFY_MODE=true
        shift
        ;;
      --destroy)
        DESTROY_MODE=true
        shift
        ;;
      --reset-all-data)
        RESET_ALL_DATA_MODE=true
        shift
        ;;
      --yes)
        CONFIRMED_RESET=true
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "unknown argument: $1"
        ;;
    esac
  done

  require_tool curl
  require_tool jq
  ensure_health

  if [[ "${RESET_ALL_DATA_MODE}" == "true" ]]; then
    if [[ "${CONFIRMED_RESET}" != "true" ]]; then
      fail "--reset-all-data requires --yes"
    fi
    if [[ "${VERIFY_MODE}" == "true" ]]; then
      fail "choose either --verify or --reset-all-data"
    fi
    if [[ "${DESTROY_MODE}" == "true" ]]; then
      fail "choose either --destroy or --reset-all-data"
    fi
    reset_all_data
    printf 'Reset repo-relevant Grafana test data at %s\n' "${GRAFANA_URL}"
    return
  fi

  if [[ "${VERIFY_MODE}" == "true" ]]; then
    if [[ "${DESTROY_MODE}" == "true" ]]; then
      fail "choose either --verify or --destroy"
    fi
    verify_sample_data
    return
  fi

  if [[ "${DESTROY_MODE}" == "true" ]]; then
    destroy_extra_org "Audit Org" "audit-prom" "Audit Prometheus" "audit-home" "audit-reviewers" "audit-bot"
    destroy_extra_org "QA Org" "qa-prom" "QA Prometheus" "qa-overview" "qa-team" "qa-bot"
    destroy_extra_org "Org Two" "org-two-prom" "Org Two Prometheus" "org-two-main" "org-two-editors" "org-two-bot"
    destroy_main_org "1"
    printf 'Destroyed sample Grafana data at %s\n' "${GRAFANA_URL}"
    return
  fi

  seed_main_org "1"
  org_two_id="$(ensure_org "Org Two")"
  qa_org_id="$(ensure_org "QA Org")"
  audit_org_id="$(ensure_org "Audit Org")"

  ensure_org_membership "${org_two_id}" "sample-editor" "Editor"
  ensure_org_membership "${org_two_id}" "sample-platform-admin" "Admin"
  ensure_org_membership "${qa_org_id}" "sample-viewer" "Editor"
  ensure_org_membership "${qa_org_id}" "sample-editor" "Admin"
  ensure_org_membership "${audit_org_id}" "sample-auditor" "Viewer"
  ensure_org_membership "${audit_org_id}" "sample-platform-admin" "Admin"

  seed_extra_org "Org Two" "org-two-prom" "Org Two Prometheus" "$(dashboard_org_two)" "org-two-editors" '["sample-editor"]' '["sample-platform-admin"]' "org-two-bot" "Editor" false
  seed_extra_org "QA Org" "qa-prom" "QA Prometheus" "$(dashboard_qa_overview)" "qa-team" '["sample-viewer"]' '["sample-editor"]' "qa-bot" "Editor" false
  seed_extra_org "Audit Org" "audit-prom" "Audit Prometheus" "$(dashboard_audit_home)" "audit-reviewers" '["sample-auditor"]' '["sample-platform-admin"]' "audit-bot" "Viewer" false

  printf 'Seeded sample Grafana data at %s\n' "${GRAFANA_URL}"
}

main "$@"
