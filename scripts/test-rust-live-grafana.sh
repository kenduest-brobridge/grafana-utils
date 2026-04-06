#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_DIR="$ROOT_DIR/rust"
CARGO_BIN="${CARGO_BIN:-cargo}"
RUST_LIVE_SCOPE="${RUST_LIVE_SCOPE:-full}"
GRAFANA_IMAGE="${GRAFANA_IMAGE:-grafana/grafana:12.4.1}"
GRAFANA_PORT="${GRAFANA_PORT:-43011}"
GRAFANA_USER="${GRAFANA_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-admin}"
GRAFANA_API_TOKEN="${GRAFANA_API_TOKEN:-}"
GRAFANA_URL=""
CONTAINER_NAME="${GRAFANA_CONTAINER_NAME:-grafana-util-live-grafana}"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/grafana-util-rust-live.XXXXXX")"
DASHBOARD_EXPORT_DIR="${WORK_DIR}/dashboards"
DASHBOARD_DRY_RUN_DIR="${WORK_DIR}/dashboards-dry-run"
DASHBOARD_INSPECTION_EXPORT_DIR="${WORK_DIR}/dashboards-inspection"
DATASOURCE_EXPORT_DIR="${WORK_DIR}/datasources"
DATASOURCE_MULTI_ORG_EXPORT_DIR="${WORK_DIR}/datasources-all-orgs"
ALERT_EXPORT_DIR="${WORK_DIR}/alerts"
MULTI_ORG_EXPORT_DIR="${WORK_DIR}/dashboards-all-orgs"
ACCESS_ORG_EXPORT_DIR="${WORK_DIR}/access-orgs"
ACCESS_SERVICE_ACCOUNT_EXPORT_DIR="${WORK_DIR}/access-service-accounts"
ACCESS_TEAM_REPLAY_EXPORT_DIR="${WORK_DIR}/access-teams-replay"
ACCESS_USER_REPLAY_EXPORT_DIR="${WORK_DIR}/access-users-replay"
ACCESS_USER_ORG_REPLAY_EXPORT_DIR="${WORK_DIR}/access-users-org-replay"
SYNC_BUNDLE_FILE="${WORK_DIR}/sync-source-bundle.json"
SYNC_TARGET_INVENTORY_FILE="${WORK_DIR}/sync-target-inventory.json"
SYNC_BUNDLE_PREFLIGHT_FILE="${WORK_DIR}/sync-bundle-preflight.json"
ALERT_CONTACT_FILE=""
ALERT_CONTACT_UID=""
ALERT_MUTE_TIMING_FILE=""
ALERT_MUTE_TIMING_NAME=""
ALERT_TEMPLATE_FILE=""
ALERT_TEMPLATE_NAME=""
ALERT_RULE_FILE=""
ALERT_RULE_UID=""

cleanup() {
  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
  rm -rf "${WORK_DIR}"
}

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

check_access_export_metadata_contract() {
  local export_dir="$1"
  local bundle_name="$2"
  local expected_kind="$3"
  local minimum_records="${4:-1}"
  local bundle_path="${export_dir}/${bundle_name}"
  local metadata_path="${export_dir}/export-metadata.json"
  local bundle_record_count=""

  [[ -f "${bundle_path}" ]] \
    || fail "rust access export did not write ${bundle_name}"
  [[ -f "${metadata_path}" ]] \
    || fail "rust access export did not write export-metadata.json"

  bundle_record_count="$(jq -r '.records | length' "${bundle_path}")"
  jq -e '
    (.kind == $kind)
    and (.version == 1)
    and ((.records | length) == $record_count)
    and ($record_count >= ($minimum_records | tonumber))
  ' \
    --arg kind "${expected_kind}" \
    --arg minimum_records "${minimum_records}" \
    --argjson record_count "${bundle_record_count}" \
    "${bundle_path}" >/dev/null \
    || fail "rust access export bundle contract failed for ${bundle_name}"

  jq -e '
    (.kind == $kind)
    and (.version == 1)
    and (.recordCount == $record_count)
    and (.sourceUrl == $source_url)
    and (.sourceDir == $source_dir)
  ' \
    --arg kind "${expected_kind}" \
    --argjson record_count "${bundle_record_count}" \
    --arg source_url "${GRAFANA_URL}" \
    --arg source_dir "${export_dir}" \
    "${metadata_path}" >/dev/null \
    || fail "rust access export metadata contract failed for ${bundle_name}"
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

normalize_inspection_report_projection() {
  local path="$1"

  jq -S '
    def stable_list:
      sort | unique;
    def stable_query:
      {
        org,
        orgId,
        dashboardUid,
        dashboardTitle,
        dashboardTags: (.dashboardTags | stable_list),
        folderPath,
        folderFullPath,
        folderLevel,
        folderUid,
        parentFolderUid,
        panelId,
        panelTitle,
        panelType,
        panelTargetCount,
        panelQueryCount,
        panelDatasourceCount,
        panelVariables: (.panelVariables | stable_list),
        refId,
        datasource,
        datasourceName,
        datasourceUid,
        datasourceOrg,
        datasourceOrgId,
        datasourceDatabase,
        datasourceBucket,
        datasourceOrganization,
        datasourceIndexPattern,
        datasourceType,
        datasourceFamily,
        queryField,
        targetHidden,
        targetDisabled,
        query,
        queryVariables: (.queryVariables | stable_list),
        metrics: (.metrics | stable_list),
        functions: (.functions | stable_list),
        measurements: (.measurements | stable_list),
        buckets: (.buckets | stable_list)
      };
    {
      summary: {
        dashboardCount: .summary.dashboardCount,
        panelCount: .summary.panelCount,
        queryCount: .summary.queryCount,
        queryRecordCount: .summary.queryRecordCount
      },
      queries: ([.queries[] | stable_query] | sort_by(.dashboardUid, .panelId, .refId, .datasourceUid, .datasourceFamily, .queryField, .datasourceName))
    }
  ' "${path}"
}

normalize_inspection_governance_projection() {
  local path="$1"

  jq -S '
    def stable_list:
      sort | unique;
    def stable_family:
      {
        family,
        datasourceTypes: (.datasourceTypes | stable_list),
        datasourceCount,
        dashboardCount,
        panelCount,
        queryCount
      };
    def stable_dependency:
      {
        dashboardUid,
        dashboardTitle,
        folderPath,
        panelCount,
        queryCount,
        datasources: (.datasources | stable_list),
        datasourceFamilies: (.datasourceFamilies | stable_list)
      };
    def stable_datasource:
      {
        datasourceUid,
        datasource,
        family,
        queryCount,
        dashboardCount,
        panelCount,
        queryFields: (.queryFields | stable_list),
        orphaned
      };
    def stable_risk:
      {
        kind,
        severity,
        category,
        dashboardUid,
        panelId,
        datasource,
        detail,
        recommendation
      };
    {
      summary: {
        dashboardCount: .summary.dashboardCount,
        queryRecordCount: .summary.queryRecordCount,
        datasourceInventoryCount: .summary.datasourceInventoryCount,
        datasourceFamilyCount: .summary.datasourceFamilyCount,
        datasourceCoverageCount: .summary.datasourceCoverageCount,
        mixedDatasourceDashboardCount: .summary.mixedDatasourceDashboardCount,
        orphanedDatasourceCount: .summary.orphanedDatasourceCount,
        riskRecordCount: .summary.riskRecordCount
      },
      datasourceFamilies: ([.datasourceFamilies[] | stable_family] | sort_by(.family, .datasourceCount, .dashboardCount, .panelCount, .queryCount)),
      dashboardDependencies: ([.dashboardDependencies[] | stable_dependency] | sort_by(.dashboardUid, .folderPath, .panelCount, .queryCount)),
      datasources: ([.datasources[] | stable_datasource] | sort_by(.datasourceUid, .family, .queryCount, .dashboardCount, .panelCount)),
      riskRecords: ([.riskRecords[] | stable_risk] | sort_by(.kind, .severity, .category, .dashboardUid, .panelId, .datasource, .detail, .recommendation))
    }
  ' "${path}"
}

compare_normalized_report_json() {
  local label="$1"
  local left="$2"
  local right="$3"
  local left_normalized="${WORK_DIR}/${label}-left.json"
  local right_normalized="${WORK_DIR}/${label}-right.json"

  normalize_inspection_report_projection "${left}" >"${left_normalized}"
  normalize_inspection_report_projection "${right}" >"${right_normalized}"
  if ! diff -u "${left_normalized}" "${right_normalized}" >/dev/null; then
    diff -u "${left_normalized}" "${right_normalized}" >&2 || true
    fail "${label} parity mismatch between inspect-export and inspect-live"
  fi
}

compare_normalized_governance_json() {
  local label="$1"
  local left="$2"
  local right="$3"
  local left_normalized="${WORK_DIR}/${label}-left.json"
  local right_normalized="${WORK_DIR}/${label}-right.json"

  normalize_inspection_governance_projection "${left}" >"${left_normalized}"
  normalize_inspection_governance_projection "${right}" >"${right_normalized}"
  if ! diff -u "${left_normalized}" "${right_normalized}" >/dev/null; then
    diff -u "${left_normalized}" "${right_normalized}" >&2 || true
    fail "${label} parity mismatch between inspect-export and inspect-live"
  fi
}

rewrite_contact_point_url() {
  local path="$1"
  local url="$2"
  local tmp_path="${path}.tmp"

  jq --arg url "${url}" '.spec.settings.url = $url' "${path}" >"${tmp_path}" \
    || fail "failed to rewrite contact point URL in ${path}"
  mv "${tmp_path}" "${path}"
}

rewrite_mute_timing_end_time() {
  local path="$1"
  local end_time="$2"
  local tmp_path="${path}.tmp"

  jq --arg end_time "${end_time}" '.spec.time_intervals[0].times[0].end_time = $end_time' "${path}" >"${tmp_path}" \
    || fail "failed to rewrite mute timing in ${path}"
  mv "${tmp_path}" "${path}"
}

rewrite_template_body() {
  local path="$1"
  local template="$2"
  local tmp_path="${path}.tmp"

  jq --arg template "${template}" '.spec.template = $template' "${path}" >"${tmp_path}" \
    || fail "failed to rewrite template in ${path}"
  mv "${tmp_path}" "${path}"
}

rewrite_alert_rule_title() {
  local path="$1"
  local title="$2"
  local tmp_path="${path}.tmp"

  jq --arg title "${title}" '.spec.title = $title' "${path}" >"${tmp_path}" \
    || fail "failed to rewrite alert rule in ${path}"
  mv "${tmp_path}" "${path}"
}

check_alert_export_contract() {
  local export_dir="$1"
  local root_index_path="${export_dir}/index.json"
  local contact_index_path="${export_dir}/raw/contact-points/index.json"
  local policies_path="${export_dir}/raw/policies/notification-policies.json"

  [[ -f "${root_index_path}" ]] || fail "alert export root index was not written"
  [[ -f "${contact_index_path}" ]] || fail "alert export did not write contact-point index.json"
  [[ -f "${policies_path}" ]] || fail "alert export did not write notification-policies.json"

  jq -e '
    (.kind == "grafana-util-alert-export-index")
    and (.schemaVersion == 1)
    and (.apiVersion == 1)
    and ((.rules | length) >= 1)
    and ((."contact-points" | length) >= 1)
    and ((."mute-timings" | length) >= 1)
    and ((.policies | length) == 1)
    and ((.templates | length) >= 1)
  ' "${root_index_path}" >/dev/null \
    || fail "alert export root index contract failed"

  jq -e '
    (length >= 1)
    and any(.uid == "smoke-webhook" and (.path | contains("Smoke_Webhook")))
  ' "${contact_index_path}" >/dev/null \
    || fail "alert export contact-point index contract failed"

  jq -e '
    (.kind == "grafana-notification-policies")
    and (.spec | type == "object")
  ' "${policies_path}" >/dev/null \
    || fail "alert export notification-policies contract failed"
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

  docker rm -f "${CONTAINER_NAME}" >/dev/null 2>&1 || true
  check_requested_grafana_port
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

seed_datasource_typed() {
  local org_id="${1:-}"
  local name="${2:?datasource name is required}"
  local uid="${3:?datasource uid is required}"
  local datasource_type="${4:?datasource type is required}"
  local datasource_url="${5:?datasource url is required}"
  local is_default="${6:-false}"
  local api_runner="api"
  if [[ -n "${org_id}" ]]; then
    api_runner="api_org ${org_id}"
  fi
  ${api_runner} POST "/api/datasources" "{
    \"uid\": \"${uid}\",
    \"name\": \"${name}\",
    \"type\": \"${datasource_type}\",
    \"access\": \"proxy\",
    \"url\": \"${datasource_url}\",
    \"isDefault\": ${is_default}
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

seed_core_family_inspection_dashboard() {
  api POST "/api/dashboards/db" '{
    "dashboard": {
      "id": null,
      "uid": "inspect-core-families",
      "title": "Inspect Core Families",
      "tags": ["smoke", "inspect"],
      "timezone": "browser",
      "schemaVersion": 39,
      "version": 0,
      "panels": [
        {
          "id": 11,
          "title": "Prometheus Family",
          "type": "timeseries",
          "datasource": {"uid": "smoke-prometheus", "type": "prometheus"},
          "targets": [
            {
              "refId": "A",
              "datasource": {"uid": "smoke-prometheus", "type": "prometheus"},
              "expr": "rate(http_requests_total[5m])"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
        },
        {
          "id": 12,
          "title": "Loki Family",
          "type": "logs",
          "datasource": {"uid": "smoke-loki", "type": "loki"},
          "targets": [
            {
              "refId": "B",
              "datasource": {"uid": "smoke-loki", "type": "loki"},
              "expr": "count_over_time({job=\"grafana\"} | json [5m])"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
        },
        {
          "id": 13,
          "title": "Flux Family",
          "type": "timeseries",
          "datasource": {"uid": "smoke-influx", "type": "influxdb"},
          "targets": [
            {
              "refId": "C",
              "datasource": {"uid": "smoke-influx", "type": "influxdb"},
              "query": "from(bucket: \"prod\") |> range(start: -1h)"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8}
        },
        {
          "id": 14,
          "title": "SQL Family",
          "type": "table",
          "datasource": {"uid": "smoke-postgres", "type": "postgres"},
          "targets": [
            {
              "refId": "D",
              "datasource": {"uid": "smoke-postgres", "type": "postgres"},
              "rawSql": "SELECT * FROM public.cpu_metrics LIMIT 5"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
        },
        {
          "id": 15,
          "title": "Search Family",
          "type": "table",
          "datasource": {"uid": "smoke-search", "type": "elasticsearch"},
          "targets": [
            {
              "refId": "E",
              "datasource": {"uid": "smoke-search", "type": "elasticsearch"},
              "query": "status:500 AND _exists_:@timestamp"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 0, "y": 16}
        },
        {
          "id": 16,
          "title": "Tracing Family",
          "type": "table",
          "datasource": {"uid": "smoke-tempo", "type": "tempo"},
          "targets": [
            {
              "refId": "F",
              "datasource": {"uid": "smoke-tempo", "type": "tempo"},
              "query": "service.name:checkout AND trace.id:abc123"
            }
          ],
          "gridPos": {"h": 8, "w": 12, "x": 12, "y": 16}
        }
      ]
    },
    "folderUid": "",
    "overwrite": true,
    "message": "smoke test inspect fixture"
  }' >/dev/null
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
  if api GET "/api/v1/provisioning/contact-points" | jq -e 'any(.uid == "smoke-webhook")' >/dev/null; then
    return
  fi
  api POST "/api/v1/provisioning/contact-points" '{
    "uid": "smoke-webhook",
    "name": "Smoke Webhook",
    "type": "webhook",
    "settings": {
      "url": "http://127.0.0.1/notify"
    }
  }' >/dev/null
}

seed_alert_folder() {
  if api GET "/api/folders/smoke-alerts" >/dev/null 2>&1; then
    return
  fi
  api POST "/api/folders" '{
    "uid": "smoke-alerts",
    "title": "Smoke Alerts"
  }' >/dev/null
}

seed_mute_timing() {
  if api GET "/api/v1/provisioning/mute-timings" | jq -e 'any(.name == "Off Hours")' >/dev/null; then
    return
  fi
  api POST "/api/v1/provisioning/mute-timings" '{
    "name": "Off Hours",
    "time_intervals": [
      {
        "times": [
          {
            "start_time": "00:00",
            "end_time": "06:00"
          }
        ]
      }
    ]
  }' >/dev/null
}

seed_template() {
  if api GET "/api/v1/provisioning/templates/slack.default" >/dev/null 2>&1; then
    return
  fi
  api PUT "/api/v1/provisioning/templates/slack.default" '{
    "template": "{{ define \"slack.default\" }}ok{{ end }}",
    "version": ""
  }' >/dev/null
}

seed_alert_rule() {
  if api GET "/api/v1/provisioning/alert-rules/cpu-high" >/dev/null 2>&1; then
    return
  fi
  api POST "/api/v1/provisioning/alert-rules" '{
    "uid": "cpu-high",
    "title": "CPU High",
    "ruleGroup": "CPU Alerts",
    "folderUID": "smoke-alerts",
    "noDataState": "OK",
    "execErrState": "OK",
    "for": "5m",
    "orgID": 1,
    "condition": "B",
    "annotations": {
      "summary": "cpu high smoke"
    },
    "labels": {
      "severity": "warning"
    },
    "isPaused": false,
    "notification_settings": null,
    "data": [
      {
        "refId": "A",
        "queryType": "",
        "relativeTimeRange": {
          "from": 600,
          "to": 0
        },
        "datasourceUid": "smoke-prometheus",
        "model": {
          "datasource": {
            "type": "prometheus",
            "uid": "smoke-prometheus"
          },
          "expr": "up",
          "hide": false,
          "intervalMs": 1000,
          "maxDataPoints": 43200,
          "refId": "A"
        }
      },
      {
        "refId": "B",
        "queryType": "",
        "relativeTimeRange": {
          "from": 0,
          "to": 0
        },
        "datasourceUid": "-100",
        "model": {
          "conditions": [
            {
              "evaluator": {
                "params": [0.5],
                "type": "gt"
              },
              "operator": {
                "type": "and"
              },
              "query": {
                "params": ["A"]
              },
              "reducer": {
                "params": [],
                "type": "last"
              },
              "type": "query"
            }
          ],
          "datasource": {
            "type": "__expr__",
            "uid": "-100"
          },
          "hide": false,
          "intervalMs": 1000,
          "maxDataPoints": 43200,
          "refId": "B",
          "type": "classic_conditions"
        }
      }
    ]
  }' >/dev/null
}

seed_alert_resources() {
  if ! api GET "/api/datasources" | jq -e 'any(.uid == "smoke-prometheus")' >/dev/null; then
    seed_datasource "" "Smoke Prometheus" "smoke-prometheus"
  fi
  seed_alert_folder
  seed_contact_point
  seed_mute_timing
  seed_template
  seed_alert_rule
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
  local list_json team_json org_json org_replay_json org_list_json service_account_json token_json token_delete_json delete_json user_json
  local user_diff_same_log="${WORK_DIR}/access-user-diff-same.log"
  local user_diff_changed_log="${WORK_DIR}/access-user-diff-changed.log"
  local user_import_dry_run_json="${WORK_DIR}/access-user-import-dry-run.json"
  local user_mutated_bundle="${WORK_DIR}/access-user-bundle.mutated.json"
  local user_record_count="0"
  local user_org_diff_same_log="${WORK_DIR}/access-user-org-diff-same.log"
  local user_org_diff_changed_log="${WORK_DIR}/access-user-org-diff-changed.log"
  local user_org_import_dry_run_json="${WORK_DIR}/access-user-org-import-dry-run.json"
  local user_org_mutated_bundle="${WORK_DIR}/access-user-org-bundle.mutated.json"
  local user_org_record_count="0"
  local team_diff_same_log="${WORK_DIR}/access-team-diff-same.log"
  local team_diff_changed_log="${WORK_DIR}/access-team-diff-changed.log"
  local team_import_dry_run_json="${WORK_DIR}/access-team-import-dry-run.json"
  local team_mutated_bundle="${WORK_DIR}/access-team-bundle.mutated.json"
  local team_record_count="0"
  local org_diff_same_log="${WORK_DIR}/access-org-diff-same.log"
  local org_diff_changed_log="${WORK_DIR}/access-org-diff-changed.log"
  local org_import_dry_run_log="${WORK_DIR}/access-org-import-dry-run.log"
  local org_mutated_bundle="${WORK_DIR}/access-org-bundle.mutated.json"
  local org_record_count="0"
  local org_replay_id=""
  local service_account_diff_same_log="${WORK_DIR}/access-service-account-diff-same.log"
  local service_account_diff_changed_log="${WORK_DIR}/access-service-account-diff-changed.log"
  local service_account_import_dry_run_json="${WORK_DIR}/access-service-account-import-dry-run.json"
  local service_account_mutated_bundle="${WORK_DIR}/access-service-account-bundle.mutated.json"
  local service_account_record_count="0"

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
    --login rust-access-user-replay \
    --email rust-access-user-replay@example.com \
    --name "Rust Access User Replay" \
    --org-role Viewer \
    --password secret123 >/dev/null

  "$(access_bin)" access user export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --export-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" \
    --overwrite >/dev/null

  check_access_export_metadata_contract \
    "${ACCESS_USER_REPLAY_EXPORT_DIR}" \
    "users.json" \
    "grafana-utils-access-user-export-index"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --diff-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" >"${user_diff_same_log}"
  grep -q 'No user differences across ' "${user_diff_same_log}" \
    || fail "rust access user diff did not report same-state after export"

  jq '
    .records |= map(
      if .login == "rust-access-user-replay"
      then .name = "Rust Access User Replay Two"
        | .grafanaAdmin = true
        | .password = "recreate-secret123"
      else .
      end
    )
  ' "${ACCESS_USER_REPLAY_EXPORT_DIR}/users.json" >"${user_mutated_bundle}"
  mv "${user_mutated_bundle}" "${ACCESS_USER_REPLAY_EXPORT_DIR}/users.json"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --diff-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" >"${user_diff_changed_log}"
  grep -q 'Diff different user rust-access-user-replay fields=' "${user_diff_changed_log}" \
    || fail "rust access user diff did not report the mutated replay drift"
  grep -q 'name' "${user_diff_changed_log}" \
    || fail "rust access user diff did not report the replay name drift"
  grep -q 'grafanaAdmin' "${user_diff_changed_log}" \
    || fail "rust access user diff did not report the replay grafanaAdmin drift"

  "$(access_bin)" access user import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --import-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --dry-run \
    --json >"${user_import_dry_run_json}"
  jq -e '
    (.summary.processed >= 1)
    and (.summary.updated >= 1)
    and (.rows | any(.identity == "rust-access-user-replay" and .action == "update-profile" and .detail == "would update user profile"))
    and (.rows | any(.identity == "rust-access-user-replay" and .action == "update-admin" and .detail == "would update grafanaAdmin -> true"))
    and (.rows | any(.identity == "rust-access-user-replay" and .action == "updated" and .detail == "would update user"))
  ' "${user_import_dry_run_json}" >/dev/null \
    || fail "rust access user dry-run import did not emit the expected structured replay preview"

  "$(access_bin)" access user import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --import-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" \
    --replace-existing >/dev/null

  user_json="$(
    "$(access_bin)" access user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope global \
      --login rust-access-user-replay \
      --json
  )"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].name')" == "Rust Access User Replay Two" ]] \
    || fail "rust access user import did not update the replay user name"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].grafanaAdmin')" == "true" ]] \
    || fail "rust access user import did not update the replay grafana admin flag"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --diff-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" >"${user_diff_same_log}"
  grep -q 'No user differences across ' "${user_diff_same_log}" \
    || fail "rust access user diff did not return to same-state after replay import"

  "$(access_bin)" access user delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --login rust-access-user-replay \
    --yes >/dev/null

  "$(access_bin)" access user import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope global \
    --import-dir "${ACCESS_USER_REPLAY_EXPORT_DIR}" \
    --replace-existing >/dev/null

  user_json="$(
    "$(access_bin)" access user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope global \
      --login rust-access-user-replay \
      --json
  )"
  [[ "$(printf '%s' "${user_json}" | jq '[.[] | select(.login == "rust-access-user-replay")] | length')" == "1" ]] \
    || fail "rust access user import did not recreate the deleted replay user"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].name')" == "Rust Access User Replay Two" ]] \
    || fail "rust access user recreate import did not preserve the replay user name"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].grafanaAdmin')" == "true" ]] \
    || fail "rust access user recreate import did not preserve the replay grafana admin flag"

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --login rust-access-user-org-replay \
    --email rust-access-user-org-replay@example.com \
    --name "Rust Access User Org Replay" \
    --org-role Viewer \
    --password secret123 >/dev/null

  "$(access_bin)" access team add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-user-org-current \
    --member rust-access-user-org-replay >/dev/null

  "$(access_bin)" access team add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-user-org-target >/dev/null

  "$(access_bin)" access user export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --with-teams \
    --export-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" \
    --overwrite >/dev/null

  check_access_export_metadata_contract \
    "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" \
    "users.json" \
    "grafana-utils-access-user-export-index"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --diff-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" >"${user_org_diff_same_log}"
  grep -q 'No user differences across ' "${user_org_diff_same_log}" \
    || fail "rust access org user diff did not report same-state after export"

  jq '
    .records |= map(
      if .login == "rust-access-user-org-replay"
      then .orgRole = "Editor"
        | .teams = ["rust-access-user-org-target"]
      else .
      end
    )
  ' "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}/users.json" >"${user_org_mutated_bundle}"
  mv "${user_org_mutated_bundle}" "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}/users.json"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --diff-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" >"${user_org_diff_changed_log}"
  grep -q 'Diff different user rust-access-user-org-replay fields=' "${user_org_diff_changed_log}" \
    || fail "rust access org user diff did not report the mutated replay drift"
  grep -q 'orgRole' "${user_org_diff_changed_log}" \
    || fail "rust access org user diff did not report the replay orgRole drift"
  grep -q 'teams' "${user_org_diff_changed_log}" \
    || fail "rust access org user diff did not report the replay team drift"

  "$(access_bin)" access user import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --import-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --yes \
    --dry-run \
    --json >"${user_org_import_dry_run_json}"
  jq -e '
    (.summary.processed >= 1)
    and (.summary.updated >= 1)
    and (.rows | any(.identity == "rust-access-user-org-replay" and .action == "update-org-role" and .detail == "would update orgRole -> Editor"))
    and (.rows | any(.identity == "rust-access-user-org-replay" and .action == "add-team" and .detail == "would add user to team rust-access-user-org-target"))
    and (.rows | any(.identity == "rust-access-user-org-replay" and .action == "remove-team" and .detail == "would remove user from team rust-access-user-org-current"))
    and (.rows | any(.identity == "rust-access-user-org-replay" and .action == "updated" and .detail == "would update user"))
  ' "${user_org_import_dry_run_json}" >/dev/null \
    || fail "rust access org user dry-run import did not emit the expected structured replay preview"

  "$(access_bin)" access user import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --import-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --yes >/dev/null

  user_json="$(
    "$(access_bin)" access user list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --scope org \
      --with-teams \
      --login rust-access-user-org-replay \
      --json
  )"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].orgRole')" == "Editor" ]] \
    || fail "rust access org user import did not update the replay org role"
  [[ "$(printf '%s' "${user_json}" | jq -r '.[0].teams | join(",")')" == "rust-access-user-org-target" ]] \
    || fail "rust access org user import did not update the replay team membership"

  "$(access_bin)" access user diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --scope org \
    --diff-dir "${ACCESS_USER_ORG_REPLAY_EXPORT_DIR}" >"${user_org_diff_same_log}"
  grep -q 'No user differences across ' "${user_org_diff_same_log}" \
    || fail "rust access org user diff did not return to same-state after replay import"

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

  "$(access_bin)" access team add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-team-replay \
    --member rust-access-team-member >/dev/null

  "$(access_bin)" access team export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${ACCESS_TEAM_REPLAY_EXPORT_DIR}" \
    --overwrite \
    --with-members >/dev/null

  check_access_export_metadata_contract \
    "${ACCESS_TEAM_REPLAY_EXPORT_DIR}" \
    "teams.json" \
    "grafana-utils-access-team-export-index"

  jq '
    .records |= (
      map(select(.name == "rust-access-team-replay"))
      | map(.members = ["rust-access-team-member@example.com"] | .admins = [])
    )
  ' "${ACCESS_TEAM_REPLAY_EXPORT_DIR}/teams.json" >"${team_mutated_bundle}"
  mv "${team_mutated_bundle}" "${ACCESS_TEAM_REPLAY_EXPORT_DIR}/teams.json"

  "$(access_bin)" access team import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_TEAM_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --dry-run \
    --yes \
    --json >"${team_import_dry_run_json}"
  jq -e '
    (.summary.processed == 1)
    and (.summary.created == 0)
    and (.summary.updated == 1)
    and (.summary.skipped == 0)
    and (.rows | length >= 1)
    and (.rows | any(.identity == "rust-access-team-replay" and .action == "updated" and .detail == "would update team"))
  ' "${team_import_dry_run_json}" >/dev/null \
    || fail "rust access team dry-run import did not preview the membership drift"

  "$(access_bin)" access team import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_TEAM_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --yes >/dev/null

  team_json="$(
    "$(access_bin)" access team list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-team-replay \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].name')" == "rust-access-team-replay" ]] \
    || fail "rust access team import did not keep the replay team name"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].members | length')" == "1" ]] \
    || fail "rust access team import did not reduce the replay membership set"

  "$(access_bin)" access team delete \
    --url "${GRAFANA_URL}" \
    --insecure \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-team-replay \
    --yes >/dev/null

  "$(access_bin)" access team import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_TEAM_REPLAY_EXPORT_DIR}" \
    --replace-existing \
    --yes >/dev/null

  team_json="$(
    "$(access_bin)" access team list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-team-replay \
      --with-members \
      --json
  )"
  [[ "$(printf '%s' "${team_json}" | jq '[.[] | select(.name == "rust-access-team-replay")] | length')" == "1" ]] \
    || fail "rust access team import did not recreate the deleted team"
  [[ "$(printf '%s' "${team_json}" | jq -r '.[0].members | length')" == "1" ]] \
    || fail "rust access team recreate import did not preserve the replay membership"

  org_json="$(
    "$(access_bin)" access org add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-live-delete-target \
      --json
  )"
  [[ "$(printf '%s' "${org_json}" | jq -r '.[0].name')" == "rust-access-live-delete-target" ]] \
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

  org_replay_json="$(
    "$(access_bin)" access org add \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-org-replay \
      --json
  )"
  org_replay_id="$(printf '%s' "${org_replay_json}" | jq -r '.[0].id')"
  [[ -n "${org_replay_id}" ]] \
    || fail "rust access org add did not return a replay target id"

  "$(access_bin)" access user add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --org-id "${org_replay_id}" \
    --login rust-access-org-replay-user \
    --email rust-access-org-replay-user@example.com \
    --name "Rust Access Org Replay User" \
    --org-role Viewer \
    --password secret123 >/dev/null

  "$(access_bin)" access org export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${ACCESS_ORG_EXPORT_DIR}" \
    --with-users \
    --overwrite >/dev/null

  check_access_export_metadata_contract \
    "${ACCESS_ORG_EXPORT_DIR}" \
    "orgs.json" \
    "grafana-utils-access-org-export-index"

  "$(access_bin)" access org diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --diff-dir "${ACCESS_ORG_EXPORT_DIR}" >"${org_diff_same_log}"
  grep -q 'No org differences across ' "${org_diff_same_log}" \
    || fail "rust access org diff did not report same-state after export"

  jq '
    .records |= map(
      if .name == "rust-access-org-replay"
      then .users |= map(
        if .login == "rust-access-org-replay-user"
        then .orgRole = "Editor"
        else .
        end
      )
      else .
      end
    )
  ' "${ACCESS_ORG_EXPORT_DIR}/orgs.json" >"${org_mutated_bundle}"
  mv "${org_mutated_bundle}" "${ACCESS_ORG_EXPORT_DIR}/orgs.json"

  "$(access_bin)" access org diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --diff-dir "${ACCESS_ORG_EXPORT_DIR}" >"${org_diff_changed_log}"
  grep -q 'Diff different org rust-access-org-replay fields=users' "${org_diff_changed_log}" \
    || fail "rust access org diff did not report the mutated user-role drift"

  "$(access_bin)" access org import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_ORG_EXPORT_DIR}" \
    --replace-existing \
    --dry-run >"${org_import_dry_run_log}"
  # Dry-run does not inspect live org users, so the preview stays additive here.
  grep -q 'Would add org user rust-access-org-replay-user -> Editor in org rust-access-org-replay' \
    "${org_import_dry_run_log}" \
    || fail "rust access org dry-run import did not predict the expected user-role update"
  grep -q 'Import summary: processed=' "${org_import_dry_run_log}" \
    || fail "rust access org dry-run import did not print the expected summary"

  "$(access_bin)" access org import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_ORG_EXPORT_DIR}" \
    --replace-existing >/dev/null

  org_list_json="$(
    "$(access_bin)" access org list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-org-replay \
      --with-users \
      --json
  )"
  [[ "$(printf '%s' "${org_list_json}" | jq 'length')" == "1" ]] \
    || fail "rust access org import did not preserve the replay target org"
  jq -e '.[0].users | any(.login == "rust-access-org-replay-user" and .orgRole == "Editor")' \
    <<<"${org_list_json}" >/dev/null \
    || fail "rust access org import did not preserve the replay target user role"

  "$(access_bin)" access org diff \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --diff-dir "${ACCESS_ORG_EXPORT_DIR}" >"${org_diff_same_log}"
  grep -q 'No org differences across ' "${org_diff_same_log}" \
    || fail "rust access org diff did not return to same-state after import replay"

  "$(access_bin)" access org delete \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-org-replay \
    --yes >/dev/null

  "$(access_bin)" access org import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${ACCESS_ORG_EXPORT_DIR}" \
    --replace-existing >/dev/null

  org_list_json="$(
    "$(access_bin)" access org list \
      --url "${GRAFANA_URL}" \
      --basic-user "${GRAFANA_USER}" \
      --basic-password "${GRAFANA_PASSWORD}" \
      --name rust-access-org-replay \
      --with-users \
      --json
  )"
  [[ "$(printf '%s' "${org_list_json}" | jq 'length')" == "1" ]] \
    || fail "rust access org recreate import did not restore the replay target org"
  jq -e '.[0].users | any(.login == "rust-access-org-replay-user" and .orgRole == "Editor")' \
    <<<"${org_list_json}" >/dev/null \
    || fail "rust access org recreate import did not preserve the replay target user role"

  "$(access_bin)" access org delete \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --name rust-access-org-replay \
    --yes >/dev/null

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

  check_access_export_metadata_contract \
    "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" \
    "service-accounts.json" \
    "grafana-utils-access-service-account-export-index"
  service_account_record_count="$(
    jq -r '.records | length' "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json"
  )"

  "$(access_bin)" access service-account diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" >"${service_account_diff_same_log}"
  grep -q 'No service-account differences across ' "${service_account_diff_same_log}" \
    || fail "rust access service-account diff did not report same-state after export"

  jq '
    .records |= map(
      if .name == "rust-access-service-account"
      then .role = "Viewer" | .disabled = true
      else .
      end
    )
  ' "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json" >"${service_account_mutated_bundle}"
  mv "${service_account_mutated_bundle}" "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}/service-accounts.json"

  "$(access_bin)" access service-account diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" >"${service_account_diff_changed_log}"
  grep -q 'Diff different service-account rust-access-service-account fields=disabled,role' "${service_account_diff_changed_log}" \
    || fail "rust access service-account diff did not report the mutated role/disabled drift"

  "$(access_bin)" access service-account import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" \
    --replace-existing \
    --dry-run \
    --json >"${service_account_import_dry_run_json}"
  jq -e '
    (.summary.processed == ($record_count | tonumber))
    and (.summary.created == 0)
    and (.summary.updated == 1)
    and (.summary.skipped == (($record_count | tonumber) - 1))
    and (.rows | length == ($record_count | tonumber))
    and (.rows | any(.identity == "rust-access-service-account" and .action == "update" and .detail == "would update fields=role,disabled"))
  ' --arg record_count "${service_account_record_count}" "${service_account_import_dry_run_json}" >/dev/null \
    || fail "rust access service-account dry-run import did not predict the expected update"

  "$(access_bin)" access service-account import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" \
    --replace-existing >/dev/null

  service_account_json="$(
    "$(access_bin)" access service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --query rust-access-service-account \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[0].role')" == "Viewer" ]] \
    || fail "rust access service-account import did not update the exported role"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[0].disabled')" == "true" ]] \
    || fail "rust access service-account import did not update the exported disabled flag"

  "$(access_bin)" access service-account diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" >"${service_account_diff_same_log}"
  grep -q 'No service-account differences across ' "${service_account_diff_same_log}" \
    || fail "rust access service-account diff did not return to same-state after import replay"

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

  "$(access_bin)" access service-account import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ACCESS_SERVICE_ACCOUNT_EXPORT_DIR}" \
    --replace-existing >/dev/null

  service_account_json="$(
    "$(access_bin)" access service-account list \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --query rust-access-service-account \
      --json
  )"
  [[ "$(printf '%s' "${service_account_json}" | jq '[.[] | select(.name == "rust-access-service-account")] | length')" == "1" ]] \
    || fail "rust access service-account import did not recreate the deleted service account"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[0].role')" == "Viewer" ]] \
    || fail "rust access service-account recreate import did not preserve the exported role"
  [[ "$(printf '%s' "${service_account_json}" | jq -r '.[0].disabled')" == "true" ]] \
    || fail "rust access service-account recreate import did not preserve the exported disabled flag"
}

run_datasource_smoke() {
  local add_dry_run_log="${WORK_DIR}/datasource-add-dry-run.json"
  local delete_dry_run_log="${WORK_DIR}/datasource-delete-dry-run.json"
  local dry_run_log="${WORK_DIR}/datasource-import-dry-run.json"
  local routed_dry_run_log="${WORK_DIR}/datasource-routed-import-dry-run.json"
  local missing_org_dry_run_log="${WORK_DIR}/datasource-routed-missing-org-dry-run.json"
  local recreate_dry_run_log="${WORK_DIR}/datasource-routed-recreate-dry-run.json"
  local secret_uid="smoke-prometheus-secret"
  local secret_after_add=""
  local secret_after_modify=""
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

  "$(datasource_bin)" datasource add \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --uid "${secret_uid}" \
    --name "Smoke Prometheus Secret" \
    --type prometheus \
    --datasource-url "http://prometheus-secret.invalid" \
    --apply-supported-defaults \
    --basic-auth \
    --basic-auth-user "metrics-user" \
    --basic-auth-password "metrics-pass" \
    --http-header "X-Scope-OrgID=tenant-a" >/dev/null

  secret_after_add="$(api GET "/api/datasources/uid/${secret_uid}")"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.basicAuthUser')" == "metrics-user" ]] \
    || fail "datasource secret add did not persist basicAuthUser"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.jsonData.httpMethod')" == "POST" ]] \
    || fail "datasource secret add did not keep prometheus preset httpMethod"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.jsonData.httpHeaderName1')" == "X-Scope-OrgID" ]] \
    || fail "datasource secret add did not persist httpHeaderName1"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.secureJsonFields.basicAuthPassword')" == "true" ]] \
    || fail "datasource secret add did not mark basicAuthPassword as server-managed secret"
  [[ "$(printf '%s' "${secret_after_add}" | jq -r '.secureJsonFields.httpHeaderValue1')" == "true" ]] \
    || fail "datasource secret add did not mark httpHeaderValue1 as server-managed secret"

  "$(datasource_bin)" datasource modify \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --uid "${secret_uid}" \
    --basic-auth-password "override-pass" >/dev/null

  secret_after_modify="$(api GET "/api/datasources/uid/${secret_uid}")"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.basicAuthUser')" == "metrics-user" ]] \
    || fail "datasource secret modify did not preserve basicAuthUser"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.jsonData.httpHeaderName1')" == "X-Scope-OrgID" ]] \
    || fail "datasource secret modify did not preserve httpHeaderName1"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.secureJsonFields.basicAuthPassword')" == "true" ]] \
    || fail "datasource secret modify did not keep basicAuthPassword server-managed secret flag"
  [[ "$(printf '%s' "${secret_after_modify}" | jq -r '.secureJsonFields.httpHeaderValue1')" == "true" ]] \
    || fail "datasource secret modify unexpectedly cleared the existing httpHeaderValue1 secret flag"

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
    --uid smoke-prometheus-extra \
    --yes >/dev/null

  "$(datasource_bin)" datasource delete \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --uid "${secret_uid}" \
    --yes >/dev/null

  if api GET "/api/datasources" | jq -e '.[] | select(.uid == "smoke-prometheus-extra")' >/dev/null; then
    fail "datasource delete did not remove the smoke-prometheus-extra datasource"
  fi
  if api GET "/api/datasources" | jq -e --arg uid "${secret_uid}" '.[] | select(.uid == $uid)' >/dev/null; then
    fail "datasource delete did not remove the ${secret_uid} datasource"
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

  jq -e --arg org_id "${org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 1
    and .summary.missingOrgCount == 0
    and .summary.wouldCreateOrgCount == 0
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "exists")
    and (.orgs[0].targetOrgId | tostring) == $org_id
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "exists")
    and (.imports[0].targetOrgId | tostring) == $org_id
  ' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run existing-org matrix was not correct"
  jq -e '.orgs | any(.orgAction == "exists")' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not report an existing org"
  jq -e '.imports | any(.datasources[]?.uid == "org-two-prometheus")' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not preview the selected org datasource"
  jq -e --arg org_id "${org_two_id}" 'all(.orgs[]; (.sourceOrgId | tostring) == $org_id)' "${routed_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run did not filter unselected orgs"

  delete_org "${org_two_id}"

  "$(datasource_bin)" datasource import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${DATASOURCE_MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${org_two_id}" \
    --replace-existing \
    --dry-run \
    --json | tee "${missing_org_dry_run_log}" >/dev/null

  jq -e --arg org_id "${org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 0
    and .summary.missingOrgCount == 1
    and .summary.wouldCreateOrgCount == 0
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "missing")
    and (.orgs[0].targetOrgId == null)
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "missing")
    and (.imports[0].targetOrgId == null)
  ' "${missing_org_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run missing-org matrix was not correct"

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

  jq -e --arg org_id "${org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 0
    and .summary.missingOrgCount == 0
    and .summary.wouldCreateOrgCount == 1
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "would-create")
    and (.orgs[0].targetOrgId == null)
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "would-create")
    and (.imports[0].targetOrgId == null)
  ' "${recreate_dry_run_log}" >/dev/null \
    || fail "datasource routed dry-run would-create matrix was not correct"
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
  [[ "${recreated_org_id}" != "${org_two_id}" ]] || fail "datasource routed import reused deleted org id unexpectedly"
  api_org "${recreated_org_id}" GET "/api/datasources" | jq -e '.[] | select(.uid == "org-two-prometheus")' >/dev/null \
    || fail "datasource routed import did not restore the org-two datasource"
}

run_dashboard_smoke() {
  local diff_log="${WORK_DIR}/dashboard-diff.log"
  local dry_run_log="${WORK_DIR}/dashboard-import-dry-run.log"
  local routed_dry_run_log="${WORK_DIR}/dashboard-routed-import-dry-run.log"
  local missing_org_dry_run_log="${WORK_DIR}/dashboard-routed-missing-org-dry-run.log"
  local recreate_dry_run_log="${WORK_DIR}/dashboard-routed-recreate-dry-run.log"
  local multi_org_org_two_id=""
  local recreated_org_id=""
  local prompt_file

  "$(dashboard_bin)" dashboard export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DASHBOARD_EXPORT_DIR}" \
    --overwrite

  [[ -f "${DASHBOARD_EXPORT_DIR}/raw/index.json" ]] || fail "dashboard raw index was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/raw/export-metadata.json" ]] || fail "dashboard raw metadata was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/prompt/index.json" ]] || fail "dashboard prompt index was not written"
  [[ -f "${DASHBOARD_EXPORT_DIR}/prompt/export-metadata.json" ]] || fail "dashboard prompt metadata was not written"

  prompt_file=""
  while IFS= read -r candidate; do
    if jq -e '.uid == "smoke-dashboard"' "${candidate}" >/dev/null 2>&1; then
      prompt_file="${candidate}"
      break
    fi
  done < <(find "${DASHBOARD_EXPORT_DIR}/prompt" -type f -name '*.json' ! -name 'index.json' ! -name 'export-metadata.json' | sort)
  [[ -n "${prompt_file}" ]] || fail "dashboard prompt export did not produce a dashboard file"
  jq -e '.__inputs | length > 0' "${prompt_file}" >/dev/null \
    || fail "dashboard prompt export did not include __inputs"
  jq -e '.__inputs | map(.name) | any(startswith("DS_PROMETHEUS"))' "${prompt_file}" >/dev/null \
    || fail "dashboard prompt export did not rewrite datasource inputs"

  "$(dashboard_bin)" dashboard diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw"

  "$(dashboard_bin)" dashboard export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DASHBOARD_DRY_RUN_DIR}" \
    --overwrite \
    --dry-run

  [[ ! -e "${DASHBOARD_DRY_RUN_DIR}" ]] || fail "dashboard dry-run export created output files"

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run | tee "${dry_run_log}" >/dev/null
  grep -Eq 'Dry-run checked 1 dashboard\(s\)' "${dry_run_log}" || fail "dashboard dry-run import summary was not printed"

  seed_dashboard "Smoke Dashboard Drifted"
  if "$(dashboard_bin)" dashboard diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" >"${diff_log}" 2>&1; then
    fail "dashboard diff should have failed after live drift"
  fi
  grep -q 'Dashboard diff found 1 differing item(s).' "${diff_log}" || fail "dashboard diff drift summary was not printed"

  api DELETE "/api/dashboards/uid/smoke-dashboard" >/dev/null

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --replace-existing >/dev/null

  api GET "/api/dashboards/uid/smoke-dashboard" | grep -q '"uid":"smoke-dashboard"' \
    || fail "dashboard import did not recreate the exported dashboard"

  multi_org_org_two_id="$(create_org "Org Two")"
  [[ -n "${multi_org_org_two_id}" ]] || fail "failed to create Org Two for routed import smoke"
  seed_dashboard "Org Two Smoke Dashboard" "org-two-smoke-dashboard" "${multi_org_org_two_id}"

  "$(dashboard_bin)" dashboard export \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --export-dir "${MULTI_ORG_EXPORT_DIR}" \
    --overwrite \
    --all-orgs \
    --without-dashboard-prompt >/dev/null

  [[ -d "${MULTI_ORG_EXPORT_DIR}/org_1_Main_Org/raw" ]] || fail "multi-org export did not include org 1 raw export"
  [[ -d "${MULTI_ORG_EXPORT_DIR}/org_${multi_org_org_two_id}_Org_Two/raw" ]] || fail "multi-org export did not include org 2 raw export"

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --dry-run \
    --json | tee "${routed_dry_run_log}" >/dev/null
  jq -e --arg org_id "${multi_org_org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 1
    and .summary.missingOrgCount == 0
    and .summary.wouldCreateOrgCount == 0
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "exists")
    and (.orgs[0].targetOrgId | tostring) == $org_id
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "exists")
    and (.imports[0].targetOrgId | tostring) == $org_id
  ' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run existing-org matrix was not correct"
  jq -e '.orgs | any(.orgAction == "exists")' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not report an existing org"
  jq -e '.imports | any(.dashboards[]?.uid == "org-two-smoke-dashboard")' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not preview the selected org dashboard"
  jq -e --arg org_id "${multi_org_org_two_id}" 'all(.orgs[]; (.sourceOrgId | tostring) == $org_id)' "${routed_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not filter unselected orgs"

  delete_org "${multi_org_org_two_id}"

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --dry-run \
    --json | tee "${missing_org_dry_run_log}" >/dev/null
  jq -e --arg org_id "${multi_org_org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 0
    and .summary.missingOrgCount == 1
    and .summary.wouldCreateOrgCount == 0
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "missing")
    and (.orgs[0].targetOrgId == null)
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "missing")
    and (.imports[0].targetOrgId == null)
  ' "${missing_org_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run missing-org matrix was not correct"

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --create-missing-orgs \
    --dry-run \
    --json | tee "${recreate_dry_run_log}" >/dev/null
  jq -e --arg org_id "${multi_org_org_two_id}" '
    .summary.orgCount == 1
    and .summary.existingOrgCount == 0
    and .summary.missingOrgCount == 0
    and .summary.wouldCreateOrgCount == 1
    and (.orgs | length == 1)
    and (.imports | length == 1)
    and (.orgs[0].sourceOrgId | tostring) == $org_id
    and (.orgs[0].orgAction == "would-create")
    and (.orgs[0].targetOrgId == null)
    and (.imports[0].sourceOrgId | tostring) == $org_id
    and (.imports[0].orgAction == "would-create")
    and (.imports[0].targetOrgId == null)
  ' "${recreate_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run would-create matrix was not correct"
  jq -e '.orgs | any(.orgAction == "would-create")' "${recreate_dry_run_log}" >/dev/null \
    || fail "routed dashboard dry-run did not preview missing-org creation"

  "$(dashboard_bin)" dashboard import \
    --url "${GRAFANA_URL}" \
    --basic-user "${GRAFANA_USER}" \
    --basic-password "${GRAFANA_PASSWORD}" \
    --import-dir "${MULTI_ORG_EXPORT_DIR}" \
    --use-export-org \
    --only-org-id "${multi_org_org_two_id}" \
    --create-missing-orgs >/dev/null

  recreated_org_id="$(find_org_id_by_name "Org Two")"
  [[ -n "${recreated_org_id}" ]] || fail "routed dashboard import did not recreate Org Two"
  [[ "${recreated_org_id}" != "${multi_org_org_two_id}" ]] || fail "routed dashboard import reused deleted org id unexpectedly"
  api_org "${recreated_org_id}" GET "/api/dashboards/uid/org-two-smoke-dashboard" | grep -q '"uid":"org-two-smoke-dashboard"' \
    || fail "routed dashboard import did not restore the org-two dashboard"
}

run_dashboard_authoring_smoke() {
  local stdin_review_json="${WORK_DIR}/dashboard-authoring-review.json"
  local stdin_patch_output="${WORK_DIR}/dashboard-authoring-patched.json"
  local stdin_publish_uid="authoring-stdin-smoke"
  local stdin_publish_title="Dashboard Authoring STDIN Smoke"
  local stdin_publish_log="${WORK_DIR}/dashboard-authoring-publish.log"
  local watch_input="${WORK_DIR}/dashboard-authoring-watch.json"
  local watch_log="${WORK_DIR}/dashboard-authoring-watch.log"
  local watch_pid=""

  printf '%s\n' \
    '{"uid":"authoring-review-smoke","title":"Dashboard Authoring Review","schemaVersion":38,"panels":[]}' \
    | "$(dashboard_bin)" dashboard review \
        --input - \
        --output-format json >"${stdin_review_json}"
  jq -e '
    (.summary.inputFile == "<stdin>")
    and (.summary.documentKind == "bare")
    and (.summary.title == "Dashboard Authoring Review")
    and (.blockingIssues | length == 0)
  ' "${stdin_review_json}" >/dev/null \
    || fail "dashboard review --input - did not emit the expected stdin summary"

  printf '%s\n' \
    '{"uid":"authoring-patch-smoke","title":"Dashboard Patch","schemaVersion":38,"panels":[]}' \
    | "$(dashboard_bin)" dashboard patch-file \
        --input - \
        --output "${stdin_patch_output}" \
        --folder-uid general >/dev/null
  [[ -f "${stdin_patch_output}" ]] || fail "dashboard patch-file --input - did not write an output file"
  jq -e '
    .uid == "authoring-patch-smoke"
    and .title == "Dashboard Patch"
    and .meta.folderUid == "general"
  ' "${stdin_patch_output}" >/dev/null \
    || fail "dashboard patch-file --input - did not retain the authored draft content"

  if printf '%s\n' \
    '{"uid":"authoring-patch-smoke","title":"Dashboard Patch","schemaVersion":38,"panels":[]}' \
    | "$(dashboard_bin)" dashboard patch-file --input - --folder-uid general >/dev/null 2>"${WORK_DIR}/dashboard-authoring-patch-error.log"; then
    fail "dashboard patch-file --input - should reject in-place stdin patching"
  fi
  grep -q 'patch-file --input - requires --output' "${WORK_DIR}/dashboard-authoring-patch-error.log" \
    || fail "dashboard patch-file stdin guardrail did not print the expected error"

  if printf '%s\n' \
    '{"uid":"authoring-watch-stdin","title":"Dashboard Watch STDIN","schemaVersion":38,"panels":[]}' \
    | "$(dashboard_bin)" dashboard publish \
        --url "${GRAFANA_URL}" \
        --token "${GRAFANA_API_TOKEN}" \
        --input - \
        --dry-run \
        --watch >/dev/null 2>"${WORK_DIR}/dashboard-authoring-watch-stdin-error.log"; then
    fail "dashboard publish --watch should reject stdin input"
  fi
  grep -q -- '--watch cannot be combined with --input -' "${WORK_DIR}/dashboard-authoring-watch-stdin-error.log" \
    || fail "dashboard publish --watch stdin guardrail did not print the expected error"

  printf '%s\n' \
    "{\"uid\":\"${stdin_publish_uid}\",\"title\":\"${stdin_publish_title}\",\"schemaVersion\":38,\"panels\":[]}" \
    | "$(dashboard_bin)" dashboard publish \
        --url "${GRAFANA_URL}" \
        --token "${GRAFANA_API_TOKEN}" \
        --input - \
        --replace-existing >"${stdin_publish_log}"
  grep -q 'Import mode: create-or-update' "${stdin_publish_log}" \
    || fail "dashboard publish --input - did not run through the import pipeline"
  api GET "/api/dashboards/uid/${stdin_publish_uid}" | jq -e --arg uid "${stdin_publish_uid}" --arg title "${stdin_publish_title}" '
    .dashboard.uid == $uid and .dashboard.title == $title and .meta.folderTitle == "General"
  ' >/dev/null \
    || fail "dashboard publish --input - did not create the live dashboard in General"

  printf '%s\n' \
    '{"uid":"authoring-watch-smoke","title":"Dashboard Watch 1","schemaVersion":38,"panels":[]}' >"${watch_input}"
  ("$(dashboard_bin)" dashboard publish \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --input "${watch_input}" \
      --dry-run \
      --watch >"${watch_log}" 2>&1) &
  watch_pid=$!
  sleep 2

  printf '%s\n' '{"uid":"authoring-watch-smoke","title":"BROKEN"' >"${watch_input}"
  sleep 2
  printf '%s\n' \
    '{"uid":"authoring-watch-smoke","title":"Dashboard Watch 2","schemaVersion":38,"panels":[]}' >"${watch_input}"
  sleep 3

  grep -q 'Press Ctrl-C to stop' "${watch_log}" \
    || fail "dashboard publish --watch did not advertise how to stop the watcher"
  grep -q 'Detected dashboard input change' "${watch_log}" \
    || fail "dashboard publish --watch did not report file change detection"
  grep -q 'Dashboard publish failed for ' "${watch_log}" \
    || fail "dashboard publish --watch did not surface the transient JSON failure"
  grep -q 'Re-ran dashboard publish for ' "${watch_log}" \
    || fail "dashboard publish --watch did not recover after the file was fixed"

  kill "${watch_pid}" >/dev/null 2>&1 || true
  wait "${watch_pid}" >/dev/null 2>&1 || true

  api DELETE "/api/dashboards/uid/${stdin_publish_uid}" >/dev/null
}

run_dashboard_inspection_smoke() {
  local inspect_export_report_json="${WORK_DIR}/dashboard-inspect-export-report.json"
  local inspect_export_governance_json="${WORK_DIR}/dashboard-inspect-export-governance.json"
  local inspect_export_filter_json="${WORK_DIR}/dashboard-inspect-export-filter.json"
  local inspect_live_report_json="${WORK_DIR}/dashboard-inspect-live-report.json"
  local inspect_live_governance_json="${WORK_DIR}/dashboard-inspect-live-governance.json"

  seed_datasource_typed "" "Smoke Loki" "smoke-loki" "loki" "http://loki.invalid"
  seed_datasource_typed "" "Smoke Influx" "smoke-influx" "influxdb" "http://influxdb.invalid"
  seed_datasource_typed "" "Smoke Postgres" "smoke-postgres" "postgres" "http://postgres.invalid:5432"
  seed_datasource_typed "" "Smoke Search" "smoke-search" "elasticsearch" "http://elasticsearch.invalid:9200"
  seed_datasource_typed "" "Smoke Tempo" "smoke-tempo" "tempo" "http://tempo.invalid:3200"
  seed_core_family_inspection_dashboard

  "$(dashboard_bin)" dashboard export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --export-dir "${DASHBOARD_INSPECTION_EXPORT_DIR}" \
    --overwrite \
    --without-dashboard-prompt >/dev/null

  "$(dashboard_bin)" dashboard inspect-export \
    --import-dir "${DASHBOARD_INSPECTION_EXPORT_DIR}/raw" \
    --output-format report-json >"${inspect_export_report_json}"
  jq -e '.summary.queryRecordCount >= 7' "${inspect_export_report_json}" >/dev/null \
    || fail "dashboard inspect-export report-json did not emit the expected query rows"
  for family in prometheus loki flux sql search tracing; do
    jq -e --arg family "${family}" \
      '.queries | any(.dashboardUid == "inspect-core-families" and .datasourceFamily == $family)' \
      "${inspect_export_report_json}" >/dev/null \
      || fail "dashboard inspect-export report-json did not retain ${family} family coverage"
  done

  "$(dashboard_bin)" dashboard inspect-export \
    --import-dir "${DASHBOARD_INSPECTION_EXPORT_DIR}/raw" \
    --output-format governance-json >"${inspect_export_governance_json}"
  jq -e '.summary.datasourceFamilyCount >= 6' "${inspect_export_governance_json}" >/dev/null \
    || fail "dashboard inspect-export governance-json did not summarize the expected datasource families"
  jq -e \
    '.dashboardDependencies | any(.dashboardUid == "inspect-core-families" and (.datasourceFamilies | index("search")) and (.datasourceFamilies | index("tracing")))' \
    "${inspect_export_governance_json}" >/dev/null \
    || fail "dashboard inspect-export governance-json did not retain search/tracing family dependencies"

  "$(dashboard_bin)" dashboard inspect-export \
    --import-dir "${DASHBOARD_INSPECTION_EXPORT_DIR}/raw" \
    --output-format report-json \
    --report-filter-datasource tracing >"${inspect_export_filter_json}"
  jq -e '(.queries | length >= 1) and (.queries | all(.datasourceFamily == "tracing"))' \
    "${inspect_export_filter_json}" >/dev/null \
    || fail "dashboard inspect-export datasource-family filter did not narrow to tracing rows"

  "$(dashboard_bin)" dashboard inspect-live \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-format report-json >"${inspect_live_report_json}"
  for family in prometheus loki flux sql search tracing; do
    jq -e --arg family "${family}" \
      '.queries | any(.dashboardUid == "inspect-core-families" and .datasourceFamily == $family)' \
      "${inspect_live_report_json}" >/dev/null \
      || fail "dashboard inspect-live report-json did not retain ${family} family coverage"
  done

  "$(dashboard_bin)" dashboard inspect-live \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-format governance-json >"${inspect_live_governance_json}"
  jq -e '.summary.datasourceFamilyCount >= 6' "${inspect_live_governance_json}" >/dev/null \
    || fail "dashboard inspect-live governance-json did not summarize the expected datasource families"
  jq -e \
    '.dashboardDependencies | any(.dashboardUid == "inspect-core-families" and (.datasourceFamilies | index("search")) and (.datasourceFamilies | index("tracing")))' \
    "${inspect_live_governance_json}" >/dev/null \
    || fail "dashboard inspect-live governance-json did not retain search/tracing family dependencies"

  compare_normalized_report_json "dashboard-inspection-report-parity" \
    "${inspect_export_report_json}" \
    "${inspect_live_report_json}"
  compare_normalized_governance_json "dashboard-inspection-governance-parity" \
    "${inspect_export_governance_json}" \
    "${inspect_live_governance_json}"
}

run_alert_artifact_smoke() {
  seed_alert_resources

  "$(alert_bin)" \
    alert export \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-dir "${ALERT_EXPORT_DIR}" \
    --overwrite >/dev/null

  check_alert_export_contract "${ALERT_EXPORT_DIR}"
  ALERT_CONTACT_FILE="$(find "${ALERT_EXPORT_DIR}/raw/contact-points" -type f -name '*Smoke_Webhook*.json' | head -n 1)"
  [[ -n "${ALERT_CONTACT_FILE}" ]] || fail "alert export did not write the seeded contact point"
  ALERT_CONTACT_UID="$(jq -r '.spec.uid // empty' "${ALERT_CONTACT_FILE}")"
  [[ -n "${ALERT_CONTACT_UID}" ]] || fail "alert export contact point document did not retain uid"
  ALERT_MUTE_TIMING_FILE="$(find "${ALERT_EXPORT_DIR}/raw/mute-timings" -type f -name 'Off_Hours*.json' | head -n 1)"
  [[ -n "${ALERT_MUTE_TIMING_FILE}" ]] || fail "alert export did not write the seeded mute timing"
  ALERT_MUTE_TIMING_NAME="$(jq -r '.spec.name // empty' "${ALERT_MUTE_TIMING_FILE}")"
  [[ "${ALERT_MUTE_TIMING_NAME}" == "Off Hours" ]] || fail "alert export mute timing document did not retain name"
  ALERT_TEMPLATE_FILE="$(find "${ALERT_EXPORT_DIR}/raw/templates" -type f -name 'slack.default*.json' | head -n 1)"
  [[ -n "${ALERT_TEMPLATE_FILE}" ]] || fail "alert export did not write the seeded template"
  ALERT_TEMPLATE_NAME="$(jq -r '.spec.name // empty' "${ALERT_TEMPLATE_FILE}")"
  [[ "${ALERT_TEMPLATE_NAME}" == "slack.default" ]] || fail "alert export template document did not retain name"
  ALERT_RULE_FILE="$(find "${ALERT_EXPORT_DIR}/raw/rules" -type f -name '*cpu-high*.json' | head -n 1)"
  [[ -n "${ALERT_RULE_FILE}" ]] || fail "alert export did not write the seeded alert rule"
  ALERT_RULE_UID="$(jq -r '.spec.uid // empty' "${ALERT_RULE_FILE}")"
  [[ "${ALERT_RULE_UID}" == "cpu-high" ]] || fail "alert export alert rule document did not retain uid"
}

run_alert_replay_smoke() {
  local diff_same_log="${WORK_DIR}/alert-diff-same.log"
  local diff_changed_log="${WORK_DIR}/alert-diff-changed.log"
  local diff_missing_log="${WORK_DIR}/alert-diff-missing.log"
  local dry_run_json="${WORK_DIR}/alert-import-dry-run.json"
  local recreate_dry_run_json="${WORK_DIR}/alert-import-recreate-dry-run.json"

  if [[ -z "${ALERT_CONTACT_FILE:-}" || -z "${ALERT_CONTACT_UID:-}" ]]; then
    run_alert_artifact_smoke
  fi

  "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_same_log}"
  grep -q 'No alerting differences across ' "${diff_same_log}" \
    || fail "alert diff did not report same-state after export"

  rewrite_contact_point_url "${ALERT_CONTACT_FILE}" "http://127.0.0.1/updated"
  rewrite_mute_timing_end_time "${ALERT_MUTE_TIMING_FILE}" "07:00"
  rewrite_template_body "${ALERT_TEMPLATE_FILE}" '{{ define "slack.default" }}updated{{ end }}'
  rewrite_alert_rule_title "${ALERT_RULE_FILE}" "CPU High Updated"

  if "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_changed_log}" 2>&1; then
    fail "alert diff should have failed after local drift"
  fi
  grep -q 'Diff different' "${diff_changed_log}" || fail "alert diff did not report a changed resource"

  "$(alert_bin)" \
    alert import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run \
    --json >"${dry_run_json}"
  jq -e '
    (.summary.processed >= 2)
    and (.summary.wouldUpdate >= 4)
    and (.rows | any(.kind == "grafana-contact-point" and .identity == "smoke-webhook" and .action == "would-update"))
    and (.rows | any(.kind == "grafana-mute-timing" and .identity == "Off Hours" and .action == "would-update"))
    and (.rows | any(.kind == "grafana-notification-template" and .identity == "slack.default" and .action == "would-update"))
    and (.rows | any(.kind == "grafana-alert-rule" and .identity == "cpu-high" and .action == "would-update"))
  ' "${dry_run_json}" >/dev/null \
    || fail "alert dry-run import did not emit the expected structured update preview"

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
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_same_log}"
  grep -q 'No alerting differences across ' "${diff_same_log}" \
    || fail "alert diff did not return to same-state after replay"

  api DELETE "/api/v1/provisioning/contact-points/${ALERT_CONTACT_UID}" >/dev/null

  if "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_missing_log}" 2>&1; then
    fail "alert diff should have failed after remote contact-point deletion"
  fi
  grep -q 'Diff missing-remote' "${diff_missing_log}" \
    || fail "alert diff did not report missing-remote after contact-point deletion"

  "$(alert_bin)" \
    alert import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing \
    --dry-run \
    --json >"${recreate_dry_run_json}"
  jq -e '
    (.summary.processed >= 2)
    and (.summary.wouldCreate >= 1)
    and (.rows | any(.kind == "grafana-contact-point" and .identity == "smoke-webhook" and .action == "would-create"))
  ' "${recreate_dry_run_json}" >/dev/null \
    || fail "alert dry-run import did not preview contact-point recreation"

  "$(alert_bin)" \
    alert import \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --import-dir "${ALERT_EXPORT_DIR}/raw" \
    --replace-existing >/dev/null

  api GET "/api/v1/provisioning/contact-points" | jq -e 'any(.uid == "smoke-webhook")' >/dev/null \
    || fail "alert import did not recreate the deleted contact point"
  api GET "/api/v1/provisioning/mute-timings" | jq -e 'any(.name == "Off Hours")' >/dev/null \
    || fail "alert import did not preserve the seeded mute timing after replay"
  api GET "/api/v1/provisioning/templates/slack.default" | jq -e '.name == "slack.default"' >/dev/null \
    || fail "alert import did not preserve the seeded template after replay"
  api GET "/api/v1/provisioning/alert-rules/cpu-high" | jq -e '.uid == "cpu-high"' >/dev/null \
    || fail "alert import did not preserve the seeded alert rule after replay"

  "$(alert_bin)" \
    alert diff \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --diff-dir "${ALERT_EXPORT_DIR}/raw" >"${diff_same_log}"
  grep -q 'No alerting differences across ' "${diff_same_log}" \
    || fail "alert diff did not return to same-state after recreate import"
}

run_alert_authoring_smoke() {
  local desired_dir="${WORK_DIR}/alert-authoring-desired"
  local preview_json="${WORK_DIR}/alert-authoring-preview-route.json"
  local plan_create_json="${WORK_DIR}/alert-authoring-plan-create.json"
  local apply_create_json="${WORK_DIR}/alert-authoring-apply-create.json"
  local plan_post_apply_json="${WORK_DIR}/alert-authoring-plan-post-apply.json"
  local plan_delete_json="${WORK_DIR}/alert-authoring-plan-delete.json"
  local apply_delete_json="${WORK_DIR}/alert-authoring-apply-delete.json"
  local authoring_contact_name="authoring-webhook"
  local authoring_rule_name="authoring-cpu-high"

  seed_alert_resources
  rm -rf "${desired_dir}"

  "$(alert_bin)" \
    alert init \
    --desired-dir "${desired_dir}" >/dev/null

  "$(alert_bin)" \
    alert add-contact-point \
    --desired-dir "${desired_dir}" \
    --name "${authoring_contact_name}" >/dev/null

  "$(alert_bin)" \
    alert add-rule \
    --desired-dir "${desired_dir}" \
    --name "${authoring_rule_name}" \
    --folder smoke-alerts \
    --rule-group "CPU Alerts" \
    --receiver "${authoring_contact_name}" \
    --label team=platform \
    --severity critical \
    --expr A \
    --threshold 80 \
    --above \
    --for 5m >/dev/null

  [[ -f "${desired_dir}/contact-points/${authoring_contact_name}.yaml" ]] \
    || fail "alert authoring smoke did not write the contact-point desired file"
  [[ -f "${desired_dir}/rules/${authoring_rule_name}.yaml" ]] \
    || fail "alert authoring smoke did not write the alert-rule desired file"
  [[ -f "${desired_dir}/policies/notification-policies.yaml" ]] \
    || fail "alert authoring smoke did not write the managed policy desired file"

  "$(alert_bin)" \
    alert preview-route \
    --desired-dir "${desired_dir}" \
    --label team=platform \
    --severity critical >"${preview_json}"
  jq -e '
    (.input.labels.team == "platform")
    and (.input.severity == "critical")
    and ((.matches | type) == "array")
  ' "${preview_json}" >/dev/null \
    || fail "alert preview-route did not emit the expected desired-state preview"

  "$(alert_bin)" \
    alert plan \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --desired-dir "${desired_dir}" \
    --output-format json >"${plan_create_json}"
  jq -e --arg cp "${authoring_contact_name}" --arg rule "${authoring_rule_name}" '
    (.rows | any(.kind == "grafana-contact-point" and .identity == $cp and .action == "create"))
    and (.rows | any(.kind == "grafana-alert-rule" and .identity == $rule and .action == "create"))
    and (.rows | any(.kind == "grafana-notification-policies" and .action == "update"))
  ' "${plan_create_json}" >/dev/null \
    || fail "alert authoring plan did not emit the expected create/update rows"

  "$(alert_bin)" \
    alert apply \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --plan-file "${plan_create_json}" \
    --approve \
    --output-format json >"${apply_create_json}"
  jq -e --arg cp "${authoring_contact_name}" --arg rule "${authoring_rule_name}" '
    (.appliedCount >= 3)
    and (.results | any(.kind == "grafana-contact-point" and .identity == $cp and .action == "create"))
    and (.results | any(.kind == "grafana-notification-policies" and .action == "update"))
    and (.results | any(.kind == "grafana-alert-rule" and .identity == $rule and .action == "create"))
  ' "${apply_create_json}" >/dev/null \
    || fail "alert authoring apply did not emit the expected create/update results"

  api GET "/api/v1/provisioning/contact-points" | jq -e --arg uid "${authoring_contact_name}" 'any(.uid == $uid)' >/dev/null \
    || fail "alert authoring apply did not create the contact point in Grafana"
  api GET "/api/v1/provisioning/alert-rules/${authoring_rule_name}" | jq -e --arg uid "${authoring_rule_name}" '.uid == $uid' >/dev/null \
    || fail "alert authoring apply did not create the alert rule in Grafana"

  "$(alert_bin)" \
    alert plan \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --desired-dir "${desired_dir}" \
    --output-format json >"${plan_post_apply_json}"
  jq -e --arg cp "${authoring_contact_name}" --arg rule "${authoring_rule_name}" '
    (.rows | any(.kind == "grafana-contact-point" and .identity == $cp and .action == "noop"))
    and (.rows | any(.kind == "grafana-notification-policies" and .identity == $cp and .action == "noop"))
    and (.rows | any(.kind == "grafana-alert-rule" and .identity == $rule and .action == "noop"))
  ' "${plan_post_apply_json}" >/dev/null \
    || fail "alert authoring plan did not return the authored resources to noop after apply"

  rm -f "${desired_dir}/rules/${authoring_rule_name}.yaml"

  "$(alert_bin)" \
    alert plan \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --desired-dir "${desired_dir}" \
    --prune \
    --output-format json >"${plan_delete_json}"
  jq -e --arg rule "${authoring_rule_name}" '
    (.rows | any(.kind == "grafana-alert-rule" and .identity == $rule and .action == "delete" and .reason == "missing-from-desired-state"))
  ' "${plan_delete_json}" >/dev/null \
    || fail "alert authoring prune plan did not emit the expected delete row"

  "$(alert_bin)" \
    alert apply \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --plan-file "${plan_delete_json}" \
    --approve \
    --output-format json >"${apply_delete_json}"
  jq -e --arg rule "${authoring_rule_name}" '
    (.results | any(.kind == "grafana-alert-rule" and .identity == $rule and .action == "delete"))
  ' "${apply_delete_json}" >/dev/null \
    || fail "alert authoring prune apply did not delete the authored alert rule"

  if api GET "/api/v1/provisioning/alert-rules/${authoring_rule_name}" >/dev/null 2>&1; then
    fail "alert authoring prune apply did not remove the authored alert rule from Grafana"
  fi
}

run_alert_smoke() {
  run_alert_artifact_smoke
  run_alert_replay_smoke
  run_alert_authoring_smoke
}

prepare_sync_smoke_fixture() {
  if [[ ! -d "${DASHBOARD_EXPORT_DIR}/raw" ]]; then
    seed_datasource
    seed_dashboard "Smoke Dashboard"
    "$(dashboard_bin)" dashboard export \
      --url "${GRAFANA_URL}" \
      --token "${GRAFANA_API_TOKEN}" \
      --export-dir "${DASHBOARD_EXPORT_DIR}" \
      --overwrite \
      --without-dashboard-prompt >/dev/null
  fi

  if [[ ! -d "${ALERT_EXPORT_DIR}/raw" ]]; then
    run_alert_artifact_smoke
  fi
}

run_sync_smoke() {
  prepare_sync_smoke_fixture

  local change_preview_fetch_live_json="${WORK_DIR}/change-preview-fetch-live.json"

  "$(sync_bin)" change advanced bundle \
    --dashboard-export-dir "${DASHBOARD_EXPORT_DIR}/raw" \
    --alert-export-dir "${ALERT_EXPORT_DIR}/raw" \
    --output-file "${SYNC_BUNDLE_FILE}" \
    --output-format json >/dev/null

  [[ -f "${SYNC_BUNDLE_FILE}" ]] || fail "sync bundle did not write source bundle output"
  jq -e '.kind == "grafana-utils-sync-source-bundle"' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not emit the expected source bundle kind"
  jq -e '.summary.contactPointCount >= 1' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not record exported alert contact point count"
  jq -e '.alerts | any(.uid == "cpu-high")' "${SYNC_BUNDLE_FILE}" >/dev/null \
    || fail "sync bundle did not preserve the seeded alert rule in the smoke fixture"

  printf '{}\n' >"${SYNC_TARGET_INVENTORY_FILE}"

  "$(sync_bin)" change advanced bundle-preflight \
    --source-bundle "${SYNC_BUNDLE_FILE}" \
    --target-inventory "${SYNC_TARGET_INVENTORY_FILE}" \
    --output-format json >"${SYNC_BUNDLE_PREFLIGHT_FILE}"

  jq -e '.kind == "grafana-utils-sync-bundle-preflight"' "${SYNC_BUNDLE_PREFLIGHT_FILE}" >/dev/null \
    || fail "sync bundle-preflight did not emit the expected document kind"
  jq -e '.summary.resourceCount >= 2' "${SYNC_BUNDLE_PREFLIGHT_FILE}" >/dev/null \
    || fail "sync bundle-preflight did not count the bundled dashboard and datasource specs"

  "$(sync_bin)" change preview \
    --workspace "${WORK_DIR}" \
    --fetch-live \
    --url "${GRAFANA_URL}" \
    --token "${GRAFANA_API_TOKEN}" \
    --output-format json \
    --output-file "${change_preview_fetch_live_json}" >/dev/null

  [[ -f "${change_preview_fetch_live_json}" ]] || fail "change preview did not write a live preview artifact"
  jq -e '
    (.kind == "grafana-utils-sync-plan")
    and (.reviewed == false)
    and (.ordering.mode == "dependency-aware")
    and ((.operations | length) >= 1)
  ' "${change_preview_fetch_live_json}" >/dev/null \
    || fail "change preview --fetch-live did not emit the expected sync plan contract"
}

main() {
  command -v docker >/dev/null || fail "docker is required"
  command -v curl >/dev/null || fail "curl is required"
  command -v jq >/dev/null || fail "jq is required"

  build_rust_bins
  start_grafana
  seed_contact_point
  create_api_token

  if [[ "${RUST_LIVE_SCOPE}" == "alert-artifact" ]]; then
    run_alert_artifact_smoke
    printf 'Rust alert artifact live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
    return
  fi

  if [[ "${RUST_LIVE_SCOPE}" == "alert-replay" ]]; then
    run_alert_replay_smoke
    printf 'Rust alert replay live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
    return
  fi

  if [[ "${RUST_LIVE_SCOPE}" == "alert" ]]; then
    run_alert_smoke
    printf 'Rust alert live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
    return
  fi

  if [[ "${RUST_LIVE_SCOPE}" == "sync" ]]; then
    run_sync_smoke
    printf 'Rust sync live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
    return
  fi

  seed_datasource
  seed_dashboard "Smoke Dashboard"
  run_access_smoke
  run_dashboard_smoke
  run_dashboard_authoring_smoke
  run_dashboard_inspection_smoke
  run_alert_smoke
  run_sync_smoke
  run_datasource_smoke
  printf 'Rust live Grafana smoke test passed against %s using %s\n' "${GRAFANA_URL}" "${GRAFANA_IMAGE}"
}

main "$@"
