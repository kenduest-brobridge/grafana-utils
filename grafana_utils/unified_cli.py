#!/usr/bin/env python3
"""Unified Python entrypoint for dashboard, alert, access, datasource, and sync CLIs.

Purpose:
- Central CLI bootstrap for all Python commands so operators can use one binary
  (`grafana-util`) while tests and scripts still support old top-level forms.

Architecture:
- Keep one entry process (`grafana-util`) that only does command routing.
- Preserve old top-level commands (legacy forms) while also supporting modern
  `grafana-util <module> <command>` style.
- Delegate real argument parsing and execution to each domain CLI module so each
  domain can evolve independently.

Usage notes:
- Top-level commands are accepted in both legacy and namespaced forms.
- No Grafana API logic is implemented here; this file only maps entrypoints.

Caveats:
- Do not add domain workflows in this module; keep behavior in `dashboard_cli`,
  `alert_cli`, `access_cli`, and `datasource_cli` to avoid hidden coupling.
"""

import argparse
import sys
from typing import Any, Dict, Optional

from . import access_cli, alert_cli, dashboard_cli, datasource_cli, sync_cli


DASHBOARD_COMMAND_HELP = {
    "export": "Export dashboards into raw/ and prompt/ variants.",
    "list": "List live dashboard summaries from Grafana.",
    "list-data-sources": "List live Grafana data sources.",
    "import": "Import dashboards from exported raw JSON files.",
    "diff": "Compare exported raw dashboards with the current Grafana state.",
    "inspect-export": "Analyze a raw dashboard export directory offline.",
    "inspect-live": "Analyze live Grafana dashboards without writing a persistent export.",
}
LEGACY_DASHBOARD_COMMAND_MAP = {
    "export-dashboard": "export-dashboard",
    "list-dashboard": "list-dashboard",
    "import-dashboard": "import-dashboard",
    "diff": "diff",
    "list-data-sources": "list-data-sources",
    "inspect-export": "inspect-export",
    "inspect-live": "inspect-live",
}
UNIFIED_DASHBOARD_COMMAND_MAP = {
    "export": "export-dashboard",
    "list": "list-dashboard",
    "import": "import-dashboard",
    "diff": "diff",
    "list-data-sources": "list-data-sources",
    "inspect-export": "inspect-export",
    "inspect-live": "inspect-live",
}
ALERT_COMMAND_HELP = {
    "export-alert": "Export alerting resources into raw/ JSON files.",
    "import-alert": "Import alerting resource JSON files through the Grafana API.",
    "diff-alert": "Compare local alerting export files against live Grafana resources.",
    "list-alert-rules": "List live Grafana alert rules.",
    "list-alert-contact-points": "List live Grafana alert contact points.",
    "list-alert-mute-timings": "List live Grafana mute timings.",
    "list-alert-templates": "List live Grafana notification templates.",
}
DEPRECATED_DIRECT_DASHBOARD_COMMAND_HELP = {
    "export": "Compatibility alias. Prefer `grafana-util dashboard export`.",
    "list": "Compatibility alias. Prefer `grafana-util dashboard list`.",
    "list-data-sources": "Compatibility alias. Prefer `grafana-util datasource list`.",
    "import": "Compatibility alias. Prefer `grafana-util dashboard import`.",
    "diff": "Compatibility alias. Prefer `grafana-util dashboard diff`.",
    "inspect-export": "Compatibility alias. Prefer `grafana-util dashboard inspect-export`.",
    "inspect-live": "Compatibility alias. Prefer `grafana-util dashboard inspect-live`.",
}
DEPRECATED_ALERT_COMMAND_HELP = {
    "export-alert": "Compatibility alias. Prefer `grafana-util alert export`.",
    "import-alert": "Compatibility alias. Prefer `grafana-util alert import`.",
    "diff-alert": "Compatibility alias. Prefer `grafana-util alert diff`.",
    "list-alert-rules": "Compatibility alias. Prefer `grafana-util alert list-rules`.",
    "list-alert-contact-points": "Compatibility alias. Prefer `grafana-util alert list-contact-points`.",
    "list-alert-mute-timings": "Compatibility alias. Prefer `grafana-util alert list-mute-timings`.",
    "list-alert-templates": "Compatibility alias. Prefer `grafana-util alert list-templates`.",
}
DATASOURCE_COMMAND_HELP = {
    "list": "List live Grafana datasource inventory.",
    "add": "Create one live Grafana datasource through the Grafana API.",
    "delete": "Delete one live Grafana datasource through the Grafana API.",
    "export": "Export live Grafana datasource inventory as normalized JSON files.",
    "import": "Import datasource inventory JSON through the Grafana API.",
    "diff": "Compare exported datasource inventory with the current Grafana state.",
}
SYNC_COMMAND_HELP = {
    "plan": "Build one reviewable sync plan from desired/live JSON files.",
    "review": "Mark one sync plan document as reviewed.",
    "preflight": "Build one staged sync preflight document from local JSON inputs.",
    "assess-alerts": "Assess alert sync specs for candidate, plan-only, and blocked states.",
    "bundle-preflight": "Build one staged bundle-level preflight document from local JSON inputs.",
    "apply": "Build a gated non-live apply intent from a reviewed plan.",
}
LEGACY_ALERT_COMMAND_MAP = {
    "export-alert": "export",
    "import-alert": "import",
    "diff-alert": "diff",
    "list-alert-rules": "list-rules",
    "list-alert-contact-points": "list-contact-points",
    "list-alert-mute-timings": "list-mute-timings",
    "list-alert-templates": "list-templates",
}
ENTRYPOINT_MODULE_DISPATCH = {
    "dashboard": dashboard_cli,
    "alert": alert_cli,
    "access": access_cli,
    "datasource": datasource_cli,
    "sync": sync_cli,
}  # type: Dict[str, Any]

def _print_dashboard_group_help() -> None:
    """Print dedicated dashboard command help for the legacy/top-level entry path."""
    print(
        "Usage: grafana-util dashboard <COMMAND> [OPTIONS]\n\n"
        "Commands:\n"
        "  export             Export dashboards into raw/ and prompt/ variants.\n"
        "  list               List live dashboard summaries from Grafana.\n"
        "  list-data-sources  Compatibility alias; prefer `grafana-util datasource list`.\n"
        "  import             Import dashboards from exported raw JSON files.\n"
        "  diff               Compare exported raw dashboards with the current Grafana state.\n"
        "  inspect-export     Analyze a raw dashboard export directory offline.\n"
        "  inspect-live       Analyze live Grafana dashboards without writing a persistent export.\n\n"
        "Examples:\n"
        "  grafana-util dashboard list --url http://localhost:3000 --table\n"
        "  grafana-util datasource list --url http://localhost:3000 --table\n"
        "  grafana-util dashboard import --url http://localhost:3000 --import-dir ./dashboards/raw --replace-existing --dry-run"
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
            "  Export dashboards across all visible orgs with Basic auth:\n"
            "    grafana-util dashboard export --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n"
            "  Preview a routed dashboard import before writing:\n"
            "    grafana-util dashboard import --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --import-dir ./dashboards "
            "--use-export-org --create-missing-orgs --dry-run --output-format table\n\n"
            "  Inspect exported dashboards as a query tree:\n"
            "    grafana-util dashboard inspect-export --import-dir ./dashboards/raw "
            "--view query --layout tree --format table\n\n"
            "  Export alerting resources from the current org with an API token:\n"
            "    grafana-util alert export --url http://localhost:3000 "
            "--token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n"
            "  List Grafana organizations with memberships:\n"
            "    grafana-util access org list --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --with-users --table\n\n"
            "  List current-org teams with member details:\n"
            "    grafana-util access team list --url http://localhost:3000 "
            "--basic-user admin --basic-password admin --table\n\n"
            "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="entrypoint")
    subparsers.required = True

    dashboard_parser = subparsers.add_parser(
        "dashboard",
        help="Run dashboard export, list, import, or diff workflows.",
        add_help=False,
    )
    dashboard_subparsers = dashboard_parser.add_subparsers(dest="dashboard_command")
    dashboard_subparsers.required = False
    for command, help_text in DASHBOARD_COMMAND_HELP.items():
        if command == "list-data-sources":
            help_text = "Compatibility alias. Prefer `grafana-util datasource list`."
        dashboard_subparsers.add_parser(command, help=help_text, add_help=False)

    for command, help_text in DEPRECATED_DIRECT_DASHBOARD_COMMAND_HELP.items():
        subparsers.add_parser(command, help=help_text, add_help=False)

    subparsers.add_parser(
        "alert",
        help="Run the alerting resource CLI under grafana-util alert ...",
        add_help=False,
    )
    for command, help_text in DEPRECATED_ALERT_COMMAND_HELP.items():
        subparsers.add_parser(command, help=help_text, add_help=False)
    subparsers.add_parser(
        "access",
        help="Run the access-management CLI under grafana-util access ...",
        add_help=False,
    )
    datasource_parser = subparsers.add_parser(
        "datasource",
        help="Run the datasource inventory CLI under grafana-util datasource ...",
        add_help=False,
    )
    datasource_subparsers = datasource_parser.add_subparsers(dest="datasource_command")
    datasource_subparsers.required = False
    for command, help_text in DATASOURCE_COMMAND_HELP.items():
        datasource_subparsers.add_parser(command, help=help_text, add_help=False)
    sync_parser = subparsers.add_parser(
        "sync",
        help="Run the declarative sync planner under grafana-util sync ...",
        add_help=False,
    )
    sync_subparsers = sync_parser.add_subparsers(dest="sync_command")
    sync_subparsers.required = False
    for command, help_text in SYNC_COMMAND_HELP.items():
        sync_subparsers.add_parser(command, help=help_text, add_help=False)
    return parser

def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    """Resolve command entrypoint and delegate argument normalization.

    Flow:
    - Normalize argv from real CLI invocation.
    - Route legacy top-level commands through explicit compatibility maps.
    - Route namespaced commands (`dashboard`, `alert`, `access`, `datasource`) to
      their domain CLI modules.
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
    if command == "dashboard":
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
        raise AssertionError("argparse should have exited for unsupported dashboard command")

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

    if command == "datasource":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            datasource_cli.build_parser(prog="grafana-util datasource").print_help()
            raise SystemExit(0)
        # Keep datasource facade entrypoint aligned with dashboard-style split:
        # parse + normalize first, then delegate to workflow layer.
        return argparse.Namespace(
            entrypoint="datasource",
            forwarded_argv=argv[1:],
        )

    if command == "sync":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            sync_cli.build_parser(prog="grafana-util sync").print_help()
            raise SystemExit(0)
        return argparse.Namespace(
            entrypoint="sync",
            forwarded_argv=argv[1:],
        )

    mapped = LEGACY_DASHBOARD_COMMAND_MAP.get(command)
    if mapped:
        return argparse.Namespace(
            entrypoint="dashboard",
            forwarded_argv=[mapped] + argv[1:],
        )

    mapped = LEGACY_ALERT_COMMAND_MAP.get(command)
    if mapped:
        return argparse.Namespace(
            entrypoint="alert",
            forwarded_argv=[mapped] + argv[1:],
        )

    parser.parse_args(argv)
    raise AssertionError("argparse should have exited for unsupported command")


def main(argv: Optional[list[str]] = None) -> int:
    """Dispatch to the selected domain CLI module after unified argument mapping.

    Flow:
    - Parse args into a stable entrypoint.
    - Hand off to the matching module `main(...)`.
    - Preserve exit-code contract of the downstream module.
    """
    args = parse_args(argv)
    module = ENTRYPOINT_MODULE_DISPATCH.get(args.entrypoint)
    if module is None:
        raise RuntimeError(
            "Unsupported unified CLI entrypoint: %s" % args.entrypoint
        )
    return module.main(args.forwarded_argv)


if __name__ == "__main__":
    sys.exit(main())
