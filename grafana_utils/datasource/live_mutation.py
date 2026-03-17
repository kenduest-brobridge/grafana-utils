"""Unwired live datasource add/delete helpers.

Purpose:
- Provide reusable add/delete workflow helpers for live Grafana datasources
  without depending on the local export/import bundle format.
- Keep the implementation isolated so future CLI wiring can import these
  helpers without changing the existing datasource import contract first.

Caveats:
- This module is intentionally not wired into the user-facing CLI yet.
- Add only supports create semantics for now. If a datasource already exists,
  the helper fails closed instead of updating it.
"""

from copy import deepcopy
from typing import Any, Optional

from ..dashboard_cli import GrafanaError

ALLOWED_ADD_FIELDS = (
    "uid",
    "name",
    "type",
    "access",
    "url",
    "isDefault",
    "jsonData",
    "secureJsonData",
)


def _normalize_string(value):
    """Internal helper for normalize string."""
    if value is None:
        return ""
    return str(value).strip()


def _normalize_bool(value):
    """Internal helper for normalize bool."""
    if isinstance(value, bool):
        return value
    return _normalize_string(value).lower() in ("true", "1", "yes", "on")


def _copy_json_object(value, label):
    """Internal helper for copy json object."""
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    return deepcopy(value)


def build_datasource_identity_lookups(datasources):
    """Build datasource identity lookups implementation."""
    by_uid = {}
    by_name = {}
    for datasource in datasources:
        uid = _normalize_string(datasource.get("uid"))
        name = _normalize_string(datasource.get("name"))
        if uid:
            by_uid.setdefault(uid, []).append(datasource)
        if name:
            by_name.setdefault(name, []).append(datasource)
    return {"by_uid": by_uid, "by_name": by_name}


def resolve_datasource_target(datasources, uid=None, name=None):
    """Resolve datasource target implementation."""
    normalized_uid = _normalize_string(uid)
    normalized_name = _normalize_string(name)
    if not normalized_uid and not normalized_name:
        raise GrafanaError("Datasource target lookup requires --uid or --name.")

    lookups = build_datasource_identity_lookups(datasources)

    if normalized_uid:
        uid_matches = lookups["by_uid"].get(normalized_uid) or []
        if len(uid_matches) > 1:
            return {"state": "ambiguous-uid", "target": None}
        if len(uid_matches) == 1:
            target = uid_matches[0]
            if normalized_name:
                target_name = _normalize_string(target.get("name"))
                if target_name and target_name != normalized_name:
                    return {"state": "uid-name-mismatch", "target": target}
            return {"state": "exists-uid", "target": target}
        if not normalized_name:
            return {"state": "missing", "target": None}

    if normalized_name:
        name_matches = lookups["by_name"].get(normalized_name) or []
        if len(name_matches) > 1:
            return {"state": "ambiguous-name", "target": None}
        if len(name_matches) == 1:
            target = name_matches[0]
            if normalized_uid:
                target_uid = _normalize_string(target.get("uid"))
                if target_uid and target_uid != normalized_uid:
                    return {"state": "uid-name-mismatch", "target": target}
            return {"state": "exists-name", "target": target}

    return {"state": "missing", "target": None}


def normalize_add_spec(spec):
    """Normalize add spec implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 158
    #   Downstream callees: 32, 39, 46

    if not isinstance(spec, dict):
        raise GrafanaError("Datasource add spec must be a JSON object.")

    extra_fields = sorted(key for key in spec.keys() if key not in ALLOWED_ADD_FIELDS)
    if extra_fields:
        raise GrafanaError(
            "Datasource add spec contains unsupported field(s): %s."
            % ", ".join(extra_fields)
        )

    name = _normalize_string(spec.get("name"))
    datasource_type = _normalize_string(spec.get("type"))
    if not name:
        raise GrafanaError("Datasource add spec requires a non-empty name.")
    if not datasource_type:
        raise GrafanaError("Datasource add spec requires a non-empty type.")

    normalized = {
        "name": name,
        "type": datasource_type,
    }

    uid = _normalize_string(spec.get("uid"))
    if uid:
        normalized["uid"] = uid

    access = _normalize_string(spec.get("access"))
    if access:
        normalized["access"] = access

    url = _normalize_string(spec.get("url"))
    if url:
        normalized["url"] = url

    if "isDefault" in spec:
        normalized["isDefault"] = _normalize_bool(spec.get("isDefault"))

    if "jsonData" in spec:
        normalized["jsonData"] = _copy_json_object(spec.get("jsonData"), "jsonData")

    if "secureJsonData" in spec:
        normalized["secureJsonData"] = _copy_json_object(
            spec.get("secureJsonData"),
            "secureJsonData",
        )

    return normalized


def build_add_payload(spec):
    """Build add payload implementation."""
    normalized = normalize_add_spec(spec)
    payload = {
        "name": normalized["name"],
        "type": normalized["type"],
    }

    for field in ("uid", "access", "url", "isDefault", "jsonData", "secureJsonData"):
        if field in normalized:
            payload[field] = normalized[field]
    return payload


def plan_add_datasource(client, spec):
    """Plan add datasource implementation."""
    payload = build_add_payload(spec)
    match = resolve_datasource_target(
        client.list_datasources(),
        uid=payload.get("uid"),
        name=payload.get("name"),
    )
    action = "would-create"
    if match["state"] != "missing":
        action = "would-fail-existing"
    return {
        "action": action,
        "match": match["state"],
        "payload": payload,
        "target": match.get("target"),
    }


def add_datasource(client, spec, dry_run=False):
    """Add datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 172

    plan = plan_add_datasource(client, spec)
    if plan["action"] != "would-create":
        raise GrafanaError(
            "Datasource add blocked for name=%s uid=%s match=%s action=%s"
            % (
                plan["payload"].get("name") or "-",
                plan["payload"].get("uid") or "-",
                plan["match"],
                plan["action"],
            )
        )
    if dry_run:
        return plan
    response = client.request_json(
        "/api/datasources",
        method="POST",
        payload=plan["payload"],
    )
    return {"action": "created", "payload": plan["payload"], "response": response}


def plan_delete_datasource(client, uid=None, name=None):
    """Plan delete datasource implementation."""
    match = resolve_datasource_target(client.list_datasources(), uid=uid, name=name)
    action = "would-delete"
    if match["state"] == "missing":
        action = "would-fail-missing"
    elif match["state"] in ("ambiguous-uid", "ambiguous-name", "uid-name-mismatch"):
        action = "would-fail-ambiguous"
    return {
        "action": action,
        "match": match["state"],
        "target": match.get("target"),
    }


def delete_datasource(client, uid=None, name=None, dry_run=False):
    """Delete datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 214, 32

    plan = plan_delete_datasource(client, uid=uid, name=name)
    if plan["action"] != "would-delete":
        raise GrafanaError(
            "Datasource delete blocked for uid=%s name=%s match=%s action=%s"
            % (_normalize_string(uid) or "-", _normalize_string(name) or "-", plan["match"], plan["action"])
        )
    target = plan.get("target") or {}
    datasource_id = target.get("id")
    if datasource_id is None:
        raise GrafanaError("Datasource delete requires a live datasource id.")
    if dry_run:
        return plan
    response = client.request_json(
        "/api/datasources/%s" % datasource_id,
        method="DELETE",
    )
    return {"action": "deleted", "target": target, "response": response}


__all__ = [
    "ALLOWED_ADD_FIELDS",
    "add_datasource",
    "build_add_payload",
    "build_datasource_identity_lookups",
    "delete_datasource",
    "normalize_add_spec",
    "plan_add_datasource",
    "plan_delete_datasource",
    "resolve_datasource_target",
]
