"""Alert provisioning import/export helpers."""

import copy
import json
from pathlib import Path
from typing import Any, Optional

from ..clients.alert_client import GrafanaAlertClient
from .common import (
    CONTACT_POINT_KIND,
    LINKED_DASHBOARD_ANNOTATION_KEY,
    LINKED_PANEL_ANNOTATION_KEY,
    MUTE_TIMING_KIND,
    POLICIES_KIND,
    RESOURCE_SUBDIR_BY_KIND,
    ROOT_INDEX_KIND,
    RULE_KIND,
    RULES_SUBDIR,
    CONTACT_POINTS_SUBDIR,
    MUTE_TIMINGS_SUBDIR,
    POLICIES_SUBDIR,
    SERVER_MANAGED_FIELDS_BY_KIND,
    TEMPLATE_KIND,
    TEMPLATES_SUBDIR,
    TOOL_API_VERSION,
    TOOL_SCHEMA_VERSION,
    GrafanaApiError,
    GrafanaError,
)


def strip_server_managed_fields(kind: str, payload: dict[str, Any]) -> dict[str, Any]:
    normalized = copy.deepcopy(payload)
    for field in SERVER_MANAGED_FIELDS_BY_KIND.get(kind, set()):
        normalized.pop(field, None)
    return normalized


def get_rule_linkage(rule: dict[str, Any]) -> Optional[dict[str, str]]:
    annotations = rule.get("annotations")
    if not isinstance(annotations, dict):
        return None

    dashboard_uid = str(
        annotations.get(LINKED_DASHBOARD_ANNOTATION_KEY) or ""
    ).strip()
    if not dashboard_uid:
        return None

    panel_id = annotations.get(LINKED_PANEL_ANNOTATION_KEY)
    linkage = {"dashboardUid": dashboard_uid}
    if panel_id is not None:
        linkage["panelId"] = str(panel_id)
    return linkage


def find_panel_by_id(panels: Any, panel_id: str) -> Optional[dict[str, Any]]:
    if not isinstance(panels, list):
        return None
    for panel in panels:
        if not isinstance(panel, dict):
            continue
        current_panel_id = panel.get("id")
        if current_panel_id is not None and str(current_panel_id) == panel_id:
            return panel
        nested = find_panel_by_id(panel.get("panels"), panel_id)
        if nested is not None:
            return nested
    return None


def derive_dashboard_slug(value: str) -> str:
    slug = str(value or "").strip().strip("/")
    if not slug:
        return ""
    parts = slug.split("/")
    return parts[-1] if parts else slug


def build_linked_dashboard_metadata(
    client: GrafanaAlertClient,
    rule: dict[str, Any],
) -> Optional[dict[str, str]]:
    linkage = get_rule_linkage(rule)
    if not linkage:
        return None

    metadata = dict(linkage)
    dashboard_uid = linkage["dashboardUid"]
    try:
        dashboard_payload = client.get_dashboard(dashboard_uid)
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise
        return metadata

    dashboard = dashboard_payload.get("dashboard")
    meta = dashboard_payload.get("meta")
    if isinstance(dashboard, dict):
        metadata["dashboardTitle"] = str(dashboard.get("title") or "")
        panel_id = metadata.get("panelId")
        if panel_id:
            panel = find_panel_by_id(dashboard.get("panels"), panel_id)
            if isinstance(panel, dict):
                metadata["panelTitle"] = str(panel.get("title") or "")
                metadata["panelType"] = str(panel.get("type") or "")
    if isinstance(meta, dict):
        metadata["folderTitle"] = str(meta.get("folderTitle") or "")
        metadata["folderUid"] = str(meta.get("folderUid") or "")
        metadata["dashboardSlug"] = derive_dashboard_slug(
            meta.get("url") or meta.get("slug") or ""
        )
    return metadata


def filter_dashboard_search_matches(
    candidates: list[dict[str, Any]],
    linked_dashboard: dict[str, Any],
) -> list[dict[str, Any]]:
    dashboard_title = str(linked_dashboard.get("dashboardTitle") or "")
    filtered = [
        item for item in candidates if str(item.get("title") or "") == dashboard_title
    ]

    folder_title = str(linked_dashboard.get("folderTitle") or "")
    if folder_title:
        folder_matches = [
            item for item in filtered if str(item.get("folderTitle") or "") == folder_title
        ]
        if folder_matches:
            filtered = folder_matches

    slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug") or "")
    if slug:
        slug_matches = [
            item
            for item in filtered
            if derive_dashboard_slug(item.get("url") or item.get("slug") or "") == slug
        ]
        if slug_matches:
            filtered = slug_matches

    return filtered


def resolve_dashboard_uid_fallback(
    client: GrafanaAlertClient,
    linked_dashboard: dict[str, Any],
) -> str:
    dashboard_title = str(linked_dashboard.get("dashboardTitle") or "").strip()
    if not dashboard_title:
        raise GrafanaError(
            "Alert rule references a dashboard UID that does not exist on the target "
            "Grafana, and the export file does not include dashboard title metadata "
            "for fallback matching. Re-export the alert rule with the current tool."
        )

    candidates = client.search_dashboards(dashboard_title)
    filtered = filter_dashboard_search_matches(candidates, linked_dashboard)
    if len(filtered) == 1:
        resolved_uid = str(filtered[0].get("uid") or "")
        if resolved_uid:
            return resolved_uid

    folder_title = str(linked_dashboard.get("folderTitle") or "")
    slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug") or "")
    if not filtered:
        raise GrafanaError(
            "Cannot resolve linked dashboard for alert rule. "
            "No dashboard matched title=%r, folderTitle=%r, slug=%r."
            % (dashboard_title, folder_title, slug)
        )
    raise GrafanaError(
        "Cannot resolve linked dashboard for alert rule. "
        "Multiple dashboards matched title=%r, folderTitle=%r, slug=%r."
        % (dashboard_title, folder_title, slug)
    )


def load_string_map(
    path_value: Optional[str],
    label: str,
    load_json_file,
) -> dict[str, str]:
    if not path_value:
        return {}
    payload = load_json_file(Path(path_value))
    if not isinstance(payload, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    normalized = {}
    for key, value in payload.items():
        normalized[str(key)] = str(value)
    return normalized


def load_panel_id_map(
    path_value: Optional[str],
    load_json_file,
) -> dict[str, dict[str, str]]:
    if not path_value:
        return {}
    payload = load_json_file(Path(path_value))
    if not isinstance(payload, dict):
        raise GrafanaError("Panel ID map must be a JSON object.")
    normalized = {}
    for dashboard_uid, panel_mapping in payload.items():
        if not isinstance(panel_mapping, dict):
            raise GrafanaError(
                "Panel ID map values must be JSON objects keyed by source panel ID."
            )
        normalized[str(dashboard_uid)] = {
            str(panel_id): str(target_panel_id)
            for panel_id, target_panel_id in panel_mapping.items()
        }
    return normalized


def apply_rule_linkage_maps(
    payload: dict[str, Any],
    dashboard_uid_map: dict[str, str],
    panel_id_map: dict[str, dict[str, str]],
) -> tuple[Optional[dict[str, Any]], str]:
    linkage = get_rule_linkage(payload)
    if not linkage:
        return None, ""

    source_dashboard_uid = linkage["dashboardUid"]
    dashboard_uid = dashboard_uid_map.get(source_dashboard_uid, source_dashboard_uid)
    source_panel_id = linkage.get("panelId", "")
    mapped_panel_id = panel_id_map.get(source_dashboard_uid, {}).get(source_panel_id, "")

    normalized = copy.deepcopy(payload)
    annotations = normalized.setdefault("annotations", {})
    if not isinstance(annotations, dict):
        raise GrafanaError("Alert-rule annotations must be an object.")

    annotations[LINKED_DASHBOARD_ANNOTATION_KEY] = dashboard_uid
    if mapped_panel_id:
        annotations[LINKED_PANEL_ANNOTATION_KEY] = mapped_panel_id
    return normalized, dashboard_uid


def extract_linked_dashboard_metadata(
    document: dict[str, Any],
    dashboard_uid: str,
) -> dict[str, Any]:
    metadata = document.get("metadata")
    linked_dashboard = metadata.get("linkedDashboard") if isinstance(metadata, dict) else None
    if not isinstance(linked_dashboard, dict):
        raise GrafanaError(
            "Alert rule references dashboard UID %r, but that dashboard "
            "does not exist on the target Grafana and the export file has no linked "
            "dashboard metadata for fallback matching." % dashboard_uid
        )
    return linked_dashboard


def rewrite_rule_dashboard_linkage(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    document: dict[str, Any],
    dashboard_uid_map: dict[str, str],
    panel_id_map: dict[str, dict[str, str]],
) -> dict[str, Any]:
    normalized, dashboard_uid = apply_rule_linkage_maps(
        payload,
        dashboard_uid_map,
        panel_id_map,
    )
    if normalized is None:
        return payload

    try:
        client.get_dashboard(dashboard_uid)
        return normalized
    except GrafanaApiError as exc:
        if exc.status_code != 404:
            raise

    linked_dashboard = extract_linked_dashboard_metadata(document, dashboard_uid)
    annotations = normalized["annotations"]
    replacement_uid = resolve_dashboard_uid_fallback(client, linked_dashboard)
    annotations[LINKED_DASHBOARD_ANNOTATION_KEY] = replacement_uid
    return normalized


def build_rule_metadata(rule: dict[str, Any]) -> dict[str, Any]:
    metadata = {
        "uid": str(rule.get("uid") or ""),
        "title": str(rule.get("title") or ""),
        "folderUID": str(rule.get("folderUID") or ""),
        "ruleGroup": str(rule.get("ruleGroup") or ""),
    }
    linked_dashboard = rule.get("__linkedDashboardMetadata__")
    if isinstance(linked_dashboard, dict):
        metadata["linkedDashboard"] = {
            key: str(value or "") for key, value in linked_dashboard.items()
        }
    return metadata


def build_contact_point_metadata(contact_point: dict[str, Any]) -> dict[str, str]:
    return {
        "uid": str(contact_point.get("uid") or ""),
        "name": str(contact_point.get("name") or ""),
        "type": str(contact_point.get("type") or ""),
    }


def build_mute_timing_metadata(mute_timing: dict[str, Any]) -> dict[str, str]:
    return {"name": str(mute_timing.get("name") or "")}


def build_policies_metadata(policies: dict[str, Any]) -> dict[str, str]:
    return {"receiver": str(policies.get("receiver") or "")}


def build_template_metadata(template: dict[str, Any]) -> dict[str, str]:
    return {"name": str(template.get("name") or "")}


def build_tool_document(kind: str, spec: dict[str, Any]) -> dict[str, Any]:
    metadata_builders = {
        RULE_KIND: build_rule_metadata,
        CONTACT_POINT_KIND: build_contact_point_metadata,
        MUTE_TIMING_KIND: build_mute_timing_metadata,
        POLICIES_KIND: build_policies_metadata,
        TEMPLATE_KIND: build_template_metadata,
    }
    metadata_builder = metadata_builders[kind]
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "apiVersion": TOOL_API_VERSION,
        "kind": kind,
        "metadata": metadata_builder(spec),
        "spec": spec,
    }


def build_rule_export_document(rule: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(rule, dict):
        raise GrafanaError("Unexpected alert-rule payload from Grafana.")
    normalized_rule = strip_server_managed_fields(RULE_KIND, rule)
    linked_dashboard = normalized_rule.pop("__linkedDashboardMetadata__", None)
    document = build_tool_document(RULE_KIND, normalized_rule)
    if isinstance(linked_dashboard, dict):
        document["metadata"]["linkedDashboard"] = {
            key: str(value or "") for key, value in linked_dashboard.items()
        }
    return document


def build_contact_point_export_document(contact_point: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(contact_point, dict):
        raise GrafanaError("Unexpected contact-point payload from Grafana.")
    return build_tool_document(
        CONTACT_POINT_KIND,
        strip_server_managed_fields(CONTACT_POINT_KIND, contact_point),
    )


def build_mute_timing_export_document(mute_timing: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(mute_timing, dict):
        raise GrafanaError("Unexpected mute-timing payload from Grafana.")
    return build_tool_document(
        MUTE_TIMING_KIND,
        strip_server_managed_fields(MUTE_TIMING_KIND, mute_timing),
    )


def build_policies_export_document(policies: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(policies, dict):
        raise GrafanaError("Unexpected notification policy payload from Grafana.")
    return build_tool_document(
        POLICIES_KIND,
        strip_server_managed_fields(POLICIES_KIND, policies),
    )


def build_template_export_document(template: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(template, dict):
        raise GrafanaError("Unexpected template payload from Grafana.")
    return build_tool_document(
        TEMPLATE_KIND,
        strip_server_managed_fields(TEMPLATE_KIND, template),
    )


def reject_provisioning_export(document: dict[str, Any]) -> None:
    if (
        "groups" in document
        or "contactPoints" in document
        or "policies" in document
        or "templates" in document
    ):
        raise GrafanaError(
            "Grafana provisioning export format is not supported for API import. "
            "Use files exported by grafana-utils alert export."
        )


def detect_document_kind(document: dict[str, Any]) -> str:
    kind = document.get("kind")
    if kind in RESOURCE_SUBDIR_BY_KIND:
        return str(kind)
    if "condition" in document and "data" in document:
        return RULE_KIND
    if "time_intervals" in document and "name" in document:
        return MUTE_TIMING_KIND
    if "type" in document and "settings" in document and "name" in document:
        return CONTACT_POINT_KIND
    if "name" in document and "template" in document:
        return TEMPLATE_KIND
    if "receiver" in document or "routes" in document or "group_by" in document:
        return POLICIES_KIND
    raise GrafanaError("Cannot determine alerting resource kind from import document.")


def extract_tool_spec(document: dict[str, Any], expected_kind: str) -> dict[str, Any]:
    if document.get("kind") == expected_kind:
        api_version = document.get("apiVersion")
        if api_version not in (None, TOOL_API_VERSION):
            raise GrafanaError(
                "Unsupported %s export version: %r" % (expected_kind, api_version)
            )
        schema_version = document.get("schemaVersion")
        if schema_version not in (None, TOOL_SCHEMA_VERSION):
            raise GrafanaError(
                "Unsupported %s schema version: %r" % (expected_kind, schema_version)
            )
        spec = document.get("spec")
    else:
        spec = document
    if not isinstance(spec, dict):
        raise GrafanaError("%s import document is missing a valid spec object." % expected_kind)
    return spec


def build_rule_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        RULE_KIND, extract_tool_spec(document, RULE_KIND)
    )
    required_fields = ("title", "folderUID", "ruleGroup", "condition", "data")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Alert-rule import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["data"], list):
        raise GrafanaError("Alert-rule field 'data' must be a list.")
    return payload


def build_contact_point_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        CONTACT_POINT_KIND, extract_tool_spec(document, CONTACT_POINT_KIND)
    )
    required_fields = ("name", "type", "settings")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Contact-point import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["settings"], dict):
        raise GrafanaError("Contact-point field 'settings' must be an object.")
    return payload


def build_mute_timing_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        MUTE_TIMING_KIND, extract_tool_spec(document, MUTE_TIMING_KIND)
    )
    required_fields = ("name", "time_intervals")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Mute-timing import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["time_intervals"], list):
        raise GrafanaError("Mute-timing field 'time_intervals' must be a list.")
    return payload


def build_policies_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        POLICIES_KIND, extract_tool_spec(document, POLICIES_KIND)
    )
    if not isinstance(payload, dict):
        raise GrafanaError("Notification policies import document must be an object.")
    return payload


def build_template_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        TEMPLATE_KIND, extract_tool_spec(document, TEMPLATE_KIND)
    )
    required_fields = ("name", "template")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Template import document is missing required fields: "
            + ", ".join(missing)
        )
    return payload


def build_import_operation(document: dict[str, Any]) -> tuple[str, dict[str, Any]]:
    if not isinstance(document, dict):
        raise GrafanaError("Unexpected alerting resource document. Expected a JSON object.")
    kind = detect_document_kind(document)
    builders = {
        RULE_KIND: build_rule_import_payload,
        CONTACT_POINT_KIND: build_contact_point_import_payload,
        MUTE_TIMING_KIND: build_mute_timing_import_payload,
        POLICIES_KIND: build_policies_import_payload,
        TEMPLATE_KIND: build_template_import_payload,
    }
    return kind, builders[kind](document)


def prepare_rule_payload_for_target(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    document: dict[str, Any],
    dashboard_uid_map: dict[str, str],
    panel_id_map: dict[str, dict[str, str]],
) -> dict[str, Any]:
    return rewrite_rule_dashboard_linkage(
        client,
        payload,
        document,
        dashboard_uid_map,
        panel_id_map,
    )


def prepare_import_payload_for_target(
    client: GrafanaAlertClient,
    kind: str,
    payload: dict[str, Any],
    document: dict[str, Any],
    dashboard_uid_map: dict[str, str],
    panel_id_map: dict[str, dict[str, str]],
) -> dict[str, Any]:
    if kind == RULE_KIND:
        return prepare_rule_payload_for_target(
            client,
            payload,
            document,
            dashboard_uid_map,
            panel_id_map,
        )
    return payload


def build_compare_document(kind: str, payload: dict[str, Any]) -> dict[str, Any]:
    return {"kind": kind, "spec": payload}


def serialize_compare_document(document: dict[str, Any]) -> str:
    return json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def build_resource_identity(kind: str, payload: dict[str, Any]) -> str:
    if kind == RULE_KIND:
        return str(payload.get("uid") or "unknown")
    if kind == CONTACT_POINT_KIND:
        return str(payload.get("uid") or payload.get("name") or "unknown")
    if kind == MUTE_TIMING_KIND:
        return str(payload.get("name") or "unknown")
    if kind == TEMPLATE_KIND:
        return str(payload.get("name") or "unknown")
    return str(payload.get("receiver") or "root")


def build_diff_label(prefix: str, resource_file: Path, kind: str, identity: str) -> str:
    return "%s:%s:%s:%s" % (prefix, resource_file, kind, identity)


def determine_rule_import_action(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    replace_existing: bool,
) -> str:
    uid = str(payload.get("uid") or "")
    if not uid:
        return "would-create"
    try:
        client.get_alert_rule(uid)
    except GrafanaApiError as exc:
        if exc.status_code == 404:
            return "would-create"
        raise
    if replace_existing:
        return "would-update"
    return "would-fail-existing"


def determine_contact_point_import_action(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    replace_existing: bool,
) -> str:
    uid = str(payload.get("uid") or "")
    existing = {str(item.get("uid") or "") for item in client.list_contact_points()}
    if uid and uid in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_mute_timing_import_action(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    replace_existing: bool,
) -> str:
    name = str(payload.get("name") or "")
    existing = {str(item.get("name") or "") for item in client.list_mute_timings()}
    if name and name in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_template_import_action(
    client: GrafanaAlertClient,
    payload: dict[str, Any],
    replace_existing: bool,
) -> str:
    name = str(payload.get("name") or "")
    existing = {str(item.get("name") or "") for item in client.list_templates()}
    if name and name in existing:
        if replace_existing:
            return "would-update"
        return "would-fail-existing"
    return "would-create"


def determine_import_action(
    client: GrafanaAlertClient,
    kind: str,
    payload: dict[str, Any],
    replace_existing: bool,
) -> str:
    if kind == RULE_KIND:
        return determine_rule_import_action(client, payload, replace_existing)
    if kind == CONTACT_POINT_KIND:
        return determine_contact_point_import_action(client, payload, replace_existing)
    if kind == MUTE_TIMING_KIND:
        return determine_mute_timing_import_action(client, payload, replace_existing)
    if kind == TEMPLATE_KIND:
        return determine_template_import_action(client, payload, replace_existing)
    return "would-update"


def fetch_live_compare_document(
    client: GrafanaAlertClient,
    kind: str,
    payload: dict[str, Any],
) -> Optional[dict[str, Any]]:
    if kind == RULE_KIND:
        uid = str(payload.get("uid") or "")
        if not uid:
            return None
        try:
            remote_payload = client.get_alert_rule(uid)
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        return build_compare_document(
            kind,
            strip_server_managed_fields(kind, remote_payload),
        )

    if kind == CONTACT_POINT_KIND:
        uid = str(payload.get("uid") or "")
        if not uid:
            return None
        for item in client.list_contact_points():
            if str(item.get("uid") or "") == uid:
                return build_compare_document(
                    kind,
                    strip_server_managed_fields(kind, item),
                )
        return None

    if kind == MUTE_TIMING_KIND:
        name = str(payload.get("name") or "")
        if not name:
            return None
        for item in client.list_mute_timings():
            if str(item.get("name") or "") == name:
                return build_compare_document(
                    kind,
                    strip_server_managed_fields(kind, item),
                )
        return None

    if kind == TEMPLATE_KIND:
        name = str(payload.get("name") or "")
        if not name:
            return None
        try:
            remote_payload = client.get_template(name)
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        return build_compare_document(
            kind,
            strip_server_managed_fields(kind, remote_payload),
        )

    return build_compare_document(
        kind,
        strip_server_managed_fields(kind, client.get_notification_policies()),
    )


def build_empty_root_index() -> dict[str, Any]:
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "apiVersion": TOOL_API_VERSION,
        "kind": ROOT_INDEX_KIND,
        RULES_SUBDIR: [],
        CONTACT_POINTS_SUBDIR: [],
        MUTE_TIMINGS_SUBDIR: [],
        POLICIES_SUBDIR: [],
        TEMPLATES_SUBDIR: [],
    }
