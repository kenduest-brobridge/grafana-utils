#!/usr/bin/env python3
"""Unified Python entrypoint for dashboard, alert, access, datasource, and sync CLIs.

Purpose:
- Central CLI bootstrap for all Python commands so operators can use one binary
  (`grafana-util`) with one canonical namespaced command shape.

Architecture:
- Keep one entry process (`grafana-util`) that only does command routing.
- Accept only namespaced `grafana-util <module> <command>` forms at the unified
  entrypoint.
- Delegate real argument parsing and execution to each domain CLI module so each
  domain can evolve independently.

Usage notes:
- Use namespaced forms such as `grafana-util dashboard export`.
- No Grafana API logic is implemented here; this file only maps entrypoints.

Caveats:
- Do not add domain workflows in this module; keep behavior in `dashboard_cli`,
  `alert_cli`, `access_cli`, `datasource_cli`, and `sync_cli` to avoid hidden
  coupling.
"""

import argparse
import sys
from typing import Optional

from . import (
    access_cli,
    alert_cli,
    change_cli,
    dashboard_cli,
    datasource_cli,
    overview_cli,
    profile_cli,
    resource_cli,
    snapshot_cli,
    status_cli,
    sync_cli,
)

DASHBOARD_COMMAND_HELP = {
    "fetch-live": "Fetch one live dashboard into a local draft file.",
    "clone-live": "Clone one live dashboard into a local draft file.",
    "browse": "Browse one local dashboard tree in a preview server.",
    "edit-live": "Edit one live dashboard in an external editor and optionally apply it live.",
    "patch-file": "Patch one local dashboard JSON file in place or to a new path.",
    "serve": "Run one local dashboard preview server.",
    "review": "Review one local dashboard JSON file without touching Grafana.",
    "publish": "Publish one local dashboard JSON file through the import pipeline.",
    "raw-to-prompt": "Convert raw dashboard JSON into prompt-lane artifacts.",
    "analyze": "Analyze dashboards from live Grafana or a local export tree.",
    "validate-export": "Validate one dashboard export tree without mutating Grafana.",
    "history": "List, export, or restore dashboard revision history.",
    "export": "Export dashboards into raw/ and prompt/ variants.",
    "list": "List live dashboard summaries from Grafana.",
    "import": "Import dashboards from exported raw JSON files.",
    "delete": "Delete live dashboards by UID or folder path.",
    "diff": (
        "Compare exported raw dashboards with the current Grafana state; "
        "inspect provisioning trees separately with dashboard inspect-export --input-format provisioning."
    ),
    "inspect-export": "Analyze a raw dashboard export directory offline.",
    "inspect-live": "Analyze live Grafana dashboards without writing a persistent export.",
    "list-vars": "List dashboard templating variables from live Grafana.",
    "governance-gate": "Evaluate dashboard governance policy against inspect artifacts.",
    "topology": "Build a deterministic dashboard topology graph from governance artifacts.",
    "impact": "Summarize which dashboards and alerts would be affected by one datasource.",
    "screenshot": "Capture one Grafana dashboard or panel through a browser backend.",
}
UNIFIED_DASHBOARD_COMMAND_MAP = {
    "fetch-live": "fetch-live",
    "clone-live": "clone-live",
    "browse": "browse",
    "edit-live": "edit-live",
    "patch-file": "patch-file",
    "serve": "serve",
    "review": "review",
    "publish": "publish",
    "raw-to-prompt": "raw-to-prompt",
    "analyze": "analyze",
    "validate-export": "validate-export",
    "history": "history",
    "export": "export-dashboard",
    "list": "list-dashboard",
    "import": "import-dashboard",
    "delete": "delete-dashboard",
    "diff": "diff",
    "inspect-export": "inspect-export",
    "inspect-live": "inspect-live",
    "list-vars": "list-vars",
    "inspect-vars": "list-vars",
    "governance-gate": "governance-gate",
    "topology": "topology",
    "impact": "impact",
    "graph": "topology",
    "screenshot": "screenshot",
}
DATASOURCE_COMMAND_HELP = {
    "types": "Show the built-in supported datasource type catalog.",
    "list": "List live Grafana datasource inventory.",
    "add": "Create one live Grafana datasource through the Grafana API.",
    "modify": "Modify one live Grafana datasource through the Grafana API.",
    "delete": "Delete one live Grafana datasource through the Grafana API.",
    "export": "Export live Grafana datasource inventory as normalized JSON files.",
    "import": "Import datasource inventory JSON through the Grafana API.",
    "diff": "Compare exported datasource inventory with the current Grafana state.",
}
SYNC_COMMAND_HELP = {
    "scan": "Scan staged workspace artifacts and summarize local resource state.",
    "test": "Test whether staged workspace artifacts are structurally safe to continue.",
    "preview": "Preview what would change from staged workspace inputs.",
    "summary": "Summarize local desired sync resources from JSON.",
    "plan": "Build one reviewable sync plan from desired/live JSON files.",
    "review": "Mark one sync plan document as reviewed.",
    "preflight": "Build one staged sync preflight document from local JSON inputs.",
    "assess-alerts": "Assess alert sync specs for candidate, plan-only, and blocked states.",
    "bundle-preflight": "Build one staged bundle-level preflight document from local JSON inputs.",
    "apply": "Build a gated non-live apply intent from a reviewed plan.",
    "package": "Package exported dashboards, alerting resources, datasource inventory, and metadata into one source bundle.",
    "ci": "Run CI-oriented workspace workflows and lower-level review contracts.",
    "bundle": "Package exported dashboards, alerting resources, datasource inventory, and metadata into one source bundle. `bundle` is accepted as compatibility alias for `package`.",
}
WORKSPACE_COMMAND_HELP = SYNC_COMMAND_HELP


def _print_dashboard_group_help() -> None:
    """Print dedicated dashboard command help for the legacy/top-level entry path."""
    print(
        "Usage: grafana-util dashboard <COMMAND> [OPTIONS]\n\n"
        "Commands:\n"
        "  fetch-live         Fetch one live dashboard into a local draft file.\n"
        "  clone-live         Clone one live dashboard into a local draft file.\n"
        "  browse             Browse one local dashboard tree in a preview server.\n"
        "  edit-live          Edit one live dashboard in an external editor and optionally apply it live.\n"
        "  patch-file         Patch one local dashboard JSON file in place or to a new path.\n"
        "  serve              Run one local dashboard preview server.\n"
        "  review             Review one local dashboard JSON file without touching Grafana.\n"
        "  publish            Publish one local dashboard JSON file through the import pipeline.\n"
        "  raw-to-prompt      Convert raw dashboard JSON into prompt-lane artifacts.\n"
        "  analyze            Analyze dashboards from live Grafana or a local export tree.\n"
        "  validate-export    Validate one dashboard export tree without mutating Grafana.\n"
        "  history            List, export, or restore dashboard revision history.\n"
        "  export             Export dashboards into raw/ and prompt/ variants.\n"
        "  list               List live dashboard summaries from Grafana.\n"
        "  import             Import dashboards from exported raw JSON files.\n"
        "  delete             Delete live dashboards by UID or folder path.\n"
        "  diff               Compare exported raw dashboards with the current Grafana state; "
        "inspect provisioning trees separately with dashboard inspect-export --input-format provisioning.\n"
        "  inspect-export     Analyze a raw dashboard export directory offline.\n"
        "  inspect-live       Analyze live Grafana dashboards without writing a persistent export.\n"
        "  list-vars          List dashboard templating variables from live Grafana.\n"
        "  governance-gate    Evaluate dashboard governance policy against inspect artifacts.\n"
        "  topology (graph)   Build a deterministic dashboard topology graph from JSON artifacts.\n"
        "  impact             Summarize which dashboards and alerts would be affected by one datasource.\n"
        "  screenshot         Capture one Grafana dashboard or panel through a browser backend."
    )


def build_parser() -> argparse.ArgumentParser:
    """Build a unified parser that accepts namespaced and legacy command forms."""
    parser = argparse.ArgumentParser(
        prog="grafana-util",
        description=(
            "Unified Grafana CLI for dashboards, alerting resources, access "
            "management, datasource inventory, and declarative sync planning."
        ),
        epilog=(
            "Examples:\n\n"
    "  grafana-util dashboard export --url http://localhost:3000 --export-dir ./dashboards\n"
    "  grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt\n"
    "  grafana-util dashboard browse --input ./dashboards/raw --open-browser\n"
    "  grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main\n"
    "  grafana-util alert export --url http://localhost:3000 --output-dir ./alerts\n"
    '  grafana-util access user list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"\n'
    "  grafana-util datasource export --url http://localhost:3000 --export-dir ./datasources\n"
    "  grafana-util dashboard serve --input ./dashboards/raw --open-browser\n"
            "  grafana-util profile list\n"
            "  grafana-util change preview --workspace .\n"
            "  grafana-util overview live --profile prod\n"
            "  grafana-util status staged --desired-file ./desired.json\n"
            "  grafana-util snapshot review --input-dir ./snapshot\n"
            "  grafana-util resource kinds\n"
            "  grafana-util dashboard list-vars --dashboard-uid cpu-main --token \"$GRAFANA_API_TOKEN\"\n"
            "  grafana-util workspace plan --desired-file ./desired.json --live-file ./live.json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="entrypoint")
    subparsers.required = True

    dashboard_parser = subparsers.add_parser(
        "dashboard",
        help="Run dashboard export, list, import, or diff workflows.",
        aliases=["db"],
        add_help=False,
    )
    dashboard_subparsers = dashboard_parser.add_subparsers(dest="dashboard_command")
    dashboard_subparsers.required = False
    for command, help_text in DASHBOARD_COMMAND_HELP.items():
        dashboard_subparsers.add_parser(command, help=help_text, add_help=False)

    subparsers.add_parser(
        "alert",
        help="Run the alerting resource CLI under grafana-util alert ...",
        add_help=False,
    )
    subparsers.add_parser(
        "access",
        help="Run the access-management CLI under grafana-util access ...",
        add_help=False,
    )
    datasource_parser = subparsers.add_parser(
        "datasource",
        help="Run the datasource inventory CLI under grafana-util datasource ...",
        aliases=["ds"],
        add_help=False,
    )
    datasource_subparsers = datasource_parser.add_subparsers(dest="datasource_command")
    datasource_subparsers.required = False
    for command, help_text in DATASOURCE_COMMAND_HELP.items():
        datasource_subparsers.add_parser(command, help=help_text, add_help=False)
    workspace_parser = subparsers.add_parser(
        "workspace",
        help="Run the declarative workspace workflow under grafana-util workspace ...",
        add_help=False,
    )
    workspace_subparsers = workspace_parser.add_subparsers(dest="sync_command")
    workspace_subparsers.required = False
    for command, help_text in WORKSPACE_COMMAND_HELP.items():
        workspace_subparsers.add_parser(command, help=help_text, add_help=False)

    sync_parser = subparsers.add_parser(
        "sync",
        help="Backward-compatible alias for grafana-util workspace.",
        aliases=["sy"],
        add_help=False,
    )
    sync_subparsers = sync_parser.add_subparsers(dest="sync_command")
    sync_subparsers.required = False
    for command, help_text in SYNC_COMMAND_HELP.items():
        sync_subparsers.add_parser(command, help=help_text, add_help=False)
    subparsers.add_parser(
        "change",
        help="Run the review-first change workflow under grafana-util change ...",
        add_help=False,
    )
    subparsers.add_parser(
        "profile",
        help="Manage repo-local grafana-util profile configuration.",
        add_help=False,
    )
    subparsers.add_parser(
        "resource",
        help="Read a small set of live Grafana resources through a read-only surface.",
        add_help=False,
    )
    subparsers.add_parser(
        "overview",
        help="Render staged or live overview summaries across surfaces.",
        add_help=False,
    )
    subparsers.add_parser(
        "status",
        help="Render staged or live readiness and status checks.",
        add_help=False,
    )
    subparsers.add_parser(
        "snapshot",
        help="Export and review local snapshot inventory bundles.",
        add_help=False,
    )
    return parser


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    """Resolve command entrypoint and delegate argument normalization.

    Flow:
    - Normalize argv from real CLI invocation.
    - Route namespaced commands (`dashboard`, `alert`, `access`, `datasource`,
      `change`, `profile`, `resource`, `overview`, `status`, `snapshot`,
      `sync`) to their domain CLI modules.
    - Return the selected entrypoint plus domain-local argv slice for dispatch.
    """
    parser = build_parser()
    argv = list(sys.argv[1:] if argv is None else argv)

    # No argv means no explicit target command; keep UX stable by showing the
    # complete unified help and exiting 0.
    if not argv:
        parser.print_help()
        raise SystemExit(0)

    # Let the parser manage direct `-h`/`--help` and keep behavior consistent
    # with other module CLIs.
    if argv == ["-h"] or argv == ["--help"]:
        parser.print_help()
        raise SystemExit(0)

    command = argv[0]
    if command in ("dashboard", "db"):
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            _print_dashboard_group_help()
            raise SystemExit(0)
        mapped = UNIFIED_DASHBOARD_COMMAND_MAP.get(argv[1])
        if mapped:
            # Map modern dashboard subcommands (export/list/import/...) onto the
            # legacy argv shape consumed by dashboard_cli.
            return argparse.Namespace(
                entrypoint="dashboard",
                forwarded_argv=[mapped] + argv[2:],
            )
        parser.parse_args(argv)
        raise AssertionError(
            "argparse should have exited for unsupported dashboard command"
        )

    if command == "alert":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            alert_cli.build_parser(prog="grafana-util alert").print_help()
            raise SystemExit(0)
        # Namespace-preserving route for modern alert commands; delegated parser
        # handles command-specific defaults and output-mode normalization.
        return argparse.Namespace(entrypoint="alert", forwarded_argv=argv[1:])

    if command == "access":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            access_cli.build_parser(prog="grafana-util access").print_help()
            raise SystemExit(0)
        # Keep access entirely in its own parser module; this keeps unified routing
        # logic independent from access-specific auth and validation details.
        return argparse.Namespace(entrypoint="access", forwarded_argv=argv[1:])

    if command in ("datasource", "ds"):
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            datasource_cli.build_parser(prog="grafana-util datasource").print_help()
            raise SystemExit(0)
        if argv[1] not in DATASOURCE_COMMAND_HELP:
            parser.parse_args(argv)
            raise AssertionError(
                "argparse should have exited for unsupported datasource command"
            )
        # Keep datasource facade entrypoint aligned with dashboard-style split:
        # parse + normalize first, then delegate to workflow layer.
        return argparse.Namespace(
            entrypoint="datasource",
            forwarded_argv=argv[1:],
        )

    if command in ("workspace", "sync", "sy"):
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            if command == "workspace":
                sync_cli.build_parser(prog="grafana-util workspace").print_help()
            else:
                sync_cli.build_parser(prog="grafana-util sync").print_help()
            raise SystemExit(0)
        return argparse.Namespace(
            entrypoint="sync",
            forwarded_argv=argv[1:],
        )

    if command == "change":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            change_cli.build_parser(prog="grafana-util change").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="change", forwarded_argv=argv[1:])

    if command == "profile":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            profile_cli.build_parser(prog="grafana-util profile").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="profile", forwarded_argv=argv[1:])

    if command == "resource":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            resource_cli.build_parser(prog="grafana-util resource").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="resource", forwarded_argv=argv[1:])

    if command == "overview":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            overview_cli.build_parser(prog="grafana-util overview").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="overview", forwarded_argv=argv[1:])

    if command == "status":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            status_cli.build_parser(prog="grafana-util status").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="status", forwarded_argv=argv[1:])

    if command == "snapshot":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            snapshot_cli.build_parser(prog="grafana-util snapshot").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="snapshot", forwarded_argv=argv[1:])

    parser.parse_args(argv)
    raise AssertionError("argparse should have exited for unsupported command")


def main(argv: Optional[list[str]] = None) -> int:
    """Dispatch to the selected domain CLI module after unified argument mapping.

    Flow:
    - Parse args into a stable entrypoint.
    - Hand off to the matching module `main(...)`.
    - Preserve exit-code contract of the downstream module.
    """
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 153

    args = parse_args(argv)
    if args.entrypoint == "dashboard":
        return dashboard_cli.main(args.forwarded_argv)
    if args.entrypoint == "alert":
        return alert_cli.main(args.forwarded_argv)
    if args.entrypoint == "access":
        return access_cli.main(args.forwarded_argv)
    if args.entrypoint == "datasource":
        return datasource_cli.main(args.forwarded_argv)
    if args.entrypoint == "change":
        return change_cli.main(args.forwarded_argv)
    if args.entrypoint == "profile":
        return profile_cli.main(args.forwarded_argv)
    if args.entrypoint == "resource":
        return resource_cli.main(args.forwarded_argv)
    if args.entrypoint == "overview":
        return overview_cli.main(args.forwarded_argv)
    if args.entrypoint == "status":
        return status_cli.main(args.forwarded_argv)
    if args.entrypoint == "snapshot":
        return snapshot_cli.main(args.forwarded_argv)
    if args.entrypoint == "sync":
        return sync_cli.main(args.forwarded_argv)
    raise RuntimeError("Unsupported unified CLI entrypoint.")


if __name__ == "__main__":
    sys.exit(main())
