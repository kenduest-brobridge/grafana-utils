#!/usr/bin/env python3
"""Public Python CLI facade for conservative declarative sync planning.

Purpose:
- Expose the existing GitOps sync planning scaffold through `grafana-util sync`.
- Keep the current public surface local-file based and non-live so reviewable
  plan/apply contracts can settle before any Grafana API mutation is wired in.

Architecture:
- Parse and validate one local JSON input/output flow per subcommand.
- Delegate plan/review/apply gating to `grafana_utils.gitops_sync`.
- Keep output JSON-first so later CLI/table renderers can reuse one document
  contract.
"""

import argparse
import json
import sys
from urllib import parse

from .dashboard_cli import (
    GrafanaError,
    add_common_cli_args,
    build_client as build_dashboard_client,
)
from .datasource.live_mutation_safe import (
    build_add_payload as build_datasource_add_payload,
)
from .datasource.workflows import build_modify_datasource_payload
from .alert_sync_workbench import (
    assess_alert_sync_specs,
    render_alert_sync_assessment_text,
)
from .bundle_preflight import (
    build_bundle_preflight_document,
    render_bundle_preflight_text,
)
from .gitops_sync import (
    DEFAULT_REVIEW_TOKEN,
    SyncOperation,
    SyncPlan,
    build_apply_intent,
    build_sync_plan,
    mark_plan_reviewed,
    plan_to_document,
)
from .sync_preflight_workbench import (
    build_sync_preflight_document,
    render_sync_preflight_text,
)


PLAN_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json\n"
    "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json "
    "--allow-prune --plan-file ./sync-plan.json"
)
REVIEW_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync review --plan-file ./sync-plan.json\n"
    "  grafana-util sync review --plan-file ./sync-plan.json --output-file ./sync-plan-reviewed.json"
)
APPLY_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve\n"
    "  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --output-file ./sync-apply.json"
)
PREFLIGHT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync preflight --desired-file ./desired.json --availability-file ./availability.json\n"
    "  grafana-util sync preflight --desired-file ./desired.json --output json"
)
ASSESS_ALERTS_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync assess-alerts --alerts-file ./alerts.json\n"
    "  grafana-util sync assess-alerts --alerts-file ./alerts.json --output json"
)
BUNDLE_PREFLIGHT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json\n"
    "  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --availability-file ./availability.json --output json"
)


def build_parser(prog=None):
    """Build the sync CLI parser."""
    parser = argparse.ArgumentParser(
        prog=prog or "grafana-util sync",
        description=(
            "Build, review, and gate a local declarative Grafana sync plan "
            "without talking to Grafana."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True

    plan_parser = subparsers.add_parser(
        "plan",
        help="Build one review-required sync plan from desired/live JSON files.",
        epilog=PLAN_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    plan_input_group = plan_parser.add_argument_group("Input Source")
    plan_input_group.add_argument(
        "--desired-file",
        required=True,
        help="JSON file containing the desired managed resource list.",
    )
    plan_input_group.add_argument(
        "--live-file",
        default=None,
        help="JSON file containing the current live resource list.",
    )
    plan_input_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Read the current live state directly from Grafana instead of --live-file.",
    )
    add_common_cli_args(plan_parser)
    plan_behavior_group = plan_parser.add_argument_group("Plan Behavior")
    plan_behavior_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    plan_behavior_group.add_argument(
        "--page-size",
        type=int,
        default=500,
        help="Dashboard search page size when --fetch-live is active.",
    )
    plan_behavior_group.add_argument(
        "--allow-prune",
        action="store_true",
        help="Treat live resources missing from desired state as would-delete instead of unmanaged.",
    )
    plan_output_group = plan_parser.add_argument_group("Output")
    plan_output_group.add_argument(
        "--plan-file",
        default=None,
        help="Optional JSON file path to write the generated plan document.",
    )

    review_parser = subparsers.add_parser(
        "review",
        help="Mark a previously generated plan as reviewed.",
        epilog=REVIEW_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    review_group = review_parser.add_argument_group("Review Control")
    review_group.add_argument(
        "--plan-file",
        required=True,
        help="Input JSON plan document produced by `grafana-util sync plan`.",
    )
    review_group.add_argument(
        "--review-token",
        default=DEFAULT_REVIEW_TOKEN,
        help="Explicit review token required to mark the plan reviewed.",
    )
    review_output_group = review_parser.add_argument_group("Output")
    review_output_group.add_argument(
        "--output-file",
        default=None,
        help="Optional JSON file path to write the reviewed plan document.",
    )

    preflight_parser = subparsers.add_parser(
        "preflight",
        help="Build a staged sync preflight document from desired JSON and availability hints.",
        epilog=PREFLIGHT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    preflight_input_group = preflight_parser.add_argument_group("Input Source")
    preflight_input_group.add_argument(
        "--desired-file",
        required=True,
        help="JSON file containing the desired managed resource list.",
    )
    preflight_input_group.add_argument(
        "--availability-file",
        default=None,
        help="Optional JSON object file containing availability hints such as datasourceUids, pluginIds, and contactPoints.",
    )
    preflight_input_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch availability hints from Grafana instead of relying only on --availability-file.",
    )
    add_common_cli_args(preflight_parser)
    preflight_input_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    preflight_output_group = preflight_parser.add_argument_group("Output")
    preflight_output_group.add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the staged preflight document as text or json (default: text).",
    )

    assess_alerts_parser = subparsers.add_parser(
        "assess-alerts",
        help="Assess alert sync specs for candidate, plan-only, and blocked states.",
        epilog=ASSESS_ALERTS_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    assess_input_group = assess_alerts_parser.add_argument_group("Input")
    assess_input_group.add_argument(
        "--alerts-file",
        required=True,
        help="JSON file containing the alert sync resource list.",
    )
    assess_output_group = assess_alerts_parser.add_argument_group("Output")
    assess_output_group.add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the alert assessment as text or json (default: text).",
    )

    bundle_preflight_parser = subparsers.add_parser(
        "bundle-preflight",
        help="Build a staged bundle-level preflight document from source and target JSON inputs.",
        epilog=BUNDLE_PREFLIGHT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    bundle_input_group = bundle_preflight_parser.add_argument_group("Input Source")
    bundle_input_group.add_argument(
        "--source-bundle",
        required=True,
        help="JSON file containing the staged multi-resource source bundle.",
    )
    bundle_input_group.add_argument(
        "--target-inventory",
        required=True,
        help="JSON file containing the staged target inventory snapshot.",
    )
    bundle_input_group.add_argument(
        "--availability-file",
        default=None,
        help="Optional JSON object file containing staged availability hints.",
    )
    bundle_input_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch availability hints from Grafana instead of relying only on --availability-file.",
    )
    add_common_cli_args(bundle_preflight_parser)
    bundle_input_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    bundle_output_group = bundle_preflight_parser.add_argument_group("Output")
    bundle_output_group.add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the bundle preflight document as text or json (default: text).",
    )

    apply_parser = subparsers.add_parser(
        "apply",
        help="Build a gated apply intent from a reviewed plan without live mutation.",
        epilog=APPLY_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    apply_group = apply_parser.add_argument_group("Apply Metadata")
    apply_group.add_argument(
        "--plan-file",
        required=True,
        help="Input JSON plan document, typically already marked reviewed.",
    )
    apply_group.add_argument(
        "--approve",
        action="store_true",
        help="Explicit acknowledgement required before a non-live apply intent is emitted.",
    )
    add_common_cli_args(apply_parser)
    apply_execution_group = apply_parser.add_argument_group("Execution Control")
    apply_execution_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --execute-live is active.",
    )
    apply_execution_group.add_argument(
        "--execute-live",
        action="store_true",
        help="Apply supported sync operations to Grafana after review and approval checks pass.",
    )
    apply_execution_group.add_argument(
        "--allow-folder-delete",
        action="store_true",
        help="Allow live deletion of folders when a reviewed plan includes would-delete folder operations.",
    )
    apply_output_group = apply_parser.add_argument_group("Output")
    apply_output_group.add_argument(
        "--output-file",
        default=None,
        help="Optional JSON file path to write the apply intent document.",
    )
    return parser


def load_json_document(path):
    """Load one JSON document from disk with stable error messages."""
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc))
    except ValueError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc))


def write_json_document(path, document):
    """Write one JSON document to disk with stable formatting."""
    try:
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(document, handle, indent=2, sort_keys=False)
            handle.write("\n")
    except OSError as exc:
        raise GrafanaError("Failed to write %s: %s" % (path, exc))


def build_client(args):
    """Build one Grafana client, optionally scoped to one explicit org id."""
    client = build_dashboard_client(args)
    org_id = str(getattr(args, "org_id", "") or "").strip()
    if org_id:
        return client.with_org_id(org_id)
    return client


def _require_object(document, label):
    if not isinstance(document, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    return document


def _require_resource_list(document, label):
    if not isinstance(document, list):
        raise GrafanaError("%s must be a JSON array." % label)
    return document


def _coerce_operation(item, index):
    if not isinstance(item, dict):
        raise GrafanaError("Sync plan operation #%s must be a JSON object." % index)
    return SyncOperation(
        kind=str(item.get("kind") or "").strip(),
        identity=str(item.get("identity") or "").strip(),
        title=str(item.get("title") or "").strip(),
        action=str(item.get("action") or "").strip(),
        reason=str(item.get("reason") or "").strip(),
        changed_fields=tuple(item.get("changedFields") or ()),
        managed_fields=tuple(item.get("managedFields") or ()),
        desired=item.get("desired"),
        live=item.get("live"),
        source_path=str(item.get("sourcePath") or "").strip(),
    )


def load_plan_document(path):
    """Load one persisted sync plan document back into a SyncPlan."""
    document = _require_object(load_json_document(path), "Sync plan document")
    operations = []
    for index, item in enumerate(document.get("operations") or (), 1):
        operations.append(_coerce_operation(item, index))
    summary = document.get("summary") or {}
    if not isinstance(summary, dict):
        raise GrafanaError("Sync plan summary must be a JSON object.")
    return SyncPlan(
        dry_run=bool(document.get("dryRun")),
        review_required=bool(document.get("reviewRequired")),
        reviewed=bool(document.get("reviewed")),
        allow_prune=bool(document.get("allowPrune")),
        summary=dict(summary),
        operations=tuple(operations),
    )


def emit_document(document, output_file=None):
    """Print a JSON document and optionally persist it to disk."""
    if output_file:
        write_json_document(output_file, document)
    print(json.dumps(document, indent=2, sort_keys=False))


def _normalize_string(value, default=""):
    if value is None:
        return default
    text = str(value).strip()
    if text:
        return text
    return default


def _copy_mapping(value, label):
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    return dict(value)


def fetch_live_resource_specs(client, page_size=500):
    """Fetch a conservative live snapshot from Grafana for sync planning."""
    specs = []

    folders = client.request_json("/api/folders")
    if not isinstance(folders, list):
        raise GrafanaError("Unexpected folder list response from Grafana.")
    for folder in folders:
        if not isinstance(folder, dict):
            continue
        uid = _normalize_string(folder.get("uid"))
        if not uid:
            continue
        title = _normalize_string(
            folder.get("title"),
        )
        body = {"title": title or uid}
        parent_uid = _normalize_string(folder.get("parentUid"))
        if parent_uid:
            body["parentUid"] = parent_uid
        specs.append(
            {
                "kind": "folder",
                "uid": uid,
                "title": title or uid,
                "body": body,
            }
        )

    for summary in client.iter_dashboard_summaries(int(page_size or 500)):
        if not isinstance(summary, dict):
            continue
        uid = _normalize_string(summary.get("uid"))
        if not uid:
            continue
        dashboard_wrapper = client.fetch_dashboard_if_exists(uid)
        if dashboard_wrapper is None:
            continue
        dashboard = _copy_mapping(
            dashboard_wrapper.get("dashboard"),
            "Grafana dashboard payload",
        )
        dashboard.pop("id", None)
        specs.append(
            {
                "kind": "dashboard",
                "uid": uid,
                "title": _normalize_string(dashboard.get("title"), uid),
                "body": dashboard,
            }
        )

    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        uid = _normalize_string(datasource.get("uid"))
        name = _normalize_string(datasource.get("name"))
        identity = uid or name
        if not identity:
            continue
        body = {
            "name": name,
            "type": _normalize_string(datasource.get("type")),
            "access": _normalize_string(datasource.get("access")),
            "url": _normalize_string(datasource.get("url")),
            "isDefault": bool(datasource.get("isDefault")),
        }
        json_data = datasource.get("jsonData")
        if isinstance(json_data, dict) and json_data:
            body["jsonData"] = dict(json_data)
        specs.append(
            {
                "kind": "datasource",
                "uid": uid,
                "name": name or uid,
                "title": name or uid,
                "body": body,
            }
        )
    return specs


def run_plan(args):
    desired_specs = _require_resource_list(
        load_json_document(args.desired_file),
        "Desired sync input",
    )
    if bool(getattr(args, "fetch_live", False)):
        live_specs = fetch_live_resource_specs(
            build_client(args),
            page_size=getattr(args, "page_size", 500),
        )
    else:
        if not getattr(args, "live_file", None):
            raise GrafanaError(
                "Sync plan requires --live-file unless --fetch-live is used."
            )
        live_specs = _require_resource_list(
            load_json_document(args.live_file),
            "Live sync input",
        )
    plan = build_sync_plan(
        desired_specs=desired_specs,
        live_specs=live_specs,
        allow_prune=bool(getattr(args, "allow_prune", False)),
        dry_run=True,
        review_required=True,
    )
    document = plan_to_document(plan)
    emit_document(document, output_file=getattr(args, "plan_file", None))
    return 0


def run_review(args):
    plan = load_plan_document(args.plan_file)
    reviewed_plan = mark_plan_reviewed(
        plan,
        review_token=getattr(args, "review_token", DEFAULT_REVIEW_TOKEN),
    )
    emit_document(
        plan_to_document(reviewed_plan),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def _load_optional_object_file(path, label):
    if not path:
        return {}
    return _require_object(load_json_document(path), label)


def _merge_availability(base, extra):
    merged = dict(base or {})
    for key, value in (extra or {}).items():
        if key in ("datasourceUids", "pluginIds", "contactPoints"):
            existing = list(merged.get(key) or [])
            seen = set(str(item) for item in existing)
            for item in value or []:
                text = _normalize_string(item)
                if text and text not in seen:
                    existing.append(text)
                    seen.add(text)
            merged[key] = existing
            continue
        merged[key] = value
    return merged


def fetch_live_availability(client):
    """Fetch one conservative live availability snapshot from Grafana."""
    availability = {
        "datasourceUids": [],
        "pluginIds": [],
        "contactPoints": [],
    }
    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        uid = _normalize_string(datasource.get("uid"))
        if uid:
            availability["datasourceUids"].append(uid)

    plugins = client.request_json("/api/plugins")
    if not isinstance(plugins, list):
        raise GrafanaError("Unexpected plugin list response from Grafana.")
    for plugin in plugins:
        if not isinstance(plugin, dict):
            continue
        plugin_id = _normalize_string(plugin.get("id"))
        if plugin_id:
            availability["pluginIds"].append(plugin_id)

    contact_points = client.request_json("/api/v1/provisioning/contact-points")
    if not isinstance(contact_points, list):
        raise GrafanaError("Unexpected contact-point list response from Grafana.")
    for item in contact_points:
        if not isinstance(item, dict):
            continue
        name = _normalize_string(item.get("name"))
        uid = _normalize_string(item.get("uid"))
        if name:
            availability["contactPoints"].append(name)
        elif uid:
            availability["contactPoints"].append(uid)
    return availability


def _emit_text_or_json(document, lines, output):
    if output == "json":
        print(json.dumps(document, indent=2, sort_keys=False))
        return
    for line in lines:
        print(line)


def run_preflight(args):
    desired_specs = _require_resource_list(
        load_json_document(args.desired_file),
        "Desired sync input",
    )
    availability = _load_optional_object_file(
        getattr(args, "availability_file", None),
        "Sync availability input",
    )
    if bool(getattr(args, "fetch_live", False)):
        availability = _merge_availability(
            availability,
            fetch_live_availability(build_client(args)),
        )
    document = build_sync_preflight_document(desired_specs, availability=availability)
    _emit_text_or_json(
        document,
        render_sync_preflight_text(document),
        getattr(args, "output", "text"),
    )
    return 0


def run_assess_alerts(args):
    alert_specs = _require_resource_list(
        load_json_document(args.alerts_file),
        "Alert sync input",
    )
    document = assess_alert_sync_specs(alert_specs)
    _emit_text_or_json(
        document,
        render_alert_sync_assessment_text(document),
        getattr(args, "output", "text"),
    )
    return 0


def run_bundle_preflight(args):
    source_bundle = _require_object(
        load_json_document(args.source_bundle),
        "Source bundle input",
    )
    target_inventory = _require_object(
        load_json_document(args.target_inventory),
        "Target inventory input",
    )
    availability = _load_optional_object_file(
        getattr(args, "availability_file", None),
        "Bundle availability input",
    )
    if bool(getattr(args, "fetch_live", False)):
        availability = _merge_availability(
            availability,
            fetch_live_availability(build_client(args)),
        )
    document = build_bundle_preflight_document(
        source_bundle,
        target_inventory,
        availability=availability,
    )
    _emit_text_or_json(
        document,
        render_bundle_preflight_text(document),
        getattr(args, "output", "text"),
    )
    return 0


def _serialize_apply_intent(intent):
    return {
        "mode": intent.get("mode"),
        "reviewed": bool(intent.get("reviewed")),
        "operations": [
            {
                "kind": operation.kind,
                "identity": operation.identity,
                "title": operation.title,
                "action": operation.action,
                "reason": operation.reason,
                "changedFields": list(operation.changed_fields),
                "managedFields": list(operation.managed_fields),
                "desired": operation.desired,
                "live": operation.live,
                "sourcePath": operation.source_path,
            }
            for operation in intent.get("operations") or ()
        ],
    }


def _resolve_datasource_target(client, operation):
    identity = _normalize_string(operation.identity)
    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        if _normalize_string(datasource.get("uid")) == identity:
            return datasource
    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        if _normalize_string(datasource.get("name")) == identity:
            return datasource
    return None


def _apply_folder_operation(client, operation, allow_folder_delete):
    body = _copy_mapping(operation.desired, "Folder desired body")
    if operation.action == "would-create":
        return client.create_folder(
            uid=operation.identity,
            title=_normalize_string(
                body.get("title"), operation.title or operation.identity
            ),
            parent_uid=_normalize_string(body.get("parentUid")) or None,
        )
    if operation.action == "would-update":
        payload = {
            "uid": operation.identity,
            "title": _normalize_string(
                body.get("title"), operation.title or operation.identity
            ),
        }
        parent_uid = _normalize_string(body.get("parentUid"))
        if parent_uid:
            payload["parentUid"] = parent_uid
        return client.request_json(
            "/api/folders/%s" % parse.quote(operation.identity, safe=""),
            method="PUT",
            payload=payload,
        )
    if operation.action == "would-delete":
        if not allow_folder_delete:
            raise GrafanaError(
                "Refusing live folder delete for %s without --allow-folder-delete."
                % operation.identity
            )
        return client.request_json(
            "/api/folders/%s" % parse.quote(operation.identity, safe=""),
            params={"forceDeleteRules": "false"},
            method="DELETE",
        )
    raise GrafanaError("Unsupported folder sync action %s." % operation.action)


def _apply_dashboard_operation(client, operation):
    if operation.action == "would-delete":
        return client.request_json(
            "/api/dashboards/uid/%s" % parse.quote(operation.identity, safe=""),
            method="DELETE",
        )
    body = _copy_mapping(operation.desired, "Dashboard desired body")
    body["uid"] = operation.identity
    body["title"] = _normalize_string(
        body.get("title"), operation.title or operation.identity
    )
    body.pop("id", None)
    payload = {
        "dashboard": body,
        "overwrite": operation.action == "would-update",
    }
    folder_uid = _normalize_string(body.get("folderUid"))
    if folder_uid:
        payload["folderUid"] = folder_uid
    return client.import_dashboard(payload)


def _apply_datasource_operation(client, operation):
    body = _copy_mapping(operation.desired, "Datasource desired body")
    body["uid"] = (
        _normalize_string(body.get("uid"), operation.identity) or operation.identity
    )
    body["name"] = _normalize_string(
        body.get("name"), operation.title or operation.identity
    )
    if operation.action == "would-create":
        payload = build_datasource_add_payload(body)
        return client.request_json("/api/datasources", method="POST", payload=payload)
    target = _resolve_datasource_target(client, operation)
    if target is None:
        raise GrafanaError(
            "Could not resolve live datasource target %s during sync apply."
            % operation.identity
        )
    if operation.action == "would-update":
        payload = build_modify_datasource_payload(target, body)
        datasource_id = target.get("id")
        if datasource_id is None:
            raise GrafanaError("Datasource sync update requires a live datasource id.")
        return client.request_json(
            "/api/datasources/%s" % datasource_id,
            method="PUT",
            payload=payload,
        )
    if operation.action == "would-delete":
        datasource_id = target.get("id")
        if datasource_id is None:
            raise GrafanaError("Datasource sync delete requires a live datasource id.")
        return client.request_json(
            "/api/datasources/%s" % datasource_id,
            method="DELETE",
        )
    raise GrafanaError("Unsupported datasource sync action %s." % operation.action)


def execute_live_apply(client, operations, allow_folder_delete=False):
    """Apply one gated sync intent to Grafana for supported resource kinds."""
    results = []
    for operation in operations:
        if operation.kind == "folder":
            response = _apply_folder_operation(
                client,
                operation,
                allow_folder_delete=bool(allow_folder_delete),
            )
        elif operation.kind == "dashboard":
            response = _apply_dashboard_operation(client, operation)
        elif operation.kind == "datasource":
            response = _apply_datasource_operation(client, operation)
        elif operation.kind == "alert":
            raise GrafanaError(
                "Live sync apply does not support alert operations yet; keep alerts in plan-only mode."
            )
        else:
            raise GrafanaError("Unsupported sync resource kind %s." % operation.kind)
        results.append(
            {
                "kind": operation.kind,
                "identity": operation.identity,
                "action": operation.action,
                "response": response,
            }
        )
    return {
        "mode": "live-apply",
        "appliedCount": len(results),
        "results": results,
    }


def run_apply(args):
    plan = load_plan_document(args.plan_file)
    if plan.dry_run:
        plan = SyncPlan(
            dry_run=False,
            review_required=plan.review_required,
            reviewed=plan.reviewed,
            allow_prune=plan.allow_prune,
            summary=dict(plan.summary),
            operations=tuple(plan.operations),
        )
    intent = build_apply_intent(plan, approve=bool(getattr(args, "approve", False)))
    if bool(getattr(args, "execute_live", False)):
        live_result = execute_live_apply(
            build_client(args),
            intent.get("operations") or (),
            allow_folder_delete=bool(getattr(args, "allow_folder_delete", False)),
        )
        emit_document(
            live_result,
            output_file=getattr(args, "output_file", None),
        )
        return 0
    emit_document(
        _serialize_apply_intent(intent),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def parse_args(argv=None):
    return build_parser().parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    try:
        if args.command == "plan":
            return run_plan(args)
        if args.command == "review":
            return run_review(args)
        if args.command == "preflight":
            return run_preflight(args)
        if args.command == "assess-alerts":
            return run_assess_alerts(args)
        if args.command == "bundle-preflight":
            return run_bundle_preflight(args)
        return run_apply(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


__all__ = [
    "build_parser",
    "emit_document",
    "load_json_document",
    "load_plan_document",
    "main",
    "parse_args",
    "run_assess_alerts",
    "run_apply",
    "run_bundle_preflight",
    "run_plan",
    "run_preflight",
    "run_review",
    "write_json_document",
]
