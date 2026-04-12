#!/usr/bin/env python3
"""Public Python CLI facade for conservative declarative sync workflows.

Purpose:
- Expose the existing GitOps sync planning scaffold through `grafana-util sync`.
- Keep reviewable plan/apply contracts first-class while also supporting
  conservative live fetch/apply paths for supported resource kinds.

Architecture:
- Parse and validate one local JSON input/output flow per subcommand.
- Delegate plan/review/apply gating to `grafana_utils.gitops_sync`.
- Keep output JSON-first so later CLI/table renderers can reuse one document
  contract.
"""

import argparse
import json
import sys
from pathlib import Path
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
from .alerts.common import GrafanaError as AlertGrafanaError
from .alerts.provisioning import build_rule_import_payload
from .alert_sync_workbench import (
    assess_alert_sync_specs,
    render_alert_sync_assessment_text,
)
from .bundle_preflight_workbench import (
    build_bundle_preflight_document,
    render_bundle_preflight_text,
)
from .gitops_sync import (
    DEFAULT_REVIEW_TOKEN,
    SyncOperation,
    SyncPlan,
    build_apply_intent,
    build_sync_source_bundle_document,
    build_sync_plan,
    mark_plan_reviewed,
    normalize_resource_spec,
    plan_to_document,
    render_sync_apply_intent_text,
    render_sync_plan_text,
    render_sync_source_bundle_text,
)
from .sync_preflight_workbench import (
    SYNC_PREFLIGHT_KIND,
    build_sync_preflight_document,
    render_sync_preflight_text,
)

PLAN_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json\n"
    "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json "
    "--allow-prune --plan-file ./sync-plan.json"
)
SUMMARY_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync summary --desired-file ./desired.json\n"
    "  grafana-util sync summary --desired-file ./desired.json --output json"
)
REVIEW_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync review --plan-file ./sync-plan.json\n"
    "  grafana-util sync review --plan-file ./sync-plan.json --output json"
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
    "  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --availability-file ./availability.json --output json\n\n"
    "  Example availability file:\n"
    "    {\n"
    '      "providerNames": ["vault"],\n'
    '      "secretPlaceholderNames": ["prom-basic-auth"]\n'
    "    }"
)
BUNDLE_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n"
    "  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --datasource-export-file ./datasources/datasources.json --metadata-file ./metadata.json --output json"
)
SYNC_ROOT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util sync summary --desired-file ./desired.json\n"
    '  grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"\n'
    '  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"'
)
SYNC_LOCK_KIND = "grafana-utils-sync-lock"
SYNC_LOCK_SCHEMA_VERSION = 1
SYNC_AUDIT_KIND = "grafana-utils-sync-audit"
SYNC_AUDIT_SCHEMA_VERSION = 1
SYNC_PROMOTION_PREFLIGHT_KIND = "grafana-utils-sync-promotion-preflight"
SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION = 1
CI_COMMAND_HELP = {
    "summary": "Summarize local desired workspace resources from JSON.",
    "plan": "Build a staged workspace plan from local desired and live JSON files.",
    "mark-reviewed": "Mark a staged workspace plan as reviewed.",
    "input-test": "Build a staged workspace input-test document from local JSON.",
    "audit": "Audit managed resources against checksum lock and live state.",
    "alert-readiness": (
        "Assess alert sync specs for candidate, plan-only, and blocked states."
    ),
    "package-test": "Build a staged workspace package-test document from local JSON.",
    "promote-test": (
        "Build a staged promotion handoff from a workspace package and target inventory."
    ),
}


def add_document_input_group(parser, *definitions):
    """Add document input group implementation."""
    group = parser.add_argument_group("Input Options")
    for definition in definitions:
        flags = definition[0]
        kwargs = definition[1]
        group.add_argument(*flags, **kwargs)


def add_runtime_group(parser):
    """Add runtime group implementation."""
    return parser.add_argument_group("Runtime Options")


def add_output_group(parser):
    """Add output group implementation."""
    return parser.add_argument_group("Output Options")


def add_apply_control_group(parser):
    """Add apply control group implementation."""
    return parser.add_argument_group("Apply Control Options")


def build_parser(prog=None):
    """Build the sync CLI parser."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1317
    #   Downstream callees: 106, 115, 120, 125

    parser = argparse.ArgumentParser(
        prog=prog or "grafana-util sync",
        description=(
            "Build, review, and gate declarative Grafana sync plans with "
            "optional live Grafana fetch/apply paths."
        ),
        epilog=SYNC_ROOT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command")
    subparsers.required = True

    scan_parser = subparsers.add_parser(
        "scan",
        help="Scan staged workspace artifacts and summarize local resource state.",
        epilog=SUMMARY_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    test_parser = subparsers.add_parser(
        "test",
        help="Test whether staged workspace artifacts are structurally safe to continue.",
        epilog=PREFLIGHT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    preview_parser = subparsers.add_parser(
        "preview",
        help="Preview what would change from staged workspace inputs.",
        epilog=PLAN_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    summary_parser = subparsers.add_parser(
        "summary",
        help="Summarize local desired sync resources from JSON.",
        epilog=SUMMARY_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_document_input_group(
        summary_parser,
        (
            ("--desired-file",),
            {
                "required": True,
                "help": "JSON file containing the desired managed resource list.",
            },
        ),
    )
    add_output_group(summary_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the summary document as text or json (default: text).",
    )

    plan_parser = subparsers.add_parser(
        "plan",
        help="Build one review-required sync plan from desired/live JSON files.",
        epilog=PLAN_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_document_input_group(
        plan_parser,
        (
            ("--desired-file",),
            {
                "required": True,
                "help": "JSON file containing the desired managed resource list.",
            },
        ),
        (
            ("--live-file",),
            {
                "default": None,
                "help": "JSON file containing the current live resource list.",
            },
        ),
    )
    runtime_group = add_runtime_group(plan_parser)
    runtime_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Read the current live state directly from Grafana instead of --live-file.",
    )
    add_common_cli_args(plan_parser)
    runtime_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    runtime_group.add_argument(
        "--page-size",
        type=int,
        default=500,
        help="Dashboard search page size when --fetch-live is active.",
    )
    add_apply_control_group(plan_parser).add_argument(
        "--trace-id",
        default=None,
        help="Optional stable trace id to carry through staged plan/review/apply files.",
    )
    add_apply_control_group(plan_parser).add_argument(
        "--allow-prune",
        action="store_true",
        help="Treat live resources missing from desired state as would-delete instead of unmanaged.",
    )
    add_output_group(plan_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the plan document as text or json (default: text).",
    )
    add_output_group(plan_parser).add_argument(
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
    add_document_input_group(
        review_parser,
        (
            ("--plan-file",),
            {
                "required": True,
                "help": "Input JSON plan document produced by `grafana-util sync plan`.",
            },
        ),
    )
    add_apply_control_group(review_parser).add_argument(
        "--review-token",
        default=DEFAULT_REVIEW_TOKEN,
        help="Explicit review token required to mark the plan reviewed.",
    )
    add_apply_control_group(review_parser).add_argument(
        "--reviewed-by",
        default=None,
        help="Optional reviewer identity to record in the reviewed plan.",
    )
    add_apply_control_group(review_parser).add_argument(
        "--reviewed-at",
        default=None,
        help="Optional staged reviewed-at value to record in the reviewed plan.",
    )
    add_apply_control_group(review_parser).add_argument(
        "--review-note",
        default=None,
        help="Optional review note to record in the reviewed plan.",
    )
    add_output_group(review_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the reviewed plan document as text or json (default: text).",
    )
    add_output_group(review_parser).add_argument(
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
    add_document_input_group(
        preflight_parser,
        (
            ("--desired-file",),
            {
                "required": True,
                "help": "JSON file containing the desired managed resource list.",
            },
        ),
        (
            ("--availability-file",),
            {
                "default": None,
                "help": "Optional JSON object file containing availability hints such as datasourceUids, pluginIds, and contactPoints.",
            },
        ),
    )
    runtime_group = add_runtime_group(preflight_parser)
    runtime_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch availability hints from Grafana instead of relying only on --availability-file.",
    )
    add_common_cli_args(preflight_parser)
    runtime_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    add_output_group(preflight_parser).add_argument(
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
    add_document_input_group(
        assess_alerts_parser,
        (
            ("--alerts-file",),
            {
                "required": True,
                "help": "JSON file containing the alert sync resource list.",
            },
        ),
    )
    add_output_group(assess_alerts_parser).add_argument(
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
    add_document_input_group(
        bundle_preflight_parser,
        (
            ("--source-bundle",),
            {
                "required": True,
                "help": "JSON file containing the staged multi-resource source bundle.",
            },
        ),
        (
            ("--target-inventory",),
            {
                "required": True,
                "help": "JSON file containing the staged target inventory snapshot.",
            },
        ),
        (
            ("--availability-file",),
            {
                "default": None,
                "help": "Optional JSON object file containing staged availability hints.",
            },
        ),
    )
    runtime_group = add_runtime_group(bundle_preflight_parser)
    runtime_group.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch availability hints from Grafana instead of relying only on --availability-file.",
    )
    add_common_cli_args(bundle_preflight_parser)
    runtime_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    add_output_group(bundle_preflight_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the bundle preflight document as text or json (default: text).",
    )

    bundle_parser = subparsers.add_parser(
        "package",
        aliases=["bundle"],
        help="Package exported dashboards, alerting resources, datasource inventory, and metadata into one portable source bundle. `bundle` is accepted as a compatibility alias.",
        epilog=BUNDLE_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_document_input_group(
        bundle_parser,
        (
            ("--dashboard-export-dir",),
            {
                "default": None,
                "help": "Path to one existing dashboard raw export directory such as ./dashboards/raw.",
            },
        ),
        (
            ("--alert-export-dir",),
            {
                "default": None,
                "help": "Path to one existing alert raw export directory such as ./alerts/raw.",
            },
        ),
        (
            ("--datasource-export-file",),
            {
                "default": None,
                "help": "Optional standalone datasource inventory JSON file to include or prefer over dashboards/raw/datasources.json.",
            },
        ),
        (
            ("--metadata-file",),
            {
                "default": None,
                "help": "Optional JSON object file containing extra bundle metadata.",
            },
        ),
    )
    add_output_group(bundle_parser).add_argument(
        "--output-file",
        default=None,
        help="Optional JSON file path to write the source bundle artifact.",
    )
    add_output_group(bundle_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the source bundle as text or json (default: text).",
    )

    apply_parser = subparsers.add_parser(
        "apply",
        help="Build a gated apply intent from a reviewed plan, optionally executing it live.",
        epilog=APPLY_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    add_document_input_group(
        apply_parser,
        (
            ("--plan-file",),
            {
                "required": True,
                "help": "Input JSON plan document, typically already marked reviewed.",
            },
        ),
    )
    control_group = add_apply_control_group(apply_parser)
    control_group.add_argument(
        "--preflight-file",
        default=None,
        help="Optional JSON file containing a staged sync preflight document.",
    )
    control_group.add_argument(
        "--bundle-preflight-file",
        default=None,
        help="Optional JSON file containing a staged sync bundle-preflight document.",
    )
    control_group.add_argument(
        "--approve",
        action="store_true",
        help="Explicit acknowledgement required before an apply intent or live execution is emitted.",
    )
    add_common_cli_args(apply_parser)
    runtime_group = add_runtime_group(apply_parser)
    runtime_group.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --execute-live is active.",
    )
    control_group.add_argument(
        "--execute-live",
        action="store_true",
        help="Apply supported sync operations to Grafana after review and approval checks pass.",
    )
    control_group.add_argument(
        "--allow-folder-delete",
        action="store_true",
        help="Allow live deletion of folders when a reviewed plan includes would-delete folder operations.",
    )
    control_group.add_argument(
        "--applied-by",
        default=None,
        help="Optional apply actor identity to record in the apply intent.",
    )
    control_group.add_argument(
        "--applied-at",
        default=None,
        help="Optional staged applied-at value to record in the apply intent.",
    )
    control_group.add_argument(
        "--approval-reason",
        default=None,
        help="Optional approval reason to record in the apply intent.",
    )
    control_group.add_argument(
        "--apply-note",
        default=None,
        help="Optional apply note to record in the apply intent.",
    )
    add_output_group(apply_parser).add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the apply intent document as text or json (default: text).",
    )
    add_output_group(apply_parser).add_argument(
        "--output-file",
        default=None,
        help="Optional JSON file path to write the apply intent document.",
    )
    ci_parser = subparsers.add_parser(
        "ci",
        help="Run CI-oriented sync workflows and lower-level review contracts.",
        add_help=False,
    )
    ci_subparsers = ci_parser.add_subparsers(dest="ci_command")
    ci_subparsers.required = True
    for ci_command, ci_help in CI_COMMAND_HELP.items():
        if ci_command in ("audit", "promote-test"):
            continue
        ci_subparsers.add_parser(
            ci_command,
            help=ci_help,
            add_help=False,
        )

    ci_audit_parser = ci_subparsers.add_parser(
        "audit",
        help=CI_COMMAND_HELP["audit"],
        add_help=False,
    )
    ci_audit_parser.add_argument(
        "--managed-file",
        default=None,
        help=(
            "Optional JSON file containing the managed desired sync resource list "
            "used to define audit scope and managed fields."
        ),
    )
    ci_audit_parser.add_argument(
        "--lock-file",
        default=None,
        help="Optional JSON file containing a staged sync lock document to compare against.",
    )
    ci_audit_parser.add_argument(
        "--live-file",
        default=None,
        help="Optional JSON file containing the current live sync resource list.",
    )
    ci_audit_parser.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch the current live state directly from Grafana instead of --live-file.",
    )
    ci_audit_parser.add_argument(
        "--write-lock",
        default=None,
        help="Optional JSON file path to write the newly generated lock snapshot.",
    )
    ci_audit_parser.add_argument(
        "--fail-on-drift",
        action="store_true",
        help="Fail the command when drift is detected.",
    )
    ci_audit_parser.add_argument(
        "--page-size",
        type=int,
        default=500,
        help="Dashboard search page size when --fetch-live is active.",
    )
    ci_audit_parser.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    ci_audit_parser.add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the audit document as text or json (default: text).",
    )
    ci_audit_parser.add_argument(
        "--output-format",
        dest="output",
        choices=("text", "json"),
        default="text",
        help="Render the audit document as text or json (default: text).",
    )
    ci_audit_parser.add_argument(
        "--interactive",
        action="store_true",
        help="Open an interactive terminal browser over drift rows.",
    )
    add_common_cli_args(ci_audit_parser)

    ci_promote_test_parser = ci_subparsers.add_parser(
        "promote-test",
        help=CI_COMMAND_HELP["promote-test"],
        add_help=False,
    )
    ci_promote_test_parser.add_argument(
        "--source-bundle",
        required=True,
        help="JSON file containing the staged workspace package document.",
    )
    ci_promote_test_parser.add_argument(
        "--target-inventory",
        required=True,
        help="JSON file containing the staged target inventory snapshot.",
    )
    ci_promote_test_parser.add_argument(
        "--mapping-file",
        default=None,
        help="Optional JSON object file containing explicit cross-environment promotion mappings.",
    )
    ci_promote_test_parser.add_argument(
        "--availability-file",
        default=None,
        help="Optional JSON object file containing staged availability hints.",
    )
    ci_promote_test_parser.add_argument(
        "--fetch-live",
        action="store_true",
        help="Fetch availability hints from Grafana instead of relying only on --availability-file.",
    )
    ci_promote_test_parser.add_argument(
        "--org-id",
        default=None,
        help="Optional Grafana org id used when --fetch-live is active.",
    )
    ci_promote_test_parser.add_argument(
        "--output",
        choices=("text", "json"),
        default="text",
        help="Render the promotion-preflight document as text or json (default: text).",
    )
    ci_promote_test_parser.add_argument(
        "--output-format",
        dest="output",
        choices=("text", "json"),
        default="text",
        help="Render the promotion-preflight document as text or json (default: text).",
    )
    ci_promote_test_parser.add_argument(
        "--output-file",
        default=None,
        help="Optional JSON file path to write the staged promotion-preflight artifact.",
    )
    add_common_cli_args(ci_promote_test_parser)

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
    """Internal helper for require object."""
    if not isinstance(document, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    return document


def _is_json_int(value):
    """Internal helper for strict JSON integer checks."""
    return isinstance(value, int) and not isinstance(value, bool)


def _require_resource_list(document, label):
    """Internal helper for require resource list."""
    if not isinstance(document, list):
        raise GrafanaError("%s must be a JSON array." % label)
    return document


def _build_ci_ci_arg_parser(prog):
    """Build one small parser for ci compatibility command arguments."""

    class _NoExitArgumentParser(argparse.ArgumentParser):
        def error(self, message):
            raise GrafanaError(message)

    parser = _NoExitArgumentParser(prog=prog, add_help=False)
    add_common_cli_args(parser)
    return parser


def _canonicalize_json(value):
    """Canonicalize JSON value for deterministic checksum calculation."""
    if isinstance(value, dict):
        return {
            str(key): _canonicalize_json(value[key]) for key in sorted(value.keys(), key=str)
        }
    if isinstance(value, list):
        return [_canonicalize_json(item) for item in value]
    return value


def _checksum_json_value(value):
    """Checksum one JSON value using stable FNV-1a digest."""
    return _fnv1a64_hex(json.dumps(_canonicalize_json(value), separators=(",", "")))


def _coerce_string_map(value, label):
    """Normalize a string-keyed mapping value."""
    if value is None:
        return {}
    if not isinstance(value, dict):
        raise GrafanaError("%s must be a JSON object." % label)
    normalized = {}
    for key, mapped in value.items():
        source_key = _normalize_optional_text(key)
        if not source_key:
            continue
        mapped_value = _normalize_optional_text(mapped)
        normalized[source_key] = mapped_value
    return normalized


def _build_sync_lock_snapshot(body, managed_fields):
    """Build one stable snapshot from a managed subset of a resource body."""
    source_fields = []
    if managed_fields:
        seen = set()
        for field in managed_fields:
            text = _normalize_optional_text(field)
            if not text or text in seen:
                continue
            seen.add(text)
            source_fields.append(text)
    else:
        source_fields = sorted(body.keys())
    return {field: body.get(field) for field in source_fields}


def _build_sync_lock_document_from_specs(managed_specs, live_specs):
    """Build one lock snapshot from managed specs and raw live specs."""
    live_index = {}
    for spec in [normalize_resource_spec(item) for item in live_specs]:
        live_index[(spec.kind, spec.identity)] = spec

    resources = []
    present_count = 0
    missing_live_count = 0
    for spec in [normalize_resource_spec(item) for item in managed_specs]:
        live = live_index.get((spec.kind, spec.identity))
        if live is None:
            missing_live_count += 1
            resources.append(
                {
                    "kind": spec.kind,
                    "identity": spec.identity,
                    "title": spec.title,
                    "status": "missing-live",
                    "managedFields": list(spec.managed_fields),
                    "checksum": None,
                    "snapshot": None,
                    "sourcePath": spec.source_path,
                }
            )
            continue

        present_count += 1
        snapshot = _build_sync_lock_snapshot(live.body, spec.managed_fields)
        resources.append(
            {
                "kind": spec.kind,
                "identity": spec.identity,
                "title": live.title,
                "status": "present",
                "managedFields": list(spec.managed_fields),
                "checksum": _checksum_json_value(snapshot),
                "snapshot": snapshot,
                "sourcePath": spec.source_path,
            }
        )

    return {
        "kind": SYNC_LOCK_KIND,
        "schemaVersion": SYNC_LOCK_SCHEMA_VERSION,
        "summary": {
            "resourceCount": len(resources),
            "presentCount": present_count,
            "missingLiveCount": missing_live_count,
        },
        "resources": resources,
    }


def _load_lock_resources(document, label):
    """Load lock resource rows from a lock document."""
    lock = _require_object(document, label)
    if _normalize_optional_text(lock.get("kind")) != SYNC_LOCK_KIND:
        raise GrafanaError("%s kind is not supported." % label)
    return _require_resource_list(lock.get("resources"), "%s resources" % label)


def _load_source_mapping_from_promotion_file(mapping_file):
    """Load optional mapping input for promote-test."""
    if not mapping_file:
        return ({}, {}, {}, {}, {})

    mapping = _require_object(load_json_document(mapping_file), "Sync promotion mapping input")
    kind = _normalize_optional_text(mapping.get("kind"))
    if kind and kind != "grafana-utils-sync-promotion-mapping":
        raise GrafanaError("Sync promotion mapping input kind is not supported.")
    schema_version = mapping.get("schemaVersion")
    if schema_version is not None and schema_version != 1:
        raise GrafanaError("Sync promotion mapping schemaVersion must be 1.")

    folders = _coerce_string_map(mapping.get("folders"), "Sync promotion folder mapping")
    datasource_uids = _coerce_string_map(
        (mapping.get("datasources") or {}).get("uids"),
        "Sync promotion datasource uid mapping",
    )
    datasource_names = _coerce_string_map(
        (mapping.get("datasources") or {}).get("names"),
        "Sync promotion datasource name mapping",
    )
    source_environment = _normalize_optional_text(
        (mapping.get("metadata") or {}).get("sourceEnvironment")
    )
    target_environment = _normalize_optional_text(
        (mapping.get("metadata") or {}).get("targetEnvironment")
    )
    mapping_summary = {
        "kind": kind,
        "schemaVersion": schema_version,
        "sourceEnvironment": source_environment,
        "targetEnvironment": target_environment,
        "folderMappingCount": len(folders),
        "datasourceUidMappingCount": len(datasource_uids),
        "datasourceNameMappingCount": len(datasource_names),
    }
    return folders, datasource_uids, datasource_names, {
        "folders": folders,
        "datasource_uids": datasource_uids,
        "datasource_names": datasource_names,
    }, mapping_summary


def _extract_dashboard_body(resource):
    """Extract dashboard body consistently for preflight checks."""
    body = resource.get("body")
    if isinstance(body, dict):
        return body
    return dict(resource)


def _extract_promotion_check_values(value):
    """Build one normalized list of string values from reference field."""
    if not isinstance(value, list):
        return []
    normalized = []
    for item in value:
        text = _normalize_optional_text(item)
        if text:
            normalized.append(text)
    return normalized


def _classify_sync_promotion_mapping_check(
    kind,
    identity,
    source_value,
    mapped_value,
    mapping_source,
    target_values,
    missing_detail,
):
    """Classify one source/target mapping check."""
    if not source_value:
        return None
    if source_value in target_values:
        return {
            "kind": kind,
            "identity": identity,
            "sourceValue": source_value,
            "targetValue": source_value,
            "resolution": "direct-match",
            "mappingSource": "inventory",
            "status": "direct",
            "detail": "Target inventory already contains the same identifier.",
            "blocking": False,
        }
    if mapped_value and mapped_value in target_values:
        return {
            "kind": kind,
            "identity": identity,
            "sourceValue": source_value,
            "targetValue": mapped_value,
            "resolution": "explicit-map",
            "mappingSource": mapping_source,
            "status": "mapped",
            "detail": "Promotion mapping resolves this source identifier onto the target inventory.",
            "blocking": False,
        }
    return {
        "kind": kind,
        "identity": identity,
        "sourceValue": source_value,
        "targetValue": mapped_value,
        "resolution": "missing-map",
        "mappingSource": mapping_source,
        "status": "missing-target",
        "detail": missing_detail,
        "blocking": True,
    }


def _build_ci_promotion_check_rows(
    source_bundle,
    target_inventory,
    folder_mapping,
    datasource_uid_mapping,
    datasource_name_mapping,
):
    """Build one list of promotion remap checks."""
    target_folders = {
        _normalize_optional_text(item.get("uid"))
        for item in target_inventory.get("folders") or []
        if _normalize_optional_text(item.get("uid"))
    }
    target_datasource_uids = {
        _normalize_optional_text(item.get("uid"))
        for item in target_inventory.get("datasources") or []
        if _normalize_optional_text(item.get("uid"))
    }
    target_datasource_names = {
        _normalize_optional_text(item.get("name") or item.get("title"))
        for item in target_inventory.get("datasources") or []
        if _normalize_optional_text(item.get("name") or item.get("title"))
    }
    checks = []

    for dashboard in source_bundle.get("dashboards") or []:
        if not isinstance(dashboard, dict):
            continue
        dashboard_uid = _normalize_optional_text(dashboard.get("uid")) or "dashboard"
        folder_uid = _normalize_optional_text(dashboard.get("folderUid"))
        check = _classify_sync_promotion_mapping_check(
            "folder-remap",
            dashboard_uid,
            folder_uid,
            _normalize_optional_text(folder_mapping.get(folder_uid)),
            "folders",
            target_folders,
            "Dashboard folder UID is missing from the target inventory and has no valid promotion mapping.",
        )
        if check is not None:
            checks.append(check)

        body = _extract_dashboard_body(dashboard)
        for datasource_uid in _extract_promotion_check_values(
            body.get("datasourceUids")
        ):
            check = _classify_sync_promotion_mapping_check(
                "datasource-uid-remap",
                dashboard_uid,
                datasource_uid,
                _normalize_optional_text(datasource_uid_mapping.get(datasource_uid)),
                "datasources.uids",
                target_datasource_uids,
                "Datasource UID is missing from the target inventory and has no valid promotion mapping.",
            )
            if check is not None:
                checks.append(check)
        for datasource_name in _extract_promotion_check_values(
            body.get("datasourceNames")
        ):
            check = _classify_sync_promotion_mapping_check(
                "datasource-name-remap",
                dashboard_uid,
                datasource_name,
                _normalize_optional_text(datasource_name_mapping.get(datasource_name)),
                "datasources.names",
                target_datasource_names,
                "Datasource name is missing from the target inventory and has no valid promotion mapping.",
            )
            if check is not None:
                checks.append(check)

    for alert in source_bundle.get("alerts") or []:
        if not isinstance(alert, dict):
            continue
        alert_uid = _normalize_optional_text(alert.get("uid")) or "alert"
        body = _extract_dashboard_body(alert)
        for datasource_uid in _extract_promotion_check_values(
            body.get("datasourceUids")
        ):
            if datasource_uid in ("__expr__", "__dashboard__"):
                continue
            check = _classify_sync_promotion_mapping_check(
                "alert-datasource-uid-remap",
                alert_uid,
                datasource_uid,
                _normalize_optional_text(datasource_uid_mapping.get(datasource_uid)),
                "datasources.uids",
                target_datasource_uids,
                "Alert datasource UID is missing from the target inventory and has no valid promotion mapping.",
            )
            if check is not None:
                checks.append(check)
        for datasource_name in _extract_promotion_check_values(
            body.get("datasourceNames")
        ):
            check = _classify_sync_promotion_mapping_check(
                "alert-datasource-name-remap",
                alert_uid,
                datasource_name,
                _normalize_optional_text(datasource_name_mapping.get(datasource_name)),
                "datasources.names",
                target_datasource_names,
                "Alert datasource name is missing from the target inventory and has no valid promotion mapping.",
            )
            if check is not None:
                checks.append(check)
    return checks


def _summarize_sync_promotion_checks(checks):
    """Build promotion check summary counts."""
    return {
        "folderRemapCount": len([item for item in checks if item.get("kind") == "folder-remap"]),
        "datasourceUidRemapCount": len(
            [item for item in checks if item.get("kind") in ("datasource-uid-remap", "alert-datasource-uid-remap")]
        ),
        "datasourceNameRemapCount": len(
            [item for item in checks if item.get("kind") in ("datasource-name-remap", "alert-datasource-name-remap")]
        ),
        "resolvedCount": len([item for item in checks if not item.get("blocking")]),
        "directCount": len([item for item in checks if item.get("status") == "direct"]),
        "mappedCount": len([item for item in checks if item.get("status") == "mapped"]),
        "missingTargetCount": len([item for item in checks if item.get("status") == "missing-target"]),
    }


def _bundle_preflight_blocking_count(bundle_preflight_document):
    """Compute promotion blocking baseline from bundle preflight summary."""
    summary = _require_object(
        bundle_preflight_document.get("summary"), "Bundle preflight summary"
    )
    return (
        int(summary.get("syncBlockingCount") or 0)
        + int(summary.get("providerBlockingCount") or 0)
        + int(summary.get("secretBlockingCount") or summary.get("secretPlaceholderBlockingCount") or 0)
        + int(summary.get("alertBlockedCount") or summary.get("alertBlocked") or summary.get("alertArtifactBlockedCount") or 0)
    )


def _resource_count_from_bundle_summary(source_bundle):
    """Read target resource count from bundle summary if available."""
    summary = _require_object(source_bundle.get("summary"), "Bundle summary") if isinstance(
        source_bundle, dict
    ) else None
    if summary is not None:
        total = 0
        for value in summary.values():
            if _is_json_int(value):
                total += int(value)
        if total > 0:
            return total
    return int(len(source_bundle.get("folders") or [])) + int(
        len(source_bundle.get("datasources") or [])
    ) + int(len(source_bundle.get("dashboards") or [])) + int(
        len(source_bundle.get("alerts") or [])
    )


def build_sync_lock_document(managed_specs, live_specs):
    """Build one compatibility sync lock document."""
    return _build_sync_lock_document_from_specs(managed_specs, live_specs)


def build_sync_lock_document_from_lock(lock_document, live_specs):
    """Build one compatibility sync lock from a previous lock and live specs."""
    lock_resources = _load_lock_resources(lock_document, "Sync lock input")

    managed_specs = []
    for resource in lock_resources:
        if not isinstance(resource, dict):
            raise GrafanaError("Sync lock resource must be a JSON object.")
        kind = _normalize_optional_text(resource.get("kind"))
        identity = _normalize_optional_text(resource.get("identity"))
        if not kind or not identity:
            raise GrafanaError("Sync lock resources require non-empty kind and identity.")
        managed_specs.append(
            {
                "kind": kind,
                "title": _normalize_optional_text(resource.get("title", identity)),
                "uid": identity,
                "name": identity,
                "identity": identity,
                "body": _copy_mapping(resource.get("snapshot"), "Sync lock resource snapshot")
                if resource.get("snapshot") not in (None,)
                else {},
                "managedFields": resource.get("managedFields") or (),
                "sourcePath": _normalize_optional_text(resource.get("sourcePath")),
            }
        )
    return _build_sync_lock_document_from_specs(managed_specs, live_specs)


def build_sync_audit_document(current_lock_document, baseline_lock_document=None):
    """Build one compatibility sync audit document."""
    baseline_resources = None
    if baseline_lock_document is not None:
        baseline_resources = {
            (
                _normalize_optional_text(resource.get("kind")),
                _normalize_optional_text(resource.get("identity")),
            ): resource
            for resource in _load_lock_resources(
                baseline_lock_document, "Sync lock baseline input"
            )
        }

    drifts = []
    current_resources = _load_lock_resources(current_lock_document, "Sync lock input")
    in_sync_count = 0
    missing_lock_count = 0
    missing_live_count = 0
    current_present_count = 0
    current_missing_count = 0

    for resource in current_resources:
        if not isinstance(resource, dict):
            raise GrafanaError("Sync lock resource must be a JSON object.")
        kind = _normalize_optional_text(resource.get("kind"))
        identity = _normalize_optional_text(resource.get("identity"))
        title = _normalize_optional_text(resource.get("title")) or identity
        current_status = _normalize_optional_text(resource.get("status"))
        if current_status == "present":
            current_present_count += 1
        else:
            current_missing_count += 1
            if current_status == "missing-live":
                missing_live_count += 1

        current_checksum = resource.get("checksum")
        current_snapshot = resource.get("snapshot")
        baseline = None
        baseline_status = None
        baseline_checksum = None
        baseline_snapshot = None
        if baseline_resources is not None:
            baseline = baseline_resources.get((kind, identity))
            if baseline is not None:
                baseline_status = _normalize_optional_text(baseline.get("status"))
                baseline_checksum = baseline.get("checksum")
                baseline_snapshot = baseline.get("snapshot")

        drift_status = None
        if current_status != "present":
            drift_status = "missing-live"
        elif baseline_resources is None:
            drift_status = None
        elif baseline is None:
            drift_status = "missing-lock"
            missing_lock_count += 1
        elif baseline_checksum != current_checksum or baseline_status != current_status:
            drift_status = "drift-detected"

        if drift_status is None:
            in_sync_count += 1
            continue

        drift_fields = set()
        if isinstance(current_snapshot, dict):
            drift_fields.update(current_snapshot.keys())
        if isinstance(baseline_snapshot, dict):
            drift_fields.update(baseline_snapshot.keys())
        drifted_fields = [
            field
            for field in sorted(drift_fields)
            if (current_snapshot or {}).get(field)
            != (baseline_snapshot or {}).get(field)
        ]
        drifts.append(
            {
                "kind": kind,
                "identity": identity,
                "title": title,
                "status": drift_status,
                "baselineStatus": _normalize_optional_text(baseline_status),
                "currentStatus": current_status,
                "baselineChecksum": baseline_checksum,
                "currentChecksum": current_checksum,
                "driftedFields": drifted_fields,
                "sourcePath": _normalize_optional_text(resource.get("sourcePath")),
            }
        )

    return {
        "kind": SYNC_AUDIT_KIND,
        "schemaVersion": SYNC_AUDIT_SCHEMA_VERSION,
        "summary": {
            "managedCount": len(current_resources),
            "baselineCount": len(baseline_resources or {}),
            "currentPresentCount": current_present_count,
            "currentMissingCount": current_missing_count,
            "inSyncCount": in_sync_count,
            "driftCount": len(drifts),
            "missingLockCount": missing_lock_count,
            "missingLiveCount": missing_live_count,
        },
        "currentLock": current_lock_document,
        "baselineLock": baseline_lock_document,
        "drifts": drifts,
    }


def render_sync_audit_text(document):
    """Render one compatibility sync audit document."""
    if _normalize_optional_text(document.get("kind")) != SYNC_AUDIT_KIND:
        raise GrafanaError("Sync audit document kind is not supported.")
    summary = _require_object(document.get("summary"), "Sync audit document summary")
    lines = [
        "Sync audit",
        "Managed: %s baseline=%s current-present=%s current-missing=%s"
        % (
            int(summary.get("managedCount") or 0),
            int(summary.get("baselineCount") or 0),
            int(summary.get("currentPresentCount") or 0),
            int(summary.get("currentMissingCount") or 0),
        ),
        "Drift: count=%s in-sync=%s missing-lock=%s missing-live=%s"
        % (
            int(summary.get("driftCount") or 0),
            int(summary.get("inSyncCount") or 0),
            int(summary.get("missingLockCount") or 0),
            int(summary.get("missingLiveCount") or 0),
        ),
    ]
    for drift in _require_resource_list(document.get("drifts"), "Sync audit drift list"):
        if not isinstance(drift, dict):
            raise GrafanaError("Sync audit drift row must be a JSON object.")
        fields = drift.get("driftedFields") or []
        if not isinstance(fields, list):
            fields = []
        lines.append(
            "- [%s] %s %s fields=%s"
            % (
                _normalize_optional_text(drift.get("status")) or "unknown",
                _normalize_optional_text(drift.get("kind")) or "unknown",
                _normalize_optional_text(drift.get("identity")) or "unknown",
                ",".join(fields) if fields else "-",
            )
        )
    return lines


def build_ci_promotion_test_document(
    source_bundle,
    target_inventory,
    availability,
    mapping_file_path,
):
    """Build one compatibility sync promotion preflight document."""
    source_bundle = _require_object(source_bundle, "Workspace package input")
    target_inventory = _require_object(target_inventory, "Sync target inventory input")
    availability = _require_object(availability, "Sync availability input")
    (
        mapping_folders,
        mapping_uid,
        mapping_name,
        _mapping,
        mapping_summary,
    ) = _load_source_mapping_from_promotion_file(
        mapping_file_path
    )
    bundle_preflight = build_bundle_preflight_document(
        source_bundle, target_inventory, availability=availability
    )
    checks = _build_ci_promotion_check_rows(
        source_bundle,
        target_inventory,
        mapping_folders,
        mapping_uid,
        mapping_name,
    )
    return _build_ci_promotion_test_document_from_parts(
        bundle_preflight=bundle_preflight,
        checks=checks,
        mapping_summary=mapping_summary,
        source_summary=source_bundle.get("summary") if isinstance(source_bundle.get("summary"), dict) else {},
    )


def _normalize_mapping_keys(mapping):
    """Normalize mapping key-value strings for stable lookups."""
    return _coerce_string_map(mapping, "Sync promotion mapping")


def _build_ci_promotion_test_document_from_parts(
    bundle_preflight,
    checks,
    mapping_summary,
    source_summary=None,
):
    """Build one compatibility sync promotion preflight document from built parts."""
    bundle_preflight = _require_object(bundle_preflight, "Bundle preflight input")
    checks = list(check for check in checks if isinstance(check, dict))
    check_summary = _summarize_sync_promotion_checks(checks)
    resolved_checks = [item for item in checks if not item.get("blocking")]
    blocking_checks = [item for item in checks if item.get("blocking")]
    direct_match_count = int(check_summary.get("directCount") or 0)
    mapped_count = int(check_summary.get("mappedCount") or 0)
    missing_mapping_count = int(check_summary.get("missingTargetCount") or 0)
    bundle_blocking_count = _bundle_preflight_blocking_count(bundle_preflight)
    blocking_count = bundle_blocking_count + missing_mapping_count

    if source_summary is None:
        source_summary = {}
    source_summary = _require_object(source_summary, "Bundle summary")

    return {
        "kind": SYNC_PROMOTION_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "resourceCount": int(_resource_count_from_bundle_summary({"summary": source_summary})),
            "directMatchCount": direct_match_count,
            "mappedCount": mapped_count,
            "missingMappingCount": missing_mapping_count,
            "bundleBlockingCount": int(bundle_blocking_count),
            "blockingCount": blocking_count,
        },
        "bundlePreflight": bundle_preflight,
        "mappingSummary": {
            "mappingKind": _normalize_optional_text(mapping_summary.get("kind")),
            "mappingSchemaVersion": mapping_summary.get("schemaVersion"),
            "sourceEnvironment": _normalize_optional_text(
                mapping_summary.get("sourceEnvironment")
            ),
            "targetEnvironment": _normalize_optional_text(
                mapping_summary.get("targetEnvironment")
            ),
            "folderMappingCount": int(mapping_summary.get("folderMappingCount") or 0),
            "datasourceUidMappingCount": int(
                mapping_summary.get("datasourceUidMappingCount") or 0
            ),
            "datasourceNameMappingCount": int(
                mapping_summary.get("datasourceNameMappingCount") or 0
            ),
        },
        "checkSummary": check_summary,
        "handoffSummary": {
            "reviewRequired": True,
            "readyForReview": blocking_count == 0,
            "nextStage": "review" if blocking_count == 0 else "resolve-blockers",
            "blockingCount": blocking_count,
            "reviewInstruction": (
                "promotion handoff is ready to move into review"
                if blocking_count == 0
                else "promotion handoff is blocked until the listed remaps and bundle issues are cleared"
            ),
        },
        "continuationSummary": {
            "stagedOnly": True,
            "liveMutationAllowed": False,
            "readyForContinuation": blocking_count == 0,
            "nextStage": "staged-apply-continuation" if blocking_count == 0 else "resolve-blockers",
            "resolvedCount": int(len(resolved_checks)),
            "blockingCount": blocking_count,
            "continuationInstruction": (
                "reviewed remaps can continue into a staged apply continuation without enabling live mutation"
                if blocking_count == 0
                else "keep the promotion staged until blockers clear; do not enter the apply continuation"
            ),
        },
        "checks": checks,
        "resolvedChecks": resolved_checks,
        "blockingChecks": blocking_checks,
    }


def render_sync_promotion_preflight_text(document):
    """Render one compatibility sync promotion preflight document."""
    if _normalize_optional_text(document.get("kind")) != SYNC_PROMOTION_PREFLIGHT_KIND:
        raise GrafanaError("Sync promotion preflight document kind is not supported.")
    summary = _require_object(document.get("summary"), "Sync promotion preflight summary")
    mapping_summary = _require_object(
        document.get("mappingSummary"), "Sync promotion mapping summary"
    )
    check_summary = _require_object(
        document.get("checkSummary"), "Sync promotion check summary"
    )
    handoff_summary = _require_object(
        document.get("handoffSummary"), "Sync promotion handoff summary"
    )
    continuation_summary = _require_object(
        document.get("continuationSummary"), "Sync promotion continuation summary"
    )
    resolved_checks = _require_resource_list(
        document.get("resolvedChecks"), "Sync promotion resolved checks"
    )
    blocking_checks = _require_resource_list(
        document.get("blockingChecks"), "Sync promotion blocking checks"
    )
    bundle_preflight = _require_object(
        document.get("bundlePreflight"), "Sync promotion bundle preflight"
    )
    lines = [
        "Sync promotion preflight",
        "Summary: resources=%s direct=%s mapped=%s missing-mappings=%s bundle-blocking=%s blocking=%s"
        % (
            int(summary.get("resourceCount") or 0),
            int(summary.get("directMatchCount") or 0),
            int(summary.get("mappedCount") or 0),
            int(summary.get("missingMappingCount") or 0),
            int(summary.get("bundleBlockingCount") or 0),
            int(summary.get("blockingCount") or 0),
        ),
        "Mappings: kind=%s schema=%s source-env=%s target-env=%s folders=%s datasource-uids=%s datasource-names=%s"
        % (
            _normalize_optional_text(mapping_summary.get("mappingKind")),
            _normalize_optional_text(mapping_summary.get("mappingSchemaVersion")),
            _normalize_optional_text(mapping_summary.get("sourceEnvironment")),
            _normalize_optional_text(mapping_summary.get("targetEnvironment")),
            int(mapping_summary.get("folderMappingCount") or 0),
            int(mapping_summary.get("datasourceUidMappingCount") or 0),
            int(mapping_summary.get("datasourceNameMappingCount") or 0),
        ),
        "Check buckets: folder-remaps=%s datasource-uid-remaps=%s datasource-name-remaps=%s resolved-remaps=%s blocking-remaps=%s direct=%s mapped=%s"
        % (
            int(check_summary.get("folderRemapCount") or 0),
            int(check_summary.get("datasourceUidRemapCount") or 0),
            int(check_summary.get("datasourceNameRemapCount") or 0),
            int(check_summary.get("resolvedCount") or 0),
            int(check_summary.get("missingTargetCount") or len(blocking_checks)),
            int(check_summary.get("directCount") or 0),
            int(check_summary.get("mappedCount") or 0),
        ),
        "Reason: promotion stays blocked until blocking checks are cleared; resolved remaps stay in the review handoff for traceability.",
        "Handoff: review-required=%s ready-for-review=%s next-stage=%s blocking=%s instruction=%s"
        % (
            bool(handoff_summary.get("reviewRequired") or False),
            bool(handoff_summary.get("readyForReview") or False),
            _normalize_optional_text(handoff_summary.get("nextStage") or ""),
            int(handoff_summary.get("blockingCount") or 0),
            _normalize_optional_text(handoff_summary.get("reviewInstruction") or ""),
        ),
        "",
        "# Controlled apply continuation",
        "staged-only=%s live-mutation-allowed=%s ready-for-continuation=%s next-stage=%s resolved=%s blocking=%s instruction=%s"
        % (
            bool(continuation_summary.get("stagedOnly") or False),
            bool(continuation_summary.get("liveMutationAllowed") or False),
            bool(continuation_summary.get("readyForContinuation") or False),
            _normalize_optional_text(continuation_summary.get("nextStage") or ""),
            int(continuation_summary.get("resolvedCount") or 0),
            int(continuation_summary.get("blockingCount") or len(blocking_checks)),
            _normalize_optional_text(continuation_summary.get("continuationInstruction") or ""),
        ),
        "",
        "# Resolved remaps",
    ]
    if not resolved_checks:
        lines.append("- none status=ok detail=No resolved remaps to review.")
    for item in resolved_checks:
        lines.append(
            "- %s identity=%s source=%s target=%s resolution=%s mapping-source=%s status=%s detail=%s"
            % (
                _normalize_optional_text(item.get("kind")) or "unknown",
                _normalize_optional_text(item.get("identity")) or "unknown",
                _normalize_optional_text(item.get("sourceValue")),
                _normalize_optional_text(item.get("targetValue")),
                _normalize_optional_text(item.get("resolution")),
                _normalize_optional_text(item.get("mappingSource")),
                _normalize_optional_text(item.get("status")),
                _normalize_optional_text(item.get("detail")),
            )
        )

    lines.extend(
        [
            "",
            "# Blocking remaps",
        ]
    )
    if not blocking_checks:
        lines.append("- none status=ok detail=No blocking remaps remain.")
    for item in blocking_checks:
        lines.append(
            "- %s identity=%s source=%s target=%s resolution=%s mapping-source=%s status=%s detail=%s"
            % (
                _normalize_optional_text(item.get("kind")) or "unknown",
                _normalize_optional_text(item.get("identity")) or "unknown",
                _normalize_optional_text(item.get("sourceValue")),
                _normalize_optional_text(item.get("targetValue")),
                _normalize_optional_text(item.get("resolution")),
                _normalize_optional_text(item.get("mappingSource")),
                _normalize_optional_text(item.get("status")),
                _normalize_optional_text(item.get("detail")),
            )
        )
    lines.extend(render_bundle_preflight_text(bundle_preflight))
    return lines


def build_sync_summary_document(raw_specs):
    """Build sync summary document implementation."""
    specs = [normalize_resource_spec(item) for item in raw_specs]
    return {
        "kind": "grafana-utils-sync-summary",
        "schemaVersion": 1,
        "summary": {
            "resourceCount": len(specs),
            "dashboardCount": len([item for item in specs if item.kind == "dashboard"]),
            "datasourceCount": len(
                [item for item in specs if item.kind == "datasource"]
            ),
            "folderCount": len([item for item in specs if item.kind == "folder"]),
            "alertCount": len([item for item in specs if item.kind == "alert"]),
        },
        "resources": [
            {
                "kind": item.kind,
                "identity": item.identity,
                "title": item.title,
                "managedFields": list(item.managed_fields),
                "bodyFieldCount": len(item.body),
                "sourcePath": item.source_path,
            }
            for item in specs
        ],
    }


def render_sync_summary_text(document):
    """Render sync summary text implementation."""
    if document.get("kind") != "grafana-utils-sync-summary":
        raise GrafanaError("Sync summary document kind is not supported.")
    summary = _require_object(document.get("summary"), "Sync summary document summary")
    return "\n".join(
        [
            "Sync summary",
            "Resources: %s total, %s dashboards, %s datasources, %s folders, %s alerts"
            % (
                int(summary.get("resourceCount") or 0),
                int(summary.get("dashboardCount") or 0),
                int(summary.get("datasourceCount") or 0),
                int(summary.get("folderCount") or 0),
                int(summary.get("alertCount") or 0),
            ),
        ]
    )


def _coerce_operation(item, index):
    """Internal helper for coerce operation."""
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




def _normalize_optional_text(value):
    """Internal helper for normalize optional text."""
    if value is None:
        return ""
    normalized = str(value).strip()
    if normalized:
        return normalized
    return ""


def _normalize_trace_id(value):
    """Internal helper for normalize optional trace id."""
    return _normalize_optional_text(value)


def _fnv1a64_hex(text):
    """Internal helper for 64-bit FNV-1a hex digest."""
    if text is None:
        text = ""
    digest = 0xCBF29CE484222325
    for raw in str(text).encode("utf-8"):
        digest ^= raw
        digest = (digest * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{digest:016x}"


def _derive_trace_id(document):
    """Internal helper for derive trace id from document shape."""
    return "sync-trace-" + _fnv1a64_hex(json.dumps(document, separators=(",", "")))


def _attach_trace_id(document, trace_id=None):
    """Internal helper for attach trace id."""
    resolved = _normalize_optional_text(trace_id)
    if not resolved:
        resolved = _derive_trace_id(document)
    payload = dict(document)
    payload["traceId"] = resolved
    return payload


def _require_trace_id(document, label):
    """Internal helper for require trace id."""
    trace_id = _normalize_trace_id(document.get("traceId"))
    if not trace_id:
        raise GrafanaError(f"{label} is missing traceId.")
    return trace_id


def _deterministic_stage_marker(trace_id, stage):
    """Internal helper for deterministic marker text."""
    return f"staged:{_normalize_optional_text(trace_id)}:{_normalize_optional_text(stage)}"


def _attach_lineage(document, stage, step_index, parent_trace_id=None):
    """Internal helper for attach staged lineage."""
    payload = dict(document)
    payload["stage"] = str(stage)
    payload["stepIndex"] = int(step_index)
    parent_trace_id = _normalize_optional_text(parent_trace_id)
    if parent_trace_id:
        payload["parentTraceId"] = parent_trace_id
    else:
        payload.pop("parentTraceId", None)
    return payload


def _has_lineage_metadata(document):
    """Internal helper for lineage metadata check."""
    return any(
        key in document
        for key in ("stage", "stepIndex", "parentTraceId")
    )


def _require_optional_stage(document, label, expected_stage, expected_step_index, expected_parent_trace_id=None):
    """Internal helper for optional lineage validation."""
    if not isinstance(document, dict):
        raise GrafanaError(f"{label} must be a JSON object.")
    if not _has_lineage_metadata(document):
        return
    stage = _normalize_optional_text(document.get("stage"))
    if not stage:
        raise GrafanaError(f"{label} is missing lineage stage metadata.")
    if stage != expected_stage:
        raise GrafanaError(
            f"{label} has unexpected lineage stage {stage!r}; expected {expected_stage!r}."
        )
    step_index = document.get("stepIndex")
    if not _is_json_int(step_index):
        raise GrafanaError(f"{label} is missing lineage stepIndex metadata.")
    if step_index != expected_step_index:
        raise GrafanaError(
            f"{label} has unexpected lineage stepIndex {step_index}; expected {expected_step_index}."
        )
    actual_parent_trace_id = _normalize_optional_text(document.get("parentTraceId"))
    expected_parent_trace_id = _normalize_optional_text(expected_parent_trace_id)
    if actual_parent_trace_id and not expected_parent_trace_id:
        raise GrafanaError(
            f"{label} has unexpected lineage parentTraceId {actual_parent_trace_id!r}; expected no parent trace."
        )
    if expected_parent_trace_id and (not actual_parent_trace_id or actual_parent_trace_id != expected_parent_trace_id):
        raise GrafanaError(
            f"{label} has unexpected lineage parentTraceId {actual_parent_trace_id!r}; expected {expected_parent_trace_id!r}."
        )


def _require_matching_optional_trace_id(document, label, expected_trace_id):
    """Internal helper for optional lineage-aware trace consistency checks."""
    if not isinstance(document, dict):
        raise GrafanaError(f"{label} must be a JSON object.")
    if _has_lineage_metadata(document):
        if not _normalize_optional_text(document.get("stage")):
            raise GrafanaError(f"{label} is missing lineage stage metadata.")
        step_index = document.get("stepIndex")
        if not _is_json_int(step_index):
            raise GrafanaError(f"{label} is missing lineage stepIndex metadata.")
    trace_id = _normalize_trace_id(document.get("traceId"))
    if not trace_id:
        if _has_lineage_metadata(document):
            raise GrafanaError(
                f"{label} is missing traceId for lineage-aware staged validation."
            )
        return
    if trace_id != expected_trace_id:
        raise GrafanaError(
            f"{label} traceId {trace_id!r} does not match sync plan traceId {expected_trace_id!r}."
        )
    parent_trace_id = _normalize_optional_text(document.get("parentTraceId"))
    if parent_trace_id and parent_trace_id != expected_trace_id:
        raise GrafanaError(
            f"{label} parentTraceId {parent_trace_id!r} does not match sync plan traceId {expected_trace_id!r}."
        )


def _validate_apply_preflight(document):
    """Internal helper for preflight validation used by sync apply."""
    if not isinstance(document, dict):
        raise GrafanaError("Sync preflight document must be a JSON object.")
    if document.get("kind") != "grafana-utils-sync-preflight":
        if document.get("kind") == "grafana-utils-sync-bundle-preflight":
            raise GrafanaError(
                "Sync bundle preflight document is not supported via --preflight-file; use --bundle-preflight-file."
            )
        raise GrafanaError("Sync preflight document kind is not supported.")
    summary = document.get("summary")
    if not isinstance(summary, dict):
        raise GrafanaError("Sync preflight document is missing summary.")
    check_count = summary.get("checkCount")
    if not _is_json_int(check_count):
        raise GrafanaError("Sync preflight summary is missing checkCount.")
    ok_count = summary.get("okCount")
    if not _is_json_int(ok_count):
        raise GrafanaError("Sync preflight summary is missing okCount.")
    blocking_count = summary.get("blockingCount")
    if not _is_json_int(blocking_count):
        raise GrafanaError("Sync preflight summary is missing blockingCount.")
    if blocking_count > 0:
        raise GrafanaError(
            f"Refusing local sync apply intent because preflight reports {blocking_count} blocking checks."
        )
    return {
        "kind": document.get("kind"),
        "checkCount": check_count,
        "okCount": ok_count,
        "blockingCount": blocking_count,
    }


def _validate_apply_bundle_preflight(document):
    """Internal helper for bundle preflight validation used by sync apply."""
    if not isinstance(document, dict):
        raise GrafanaError("Sync bundle preflight document must be a JSON object.")
    if document.get("kind") != "grafana-utils-sync-bundle-preflight":
        raise GrafanaError("Sync bundle preflight document kind is not supported.")
    summary = document.get("summary")
    if not isinstance(summary, dict):
        raise GrafanaError("Sync bundle preflight document is missing summary.")
    resource_count = summary.get("resourceCount")
    if not _is_json_int(resource_count):
        raise GrafanaError("Sync bundle preflight summary is missing resourceCount.")
    sync_blocking_count = summary.get("syncBlockingCount")
    if not _is_json_int(sync_blocking_count):
        raise GrafanaError("Sync bundle preflight summary is missing syncBlockingCount.")
    provider_blocking_count = summary.get("providerBlockingCount")
    if not _is_json_int(provider_blocking_count):
        raise GrafanaError("Sync bundle preflight summary is missing providerBlockingCount.")
    blocking_count = sync_blocking_count + provider_blocking_count
    if blocking_count > 0:
        raise GrafanaError(
            f"Refusing local sync apply intent because bundle preflight reports {blocking_count} blocking checks."
        )
    return {
        "kind": document.get("kind"),
        "resourceCount": resource_count,
        "checkCount": resource_count,
        "okCount": (resource_count - blocking_count) if resource_count - blocking_count > 0 else 0,
        "blockingCount": blocking_count,
        "syncBlockingCount": sync_blocking_count,
        "providerBlockingCount": provider_blocking_count,
    }

def load_plan_document(path):
    """Load one persisted sync plan document back into a SyncPlan."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1286, 677
    #   Downstream callees: 366, 396, 457

    return _coerce_plan_document(
        _require_object(load_json_document(path), "Sync plan document")
    )


def emit_document(document, output_file=None):
    """Print a JSON document and optionally persist it to disk."""
    if output_file:
        write_json_document(output_file, document)
    print(json.dumps(document, indent=2, sort_keys=False))


def emit_document_with_output(document, lines, output, output_file=None):
    """Emit one document as text or json while keeping optional file writes."""
    if output == "json":
        emit_document(document, output_file=output_file)
        return
    if output_file:
        write_json_document(output_file, document)
    for line in lines:
        print(line)


def _normalize_string(value, default=""):
    """Internal helper for normalize string."""
    if value is None:
        return default
    text = str(value).strip()
    if text:
        return text
    return default


def _copy_mapping(value, label):
    """Internal helper for copy mapping."""
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
    alert_rules = client.request_json("/api/v1/provisioning/alert-rules")
    if not isinstance(alert_rules, list):
        raise GrafanaError("Unexpected alert-rule list response from Grafana.")
    for rule in alert_rules:
        if not isinstance(rule, dict):
            continue
        uid = _normalize_string(rule.get("uid"))
        if not uid:
            continue
        body = build_rule_import_payload(rule)
        body["uid"] = _normalize_string(body.get("uid"), uid) or uid
        specs.append(
            {
                "kind": "alert",
                "uid": uid,
                "title": _normalize_string(body.get("title"), uid),
                "body": body,
                "managedFields": [
                    field
                    for field in (
                        "condition",
                        "labels",
                        "annotations",
                        "contactPoints",
                        "for",
                        "noDataState",
                        "execErrState",
                    )
                    if field in body
                ]
                or ["condition"],
            }
        )
    return specs


def run_plan(args):
    """Run plan implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 366, 387, 403, 494, 520

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
    document = _attach_lineage(
        _attach_trace_id(
            plan_to_document(plan),
            trace_id=getattr(args, "trace_id", None),
        ),
        "plan",
        1,
        None,
    )
    emit_document_with_output(
        document,
        render_sync_plan_text(document),
        getattr(args, "output", "text"),
        output_file=getattr(args, "plan_file", None),
    )
    return 0


def run_summary(args):
    """Run summary implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 366, 403, 410, 437, 494

    desired_specs = _require_resource_list(
        load_json_document(args.desired_file),
        "Desired sync input",
    )
    document = build_sync_summary_document(desired_specs)
    if args.output == "json":
        emit_document(document)
        return 0
    print(render_sync_summary_text(document))
    return 0


def run_review(args):
    """Run review implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 475, 494

    plan_document = _require_object(
        load_json_document(args.plan_file),
        "Sync plan document",
    )
    if plan_document.get("kind") != "grafana-utils-sync-plan":
        raise GrafanaError("Sync plan document kind is not supported.")
    trace_id = _require_trace_id(plan_document, "Sync plan document")
    _require_optional_stage(plan_document, "Sync plan document", "plan", 1, None)
    plan = _coerce_plan_document(plan_document)
    reviewed_plan = mark_plan_reviewed(
        plan,
        review_token=getattr(args, "review_token", DEFAULT_REVIEW_TOKEN),
    )
    document = _attach_lineage(
        _attach_review_audit(
            _attach_trace_id(
                plan_to_document(reviewed_plan),
                trace_id,
            ),
            trace_id,
            getattr(args, "reviewed_by", None),
            getattr(args, "reviewed_at", None),
            getattr(args, "review_note", None),
        ),
        "review",
        2,
        trace_id,
    )
    emit_document_with_output(
        document,
        render_sync_plan_text(document),
        getattr(args, "output", "text"),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def _load_optional_object_file(path, label):
    """Internal helper for load optional object file."""
    if not path:
        return {}
    return _require_object(load_json_document(path), label)


def _merge_availability(base, extra):
    """Internal helper for merge availability."""
    merged = dict(base or {})
    for key, value in (extra or {}).items():
        if key in ("datasourceUids", "datasourceNames", "pluginIds", "contactPoints"):
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
        "datasourceNames": [],
        "pluginIds": [],
        "contactPoints": [],
    }
    for datasource in client.list_datasources():
        if not isinstance(datasource, dict):
            continue
        uid = _normalize_string(datasource.get("uid"))
        name = _normalize_string(datasource.get("name"))
        if uid:
            availability["datasourceUids"].append(uid)
        if name:
            availability["datasourceNames"].append(name)

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
        if uid:
            availability["contactPoints"].append(uid)
    return availability


def _emit_text_or_json(document, lines, output):
    """Internal helper for emit text or json."""
    if output == "json":
        print(json.dumps(document, indent=2, sort_keys=False))
        return
    for line in lines:
        print(line)


def _load_optional_array_file(path, label):
    """Internal helper for load optional array file."""
    if not path:
        return []
    document = load_json_document(path)
    return _require_resource_list(document, label)


def _discover_json_files(root, ignored_names):
    """Internal helper for discover json files."""
    files = []
    for path in sorted(Path(root).rglob("*.json")):
        if path.name in ignored_names:
            continue
        files.append(path)
    return files


def _dashboard_body_from_export(document):
    """Internal helper for dashboard body from export."""
    if isinstance(document, dict) and isinstance(document.get("dashboard"), dict):
        body = dict(document.get("dashboard") or {})
    else:
        body = _copy_mapping(document, "Dashboard export document")
    body.pop("id", None)
    return body


def _normalize_dashboard_bundle_item(document, source_path):
    """Internal helper for normalize dashboard bundle item."""
    body = _dashboard_body_from_export(document)
    uid = _normalize_string(body.get("uid"))
    title = _normalize_string(body.get("title"), uid)
    if not uid:
        raise GrafanaError(
            "Dashboard export document is missing dashboard.uid: %s" % source_path
        )
    return {
        "kind": "dashboard",
        "uid": uid,
        "title": title or uid,
        "body": body,
        "sourcePath": source_path,
    }


def _normalize_folder_bundle_item(record):
    """Internal helper for normalize folder bundle item."""
    record = _copy_mapping(record, "Folder inventory record")
    uid = _normalize_string(record.get("uid"))
    title = _normalize_string(record.get("title"), uid)
    body = {"title": title or uid}
    parent_uid = _normalize_string(record.get("parentUid"))
    if parent_uid:
        body["parentUid"] = parent_uid
    path = _normalize_string(record.get("path"))
    if path:
        body["path"] = path
    return {
        "kind": "folder",
        "uid": uid,
        "title": title or uid,
        "body": body,
        "sourcePath": _normalize_string(record.get("sourcePath")),
    }


def _normalize_datasource_bundle_item(record):
    """Internal helper for normalize datasource bundle item."""
    record = _copy_mapping(record, "Datasource inventory record")
    uid = _normalize_string(record.get("uid"))
    name = _normalize_string(record.get("name"), uid)
    if not (uid or name):
        raise GrafanaError("Datasource inventory record requires uid or name.")
    body = {
        "uid": uid,
        "name": name or uid,
        "type": _normalize_string(record.get("type")),
        "access": _normalize_string(record.get("access")),
        "url": _normalize_string(record.get("url")),
        "isDefault": bool(record.get("isDefault")),
    }
    return {
        "kind": "datasource",
        "uid": uid,
        "name": name or uid,
        "title": name or uid,
        "body": body,
        "sourcePath": _normalize_string(record.get("sourcePath")),
    }


def _classify_alert_export_path(relative_path):
    """Internal helper for classify alert export path."""
    parts = list(Path(relative_path).parts)
    if not parts:
        return None
    root = parts[0]
    mapping = {
        "rules": "rules",
        "contact-points": "contactPoints",
        "mute-timings": "muteTimings",
        "policies": "policies",
        "templates": "templates",
    }
    return mapping.get(root)


def _load_dashboard_bundle_sections(export_dir):
    """Internal helper for load dashboard bundle sections."""
    # Call graph: see callers/callees.
    #   Upstream callers: 954
    #   Downstream callees: 366, 396, 768, 776, 796, 812, 833

    root = Path(export_dir)
    dashboards = [
        _normalize_dashboard_bundle_item(
            load_json_document(str(path)),
            path.relative_to(root).as_posix(),
        )
        for path in _discover_json_files(
            root,
            ("index.json", "export-metadata.json", "folders.json", "datasources.json"),
        )
    ]
    folders = [
        _normalize_folder_bundle_item(item)
        for item in _load_optional_array_file(
            root / "folders.json", "Dashboard folder inventory"
        )
    ]
    datasources = [
        _normalize_datasource_bundle_item(item)
        for item in _load_optional_array_file(
            root / "datasources.json",
            "Dashboard datasource inventory",
        )
    ]
    metadata = {}
    export_metadata_path = root / "export-metadata.json"
    if export_metadata_path.is_file():
        metadata["dashboardExport"] = _require_object(
            load_json_document(str(export_metadata_path)),
            "Dashboard export metadata",
        )
    metadata["dashboardExportDir"] = str(root)
    return dashboards, datasources, folders, metadata


def _load_alerting_bundle_section(export_dir):
    """Internal helper for load alerting bundle section."""
    # Call graph: see callers/callees.
    #   Upstream callers: 954
    #   Downstream callees: 366, 396, 776, 858

    root = Path(export_dir)
    alerting = {
        "summary": {
            "ruleCount": 0,
            "contactPointCount": 0,
            "muteTimingCount": 0,
            "policyCount": 0,
            "templateCount": 0,
        },
        "rules": [],
        "contactPoints": [],
        "muteTimings": [],
        "policies": [],
        "templates": [],
    }
    for path in _discover_json_files(root, ("index.json", "export-metadata.json")):
        relative_path = path.relative_to(root).as_posix()
        section = _classify_alert_export_path(relative_path)
        if not section:
            continue
        alerting[section].append(
            {
                "sourcePath": relative_path,
                "document": load_json_document(str(path)),
            }
        )
    alerting["summary"] = {
        "ruleCount": len(alerting["rules"]),
        "contactPointCount": len(alerting["contactPoints"]),
        "muteTimingCount": len(alerting["muteTimings"]),
        "policyCount": len(alerting["policies"]),
        "templateCount": len(alerting["templates"]),
    }
    export_metadata_path = root / "export-metadata.json"
    if export_metadata_path.is_file():
        alerting["exportMetadata"] = _require_object(
            load_json_document(str(export_metadata_path)),
            "Alert export metadata",
        )
    alerting["exportDir"] = str(root)
    return alerting


def run_bundle(args):
    """Run bundle implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 377, 691, 759, 768, 833, 874, 909

    if not any(
        (
            getattr(args, "dashboard_export_dir", None),
            getattr(args, "alert_export_dir", None),
            getattr(args, "datasource_export_file", None),
            getattr(args, "metadata_file", None),
        )
    ):
        raise GrafanaError(
            "Sync bundle requires at least one export input such as --dashboard-export-dir, --alert-export-dir, --datasource-export-file, or --metadata-file."
        )
    dashboards = []
    datasources = []
    folders = []
    metadata = {}
    if getattr(args, "dashboard_export_dir", None):
        (
            dashboards,
            dashboard_datasources,
            folders,
            dashboard_metadata,
        ) = _load_dashboard_bundle_sections(args.dashboard_export_dir)
        datasources.extend(dashboard_datasources)
        metadata.update(dashboard_metadata)
    if getattr(args, "datasource_export_file", None):
        datasources = [
            _normalize_datasource_bundle_item(item)
            for item in _load_optional_array_file(
                args.datasource_export_file,
                "Datasource export inventory",
            )
        ]
        metadata["datasourceExportFile"] = str(args.datasource_export_file)
    alerting = {}
    if getattr(args, "alert_export_dir", None):
        alerting = _load_alerting_bundle_section(args.alert_export_dir)
    metadata.update(
        _load_optional_object_file(
            getattr(args, "metadata_file", None),
            "Sync bundle metadata input",
        )
    )
    document = build_sync_source_bundle_document(
        dashboards=dashboards,
        datasources=datasources,
        folders=folders,
        alerting=alerting,
        metadata=metadata,
    )
    if getattr(args, "output_file", None):
        write_json_document(args.output_file, document)
    _emit_text_or_json(
        document,
        render_sync_source_bundle_text(document),
        getattr(args, "output", "text"),
    )
    return 0


def run_preflight(args):
    """Run preflight implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 366, 387, 403, 691, 698, 716, 759

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
    """Run assess alerts implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 366, 403, 759

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
    """Run bundle preflight implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 366, 387, 396, 691, 698, 716, 759

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
    """Internal helper for serialize apply intent."""
    payload = {
        "kind": intent.get("kind"),
        "schemaVersion": intent.get("schemaVersion"),
        "mode": intent.get("mode"),
        "reviewRequired": bool(intent.get("reviewRequired")),
        "reviewed": bool(intent.get("reviewed")),
        "allowPrune": bool(intent.get("allowPrune")),
        "approved": bool(intent.get("approved")),
        "summary": intent.get("summary"),
        "alertAssessment": intent.get("alertAssessment"),
        "traceId": intent.get("traceId"),
        "stage": intent.get("stage"),
        "stepIndex": intent.get("stepIndex"),
        "parentTraceId": intent.get("parentTraceId"),
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
    for key in (
        "preflightSummary",
        "bundlePreflightSummary",
        "reviewedBy",
        "reviewedAt",
        "reviewNote",
        "appliedBy",
        "appliedAt",
        "approvalReason",
        "applyNote",
    ):
        value = intent.get(key)
        if value is not None:
            payload[key] = value
    return payload


def _attach_review_audit(document, trace_id, reviewed_by, reviewed_at, review_note):
    """Internal helper for attach review audit fields."""
    payload = dict(document)
    reviewed_by = _normalize_optional_text(reviewed_by)
    if reviewed_by:
        payload["reviewedBy"] = reviewed_by
    payload["reviewedAt"] = _normalize_optional_text(reviewed_at) or _deterministic_stage_marker(
        trace_id,
        "reviewed",
    )
    review_note = _normalize_optional_text(review_note)
    if review_note:
        payload["reviewNote"] = review_note
    return payload


def _attach_apply_audit(
    document,
    trace_id,
    applied_by,
    applied_at,
    approval_reason,
    apply_note,
):
    """Internal helper for attach apply audit fields."""
    payload = dict(document)
    applied_by = _normalize_optional_text(applied_by)
    if applied_by:
        payload["appliedBy"] = applied_by
    payload["appliedAt"] = _normalize_optional_text(applied_at) or _deterministic_stage_marker(
        trace_id,
        "applied",
    )
    approval_reason = _normalize_optional_text(approval_reason)
    if approval_reason:
        payload["approvalReason"] = approval_reason
    apply_note = _normalize_optional_text(apply_note)
    if apply_note:
        payload["applyNote"] = apply_note
    return payload


def _attach_preflight_summary(document, summary):
    """Internal helper for attach preflight summary metadata."""
    payload = dict(document)
    if summary is not None:
        payload["preflightSummary"] = summary
    return payload


def _attach_bundle_preflight_summary(document, summary):
    """Internal helper for attach bundle preflight summary metadata."""
    payload = dict(document)
    if summary is not None:
        payload["bundlePreflightSummary"] = summary
    return payload


def _coerce_plan_document(document):
    """Internal helper for sync plan document to SyncPlan object."""
    if not isinstance(document, dict):
        raise GrafanaError("Sync plan document must be a JSON object.")
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


def _resolve_datasource_target(client, operation):
    """Internal helper for resolve datasource target."""
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
    """Internal helper for apply folder operation."""
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
    """Internal helper for apply dashboard operation."""
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
    """Internal helper for apply datasource operation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1253
    #   Downstream callees: 1109, 501, 511

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


def _apply_alert_operation(client, operation):
    """Internal helper for apply alert operation."""
    uid = _normalize_string(operation.identity)
    if not uid:
        raise GrafanaError("Alert sync operations require a stable uid identity.")
    if operation.action == "would-delete":
        return client.request_json(
            "/api/v1/provisioning/alert-rules/%s" % parse.quote(uid, safe=""),
            method="DELETE",
        )
    body = _copy_mapping(operation.desired, "Alert desired body")
    body["uid"] = _normalize_string(body.get("uid"), uid) or uid
    if body["uid"] != uid:
        raise GrafanaError(
            "Alert sync body uid %s does not match operation identity %s."
            % (body["uid"], uid)
        )
    try:
        payload = build_rule_import_payload(body)
    except AlertGrafanaError as exc:
        raise GrafanaError(str(exc))
    if operation.action == "would-create":
        return client.request_json(
            "/api/v1/provisioning/alert-rules",
            method="POST",
            payload=payload,
        )
    if operation.action == "would-update":
        return client.request_json(
            "/api/v1/provisioning/alert-rules/%s" % parse.quote(uid, safe=""),
            method="PUT",
            payload=payload,
        )
    raise GrafanaError("Unsupported alert sync action %s." % operation.action)


def execute_live_apply(client, operations, allow_folder_delete=False):
    """Apply one gated sync intent to Grafana for supported resource kinds."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1286
    #   Downstream callees: 1125, 1161, 1182, 1217

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
            response = _apply_alert_operation(client, operation)
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
    """Run apply implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1322
    #   Downstream callees: 1086, 1253, 387, 475, 494

    plan_document = _require_object(
        load_json_document(args.plan_file),
        "Sync plan document",
    )
    if plan_document.get("kind") != "grafana-utils-sync-plan":
        raise GrafanaError("Sync plan document kind is not supported.")
    trace_id = _require_trace_id(plan_document, "Sync plan document")
    _require_optional_stage(plan_document, "Sync plan document", "review", 2, trace_id)
    plan = _coerce_plan_document(plan_document)
    preflight_summary = None
    if getattr(args, "preflight_file", None):
        preflight = _require_object(
            load_json_document(getattr(args, "preflight_file")),
            "Sync preflight input",
        )
        _require_matching_optional_trace_id(
            preflight,
            "Sync preflight document",
            trace_id,
        )
        preflight_summary = _validate_apply_preflight(preflight)

    bundle_preflight_summary = None
    if getattr(args, "bundle_preflight_file", None):
        bundle_preflight = _require_object(
            load_json_document(getattr(args, "bundle_preflight_file")),
            "Sync bundle preflight input",
        )
        _require_matching_optional_trace_id(
            bundle_preflight,
            "Sync bundle preflight document",
            trace_id,
        )
        bundle_preflight_summary = _validate_apply_bundle_preflight(
            bundle_preflight
        )

    intent = build_apply_intent(plan, approve=bool(getattr(args, "approve", False)))
    intent = _attach_preflight_summary(intent, preflight_summary)
    intent = _attach_bundle_preflight_summary(intent, bundle_preflight_summary)
    intent = _attach_apply_audit(
        intent,
        trace_id,
        getattr(args, "applied_by", None),
        getattr(args, "applied_at", None),
        getattr(args, "approval_reason", None),
        getattr(args, "apply_note", None),
    )
    intent = _attach_trace_id(intent, trace_id)
    intent = _attach_lineage(
        intent,
        "apply",
        3,
        trace_id,
    )
    if bool(getattr(args, "execute_live", False)):
        live_result = execute_live_apply(
            build_client(args),
            intent.get("operations") or (),
            allow_folder_delete=bool(getattr(args, "allow_folder_delete", False)),
        )
        emit_document_with_output(
            live_result,
            [
                "Sync live apply",
                "Applied: %s" % int(live_result.get("appliedCount") or 0),
            ],
            getattr(args, "output", "text"),
            output_file=getattr(args, "output_file", None),
        )
        return 0
    emit_document_with_output(
        _serialize_apply_intent(intent),
        render_sync_apply_intent_text(intent),
        getattr(args, "output", "text"),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def _dispatch_compat_command(
    args,
    command_name,
    unsupported_description=None,
):
    """Compatibility-style dispatch into existing sync commands."""
    if unsupported_description:
        raise GrafanaError(unsupported_description)
    forwarded = list(getattr(args, "forwarded_argv", ()) or ())
    return main([command_name] + forwarded)


def run_scan(args):
    """Run scan command compatibility implementation."""
    return _dispatch_compat_command(args, "summary")


def run_test(args):
    """Run test command compatibility implementation."""
    return _dispatch_compat_command(args, "preflight")


def run_preview(args):
    """Run preview command compatibility implementation."""
    return _dispatch_compat_command(args, "plan")


def run_ci_summary(args):
    """Run CI summary command compatibility implementation."""
    return _dispatch_compat_command(args, "summary")


def run_ci_plan(args):
    """Run CI plan command compatibility implementation."""
    return _dispatch_compat_command(args, "plan")


def run_ci_mark_reviewed(args):
    """Run CI mark-reviewed command compatibility implementation."""
    return _dispatch_compat_command(args, "review")


def run_ci_input_test(args):
    """Run CI input-test command compatibility implementation."""
    return _dispatch_compat_command(args, "preflight")


def run_ci_alert_readiness(args):
    """Run CI alert-readiness command compatibility implementation."""
    return _dispatch_compat_command(args, "assess-alerts")


def run_ci_package_test(args):
    """Run CI package-test command compatibility implementation."""
    return _dispatch_compat_command(args, "bundle-preflight")


def run_ci_audit(args):
    """Run CI audit command compatibility implementation."""
    if not bool(getattr(args, "managed_file", None)) and not bool(
        getattr(args, "lock_file", None)
    ):
        raise GrafanaError("Sync audit requires --managed-file, --lock-file, or both.")

    baseline_lock = None
    if getattr(args, "lock_file", None):
        baseline_lock = _require_object(load_json_document(args.lock_file), "Sync lock input")

    if bool(getattr(args, "fetch_live", False)):
        live_specs = fetch_live_resource_specs(
            build_client(args),
            page_size=getattr(args, "page_size", 500),
        )
    else:
        if not getattr(args, "live_file", None):
            raise GrafanaError("Sync audit requires --live-file unless --fetch-live is used.")
        live_specs = _require_resource_list(
            load_json_document(args.live_file),
            "Sync live input",
        )

    if getattr(args, "managed_file", None):
        managed_specs = _require_resource_list(
            load_json_document(args.managed_file),
            "Sync managed input",
        )
        current_lock = build_sync_lock_document(managed_specs, live_specs)
    else:
        if baseline_lock is None:
            raise GrafanaError("Sync audit requires --managed-file or --lock-file.")
        current_lock = build_sync_lock_document_from_lock(baseline_lock, live_specs)

    audit_document = build_sync_audit_document(
        current_lock,
        baseline_lock,
    )
    drift_count = int((audit_document.get("summary") or {}).get("driftCount") or 0)
    write_lock_path = getattr(args, "write_lock", None)
    if write_lock_path and not (args.fail_on_drift and drift_count > 0):
        write_json_document(write_lock_path, current_lock)

    if args.fail_on_drift and drift_count > 0:
        raise GrafanaError("Sync audit detected %s drifted resource(s)." % drift_count)

    emit_document_with_output(
        audit_document,
        render_sync_audit_text(audit_document),
        getattr(args, "output", "text"),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def run_ci_promote_test(args):
    """Run CI promote-test command compatibility implementation."""
    source_bundle = _require_object(
        load_json_document(args.source_bundle),
        "Source bundle input",
    )
    target_inventory = _require_object(
        load_json_document(args.target_inventory),
        "Sync target inventory input",
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
    document = build_ci_promotion_test_document(
        source_bundle=source_bundle,
        target_inventory=target_inventory,
        availability=availability,
        mapping_file_path=getattr(args, "mapping_file", None),
    )
    emit_document_with_output(
        document,
        render_sync_promotion_preflight_text(document),
        getattr(args, "output", "text"),
        output_file=getattr(args, "output_file", None),
    )
    return 0


def run_ci(args):
    """Run CI namespace dispatch."""
    if args.ci_command == "summary":
        return run_ci_summary(args)
    if args.ci_command == "plan":
        return run_ci_plan(args)
    if args.ci_command == "mark-reviewed":
        return run_ci_mark_reviewed(args)
    if args.ci_command == "input-test":
        return run_ci_input_test(args)
    if args.ci_command == "audit":
        return run_ci_audit(args)
    if args.ci_command == "alert-readiness":
        return run_ci_alert_readiness(args)
    if args.ci_command == "package-test":
        return run_ci_package_test(args)
    if args.ci_command == "promote-test":
        return run_ci_promote_test(args)
    raise GrafanaError("Unsupported workspace ci command: %s" % args.ci_command)


def parse_args(argv=None):
    """Parse args implementation."""
    parser = build_parser()
    argv = list(sys.argv[1:] if argv is None else argv)
    parsed, unknown = parser.parse_known_args(argv)
    if parsed.command in ("scan", "test", "preview"):
        parsed.forwarded_argv = unknown
        return parsed
    if parsed.command == "ci":
        parsed.forwarded_argv = unknown
        return parsed
    if unknown:
        parser.parse_args(argv)
    return parsed


def main(argv=None):
    """Main implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1015, 1039, 1054, 1286, 1317, 633, 663, 677, 954

    args = parse_args(argv)
    try:
        if args.command == "scan":
            return run_scan(args)
        if args.command == "test":
            return run_test(args)
        if args.command == "preview":
            return run_preview(args)
        if args.command == "summary":
            return run_summary(args)
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
        if args.command in ("bundle", "package"):
            return run_bundle(args)
        if args.command == "ci":
            return run_ci(args)
        return run_apply(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


__all__ = [
    "build_parser",
    "build_sync_summary_document",
    "emit_document",
    "load_json_document",
    "load_plan_document",
    "main",
    "parse_args",
    "run_ci",
    "run_ci_alert_readiness",
    "run_ci_audit",
    "run_ci_input_test",
    "run_ci_mark_reviewed",
    "run_ci_promote_test",
    "run_ci_package_test",
    "run_ci_plan",
    "run_ci_summary",
    "run_preview",
    "run_scan",
    "run_test",
    "run_summary",
    "run_assess_alerts",
    "run_apply",
    "run_bundle",
    "run_bundle_preflight",
    "run_plan",
    "run_preflight",
    "run_review",
    "render_sync_summary_text",
    "write_json_document",
]
