"""Unwired dashboard/folder permission contract helpers.

This module stages ACL export/diff data shapes for dashboard and folder
permissions without wiring them into the current live CLI paths.
"""

from typing import Any


PERMISSION_EXPORT_KIND = "grafana-utils-dashboard-permission-export"
PERMISSION_EXPORT_SCHEMA_VERSION = 1
PERMISSION_DIFF_KIND = "grafana-utils-dashboard-permission-diff"
PERMISSION_DIFF_SCHEMA_VERSION = 1
PERMISSION_PREFLIGHT_KIND = "grafana-utils-dashboard-permission-preflight"
PERMISSION_PREFLIGHT_SCHEMA_VERSION = 1
PERMISSION_PROMOTION_KIND = "grafana-utils-dashboard-permission-promotion"
PERMISSION_PROMOTION_SCHEMA_VERSION = 1
PERMISSION_BUNDLE_KIND = "grafana-utils-dashboard-permission-bundle"
PERMISSION_BUNDLE_SCHEMA_VERSION = 1
PERMISSION_BUNDLE_DIFF_KIND = "grafana-utils-dashboard-permission-bundle-diff"
PERMISSION_BUNDLE_DIFF_SCHEMA_VERSION = 1
PERMISSION_REMAP_KIND = "grafana-utils-dashboard-permission-remap"
PERMISSION_REMAP_SCHEMA_VERSION = 1

PERMISSION_LEVEL_LABELS = {
    1: "view",
    2: "edit",
    4: "admin",
}
PERMISSION_LEVEL_VALUES = {
    "view": 1,
    "edit": 2,
    "admin": 4,
}


def _normalize_text(value: Any, default: str = "") -> str:
    text = str(value or "").strip()
    if text:
        return text
    return default


def normalize_permission_level(value: Any) -> tuple[int, str]:
    """Normalize Grafana ACL levels into one stable numeric/string pair."""
    if isinstance(value, int):
        normalized = int(value)
    else:
        text = _normalize_text(value).lower()
        if text in PERMISSION_LEVEL_VALUES:
            normalized = PERMISSION_LEVEL_VALUES[text]
        else:
            try:
                normalized = int(text)
            except ValueError:
                normalized = 0
    return normalized, PERMISSION_LEVEL_LABELS.get(normalized, "unknown")


def normalize_permission_subject(record: dict[str, Any]) -> dict[str, str]:
    """Normalize one dashboard/folder permission subject identity."""
    user_id = _normalize_text(record.get("userId"))
    team_id = _normalize_text(record.get("teamId"))
    service_account_id = _normalize_text(
        record.get("serviceAccountId") or record.get("service_account_id")
    )
    role_name = _normalize_text(record.get("role") or record.get("roleName"))
    if user_id:
        return {
            "subjectType": "user",
            "subjectKey": "user:%s" % user_id,
            "subjectId": user_id,
            "subjectName": _normalize_text(
                record.get("userLogin") or record.get("userName") or record.get("login"),
                user_id,
            ),
        }
    if team_id:
        return {
            "subjectType": "team",
            "subjectKey": "team:%s" % team_id,
            "subjectId": team_id,
            "subjectName": _normalize_text(record.get("team") or record.get("teamName"), team_id),
        }
    if service_account_id:
        return {
            "subjectType": "service-account",
            "subjectKey": "service-account:%s" % service_account_id,
            "subjectId": service_account_id,
            "subjectName": _normalize_text(
                record.get("serviceAccount") or record.get("serviceAccountName"),
                service_account_id,
            ),
        }
    if role_name:
        return {
            "subjectType": "role",
            "subjectKey": "role:%s" % role_name,
            "subjectId": role_name,
            "subjectName": role_name,
        }
    return {
        "subjectType": "unknown",
        "subjectKey": "unknown",
        "subjectId": "",
        "subjectName": "unknown",
    }


def normalize_permission_record(
    resource_kind: str,
    resource_uid: str,
    resource_title: str,
    record: dict[str, Any],
) -> dict[str, Any]:
    """Normalize one raw Grafana ACL row into a stable export shape."""
    subject = normalize_permission_subject(record)
    level_value, level_name = normalize_permission_level(
        record.get("permission") or record.get("permissionName")
    )
    return {
        "resourceKind": _normalize_text(resource_kind, "dashboard"),
        "resourceUid": _normalize_text(resource_uid, "unknown"),
        "resourceTitle": _normalize_text(resource_title, "unknown"),
        "subjectType": subject["subjectType"],
        "subjectKey": subject["subjectKey"],
        "subjectId": subject["subjectId"],
        "subjectName": subject["subjectName"],
        "permission": level_value,
        "permissionName": level_name,
        "inherited": bool(record.get("inherited")),
    }


def build_permission_export_document(
    resource_kind: str,
    resource_uid: str,
    resource_title: str,
    permissions: list[dict[str, Any]],
) -> dict[str, Any]:
    """Build one staged permission export document for dashboard or folder ACLs."""
    rows = [
        normalize_permission_record(resource_kind, resource_uid, resource_title, item)
        for item in permissions
        if isinstance(item, dict)
    ]
    rows.sort(
        key=lambda item: (
            item["resourceKind"],
            item["resourceUid"],
            item["subjectType"],
            item["subjectName"],
            item["permission"],
        )
    )
    return {
        "kind": PERMISSION_EXPORT_KIND,
        "schemaVersion": PERMISSION_EXPORT_SCHEMA_VERSION,
        "resource": {
            "kind": _normalize_text(resource_kind, "dashboard"),
            "uid": _normalize_text(resource_uid, "unknown"),
            "title": _normalize_text(resource_title, "unknown"),
        },
        "summary": {
            "permissionCount": len(rows),
            "userCount": len([row for row in rows if row["subjectType"] == "user"]),
            "teamCount": len([row for row in rows if row["subjectType"] == "team"]),
            "serviceAccountCount": len(
                [row for row in rows if row["subjectType"] == "service-account"]
            ),
            "roleCount": len([row for row in rows if row["subjectType"] == "role"]),
        },
        "permissions": rows,
    }


def build_permission_diff_document(
    expected_document: dict[str, Any],
    actual_document: dict[str, Any],
) -> dict[str, Any]:
    """Compare two staged permission export documents."""
    expected_map = {
        row["subjectKey"]: row for row in expected_document.get("permissions") or [] if isinstance(row, dict)
    }
    actual_map = {
        row["subjectKey"]: row for row in actual_document.get("permissions") or [] if isinstance(row, dict)
    }
    diff_rows = []
    for key in sorted(set(expected_map) | set(actual_map)):
        expected = expected_map.get(key)
        actual = actual_map.get(key)
        if expected is None:
            diff_rows.append(
                {
                    "subjectKey": key,
                    "status": "extra-live",
                    "expectedPermissionName": "",
                    "actualPermissionName": actual.get("permissionName", ""),
                }
            )
            continue
        if actual is None:
            diff_rows.append(
                {
                    "subjectKey": key,
                    "status": "missing-live",
                    "expectedPermissionName": expected.get("permissionName", ""),
                    "actualPermissionName": "",
                }
            )
            continue
        if int(expected.get("permission") or 0) != int(actual.get("permission") or 0):
            diff_rows.append(
                {
                    "subjectKey": key,
                    "status": "changed",
                    "expectedPermissionName": expected.get("permissionName", ""),
                    "actualPermissionName": actual.get("permissionName", ""),
                }
            )
        else:
            diff_rows.append(
                {
                    "subjectKey": key,
                    "status": "same",
                    "expectedPermissionName": expected.get("permissionName", ""),
                    "actualPermissionName": actual.get("permissionName", ""),
                }
            )
    return {
        "kind": PERMISSION_DIFF_KIND,
        "schemaVersion": PERMISSION_DIFF_SCHEMA_VERSION,
        "summary": {
            "rowCount": len(diff_rows),
            "sameCount": len([row for row in diff_rows if row["status"] == "same"]),
            "changedCount": len([row for row in diff_rows if row["status"] == "changed"]),
            "missingLiveCount": len([row for row in diff_rows if row["status"] == "missing-live"]),
            "extraLiveCount": len([row for row in diff_rows if row["status"] == "extra-live"]),
        },
        "rows": diff_rows,
    }


def render_permission_export_text(document: dict[str, Any]) -> list[str]:
    """Render a staged ACL export document as text."""
    resource = document.get("resource") or {}
    summary = document.get("summary") or {}
    lines = [
        "Permission export: %s uid=%s title=%s"
        % (
            _normalize_text(resource.get("kind"), "dashboard"),
            _normalize_text(resource.get("uid"), "unknown"),
            _normalize_text(resource.get("title"), "unknown"),
        ),
        "Counts: %s permissions, %s users, %s teams, %s service-accounts, %s roles"
        % (
            int(summary.get("permissionCount") or 0),
            int(summary.get("userCount") or 0),
            int(summary.get("teamCount") or 0),
            int(summary.get("serviceAccountCount") or 0),
            int(summary.get("roleCount") or 0),
        ),
        "",
        "# Permissions",
    ]
    for row in document.get("permissions") or []:
        if not isinstance(row, dict):
            continue
        lines.append(
            "- %s %s permission=%s inherited=%s"
            % (
                _normalize_text(row.get("subjectType"), "unknown"),
                _normalize_text(row.get("subjectName"), "unknown"),
                _normalize_text(row.get("permissionName"), "unknown"),
                "true" if bool(row.get("inherited")) else "false",
            )
        )
    return lines


def build_permission_preflight_document(
    export_document: dict[str, Any],
    availability: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build reviewable availability checks for permission subjects."""
    availability = dict(availability or {})
    available_users = set(str(item) for item in (availability.get("userIds") or []))
    available_teams = set(str(item) for item in (availability.get("teamIds") or []))
    available_service_accounts = set(
        str(item) for item in (availability.get("serviceAccountIds") or [])
    )
    available_roles = set(str(item) for item in (availability.get("roles") or []))
    checks = []
    for row in export_document.get("permissions") or []:
        if not isinstance(row, dict):
            continue
        subject_type = _normalize_text(row.get("subjectType"), "unknown")
        subject_id = _normalize_text(row.get("subjectId"))
        subject_key = _normalize_text(row.get("subjectKey"), "unknown")
        status = "ok"
        detail = "Permission subject is available for promotion."
        if subject_type == "user" and subject_id not in available_users:
            status = "missing"
            detail = "Target Grafana user is missing."
        elif subject_type == "team" and subject_id not in available_teams:
            status = "missing"
            detail = "Target Grafana team is missing."
        elif subject_type == "service-account" and subject_id not in available_service_accounts:
            status = "missing"
            detail = "Target Grafana service account is missing."
        elif subject_type == "role" and subject_id not in available_roles:
            status = "missing"
            detail = "Target Grafana role is missing."
        checks.append(
            {
                "subjectKey": subject_key,
                "subjectType": subject_type,
                "subjectName": _normalize_text(row.get("subjectName"), "unknown"),
                "permissionName": _normalize_text(row.get("permissionName"), "unknown"),
                "status": status,
                "detail": detail,
            }
        )
    return {
        "kind": PERMISSION_PREFLIGHT_KIND,
        "schemaVersion": PERMISSION_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "checkCount": len(checks),
            "okCount": len([item for item in checks if item["status"] == "ok"]),
            "missingCount": len([item for item in checks if item["status"] == "missing"]),
            "blockingCount": len([item for item in checks if item["status"] != "ok"]),
        },
        "checks": checks,
    }


def build_permission_promotion_document(
    expected_document: dict[str, Any],
    actual_document: dict[str, Any],
) -> dict[str, Any]:
    """Build one promotion/drift summary document from expected vs actual ACLs."""
    diff_document = build_permission_diff_document(expected_document, actual_document)
    rows = list(diff_document.get("rows") or [])
    return {
        "kind": PERMISSION_PROMOTION_KIND,
        "schemaVersion": PERMISSION_PROMOTION_SCHEMA_VERSION,
        "summary": {
            "sameCount": int((diff_document.get("summary") or {}).get("sameCount") or 0),
            "changedCount": int((diff_document.get("summary") or {}).get("changedCount") or 0),
            "missingLiveCount": int(
                (diff_document.get("summary") or {}).get("missingLiveCount") or 0
            ),
            "extraLiveCount": int(
                (diff_document.get("summary") or {}).get("extraLiveCount") or 0
            ),
            "wouldAddCount": len([row for row in rows if row.get("status") == "missing-live"]),
            "wouldChangeCount": len([row for row in rows if row.get("status") == "changed"]),
            "wouldLeaveExtraCount": len([row for row in rows if row.get("status") == "extra-live"]),
        },
        "rows": rows,
    }


def render_permission_preflight_text(document: dict[str, Any]) -> list[str]:
    """Render permission preflight checks as text."""
    summary = document.get("summary") or {}
    lines = [
        "Permission preflight summary",
        "Checks: %s total, %s ok, %s missing, %s blocking"
        % (
            int(summary.get("checkCount") or 0),
            int(summary.get("okCount") or 0),
            int(summary.get("missingCount") or 0),
            int(summary.get("blockingCount") or 0),
        ),
        "",
        "# Checks",
    ]
    for row in document.get("checks") or []:
        if not isinstance(row, dict):
            continue
        lines.append(
            "- %s %s permission=%s status=%s"
            % (
                _normalize_text(row.get("subjectType"), "unknown"),
                _normalize_text(row.get("subjectName"), "unknown"),
                _normalize_text(row.get("permissionName"), "unknown"),
                _normalize_text(row.get("status"), "unknown"),
            )
        )
    return lines


def build_permission_bundle_document(
    resources: list[dict[str, Any]],
) -> dict[str, Any]:
    """Build a bundle document from multiple dashboard/folder permission exports."""
    documents = []
    for item in resources:
        if not isinstance(item, dict):
            continue
        documents.append(
            build_permission_export_document(
                _normalize_text(item.get("resourceKind"), "dashboard"),
                _normalize_text(item.get("resourceUid"), "unknown"),
                _normalize_text(item.get("resourceTitle"), "unknown"),
                list(item.get("permissions") or []),
            )
        )
    documents.sort(
        key=lambda item: (
            _normalize_text((item.get("resource") or {}).get("kind")),
            _normalize_text((item.get("resource") or {}).get("uid")),
        )
    )
    return {
        "kind": PERMISSION_BUNDLE_KIND,
        "schemaVersion": PERMISSION_BUNDLE_SCHEMA_VERSION,
        "summary": {
            "resourceCount": len(documents),
            "dashboardCount": len(
                [item for item in documents if (item.get("resource") or {}).get("kind") == "dashboard"]
            ),
            "folderCount": len(
                [item for item in documents if (item.get("resource") or {}).get("kind") == "folder"]
            ),
            "permissionCount": sum(
                int((item.get("summary") or {}).get("permissionCount") or 0)
                for item in documents
            ),
        },
        "resources": documents,
    }


def build_permission_bundle_diff_document(
    expected_bundle: dict[str, Any],
    actual_bundle: dict[str, Any],
) -> dict[str, Any]:
    """Build one diff summary across multiple dashboard/folder permission exports."""
    expected_map = {
        "%s:%s"
        % (
            _normalize_text((item.get("resource") or {}).get("kind")),
            _normalize_text((item.get("resource") or {}).get("uid")),
        ): item
        for item in expected_bundle.get("resources") or []
        if isinstance(item, dict)
    }
    actual_map = {
        "%s:%s"
        % (
            _normalize_text((item.get("resource") or {}).get("kind")),
            _normalize_text((item.get("resource") or {}).get("uid")),
        ): item
        for item in actual_bundle.get("resources") or []
        if isinstance(item, dict)
    }
    resource_rows = []
    for key in sorted(set(expected_map) | set(actual_map)):
        expected = expected_map.get(key)
        actual = actual_map.get(key)
        if expected is None:
            resource_rows.append(
                {
                    "resourceKey": key,
                    "status": "extra-live",
                    "diffSummary": {},
                }
            )
            continue
        if actual is None:
            resource_rows.append(
                {
                    "resourceKey": key,
                    "status": "missing-live",
                    "diffSummary": {},
                }
            )
            continue
        diff = build_permission_diff_document(expected, actual)
        diff_summary = dict(diff.get("summary") or {})
        status = "same"
        if int(diff_summary.get("changedCount") or 0) > 0:
            status = "changed"
        elif int(diff_summary.get("missingLiveCount") or 0) > 0:
            status = "missing-live"
        elif int(diff_summary.get("extraLiveCount") or 0) > 0:
            status = "extra-live"
        resource_rows.append(
            {
                "resourceKey": key,
                "status": status,
                "diffSummary": diff_summary,
            }
        )
    return {
        "kind": PERMISSION_BUNDLE_DIFF_KIND,
        "schemaVersion": PERMISSION_BUNDLE_DIFF_SCHEMA_VERSION,
        "summary": {
            "resourceCount": len(resource_rows),
            "sameCount": len([row for row in resource_rows if row["status"] == "same"]),
            "changedCount": len([row for row in resource_rows if row["status"] == "changed"]),
            "missingLiveCount": len(
                [row for row in resource_rows if row["status"] == "missing-live"]
            ),
            "extraLiveCount": len(
                [row for row in resource_rows if row["status"] == "extra-live"]
            ),
        },
        "resources": resource_rows,
    }


def render_permission_bundle_text(document: dict[str, Any]) -> list[str]:
    """Render a staged permission bundle as text."""
    summary = document.get("summary") or {}
    lines = [
        "Permission bundle summary",
        "Counts: %s resources, %s dashboards, %s folders, %s permissions"
        % (
            int(summary.get("resourceCount") or 0),
            int(summary.get("dashboardCount") or 0),
            int(summary.get("folderCount") or 0),
            int(summary.get("permissionCount") or 0),
        ),
        "",
        "# Resources",
    ]
    for item in document.get("resources") or []:
        if not isinstance(item, dict):
            continue
        resource = item.get("resource") or {}
        item_summary = item.get("summary") or {}
        lines.append(
            "- %s uid=%s title=%s permissions=%s"
            % (
                _normalize_text(resource.get("kind"), "unknown"),
                _normalize_text(resource.get("uid"), "unknown"),
                _normalize_text(resource.get("title"), "unknown"),
                int(item_summary.get("permissionCount") or 0),
            )
        )
    return lines


def build_permission_remap_document(
    bundle_document: dict[str, Any],
    remap_rules: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build one reviewable source-to-target remap plan for permission resources."""
    remap_rules = dict(remap_rules or {})
    uid_map = dict(remap_rules.get("uidMap") or {})
    title_map = dict(remap_rules.get("titleMap") or {})
    path_map = dict(remap_rules.get("pathMap") or {})
    rows = []
    for item in bundle_document.get("resources") or []:
        if not isinstance(item, dict):
            continue
        resource = item.get("resource") or {}
        resource_kind = _normalize_text(resource.get("kind"), "unknown")
        source_uid = _normalize_text(resource.get("uid"), "unknown")
        source_title = _normalize_text(resource.get("title"), "unknown")
        key = "%s:%s" % (resource_kind, source_uid)
        target_uid = _normalize_text(uid_map.get(key), source_uid)
        target_title = _normalize_text(title_map.get(key), source_title)
        target_path = _normalize_text(path_map.get(key))
        remapped = (
            target_uid != source_uid or target_title != source_title or bool(target_path)
        )
        rows.append(
            {
                "resourceKind": resource_kind,
                "sourceUid": source_uid,
                "sourceTitle": source_title,
                "targetUid": target_uid,
                "targetTitle": target_title,
                "targetPath": target_path,
                "remapped": remapped,
            }
        )
    return {
        "kind": PERMISSION_REMAP_KIND,
        "schemaVersion": PERMISSION_REMAP_SCHEMA_VERSION,
        "summary": {
            "resourceCount": len(rows),
            "remappedCount": len([row for row in rows if row["remapped"]]),
            "unchangedCount": len([row for row in rows if not row["remapped"]]),
        },
        "rules": {
            "uidMap": uid_map,
            "titleMap": title_map,
            "pathMap": path_map,
        },
        "resources": rows,
    }


def render_permission_remap_text(document: dict[str, Any]) -> list[str]:
    """Render the staged permission remap plan as text."""
    summary = document.get("summary") or {}
    lines = [
        "Permission remap summary",
        "Counts: %s resources, %s remapped, %s unchanged"
        % (
            int(summary.get("resourceCount") or 0),
            int(summary.get("remappedCount") or 0),
            int(summary.get("unchangedCount") or 0),
        ),
        "",
        "# Resources",
    ]
    for row in document.get("resources") or []:
        if not isinstance(row, dict):
            continue
        lines.append(
            "- %s %s -> %s remapped=%s"
            % (
                _normalize_text(row.get("resourceKind"), "unknown"),
                _normalize_text(row.get("sourceUid"), "unknown"),
                _normalize_text(row.get("targetUid"), "unknown"),
                "true" if bool(row.get("remapped")) else "false",
            )
        )
    return lines
