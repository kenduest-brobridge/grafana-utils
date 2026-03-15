#!/usr/bin/env python3
"""Unified Python entrypoint for dashboard, alert, access, datasource, and sync CLIs.

Purpose:
- Central CLI bootstrap for all Python commands so operators can use one binary
  (`grafana-util`) with namespaced commands and short aliases.

Architecture:
- Keep one entry process (`grafana-util`) that only does command routing.
- Delegate real argument parsing and execution to each domain CLI module so each
  domain can evolve independently.
"""

import argparse
import sys
from types import ModuleType
from typing import Optional

from . import access_cli, alert_cli, dashboard_cli, datasource_cli, sync_cli


DASHBOARD_COMMAND_HELP = {
    "export": "Export dashboards into raw/ and prompt/ variants.",
    "list": "List live dashboard summaries from Grafana.",
    "import": "Import dashboards from exported raw JSON files.",
    "diff": "Compare exported raw dashboards with the current Grafana state.",
    "inspect-export": "Analyze a raw dashboard export directory offline.",
    "inspect-live": "Analyze live Grafana dashboards without writing a persistent export.",
}
"""Canonical dashboard routing map consumed by args from top-level commands."""
UNIFIED_DASHBOARD_COMMAND_MAP = {
    "export": "export-dashboard",
    "list": "list-dashboard",
    "import": "import-dashboard",
    "diff": "diff",
    "inspect-export": "inspect-export",
    "inspect-live": "inspect-live",
}
ALERT_COMMAND_HELP = {
    "export": "Export alerting resources to JSON-formatted files.",
    "import": "Import alerting resources from JSON files.",
    "diff": "Diff alerting resources with previously exported snapshots.",
    "list-rules": "List alerting rule groups.",
    "list-contact-points": "List configured contact points.",
    "list-mute-timings": "List configured mute timings.",
    "list-templates": "List contact point templates.",
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
ENTRYPOINT_MODULE_DISPATCH: dict[str, ModuleType] = {
    "dashboard": dashboard_cli,
    "alert": alert_cli,
    "access": access_cli,
    "datasource": datasource_cli,
    "sync": sync_cli,
}
ENTRYPOINT_ALIASES = {
    "db": "dashboard",
    "ds": "datasource",
    "al": "alert",
    "ac": "access",
    "sy": "sync",
}
UNIFIED_TOP_LEVEL_HELP = (
    "Usage: grafana-util <COMMAND>\n\n"
    "Commands:\n"
    "  dashboard (db):\n"
    "    export, list, import, diff, inspect-export, inspect-live\n"
    "  datasource (ds):\n"
    "    list, add, modify, delete, export, import, diff\n"
    "  alert (al):\n"
    "    export, import, diff, list-rules, list-contact-points, list-mute-timings, list-templates\n"
    "  access (ac):\n"
    "    user, team, org, service-account\n"
    "  sync (sy):\n"
    "    plan, review, preflight, assess-alerts, bundle-preflight, apply\n\n"
    "Examples:\n"
    "  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n"
    "  grafana-util dashboard list --url http://localhost:3000 --table\n"
    "  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --layout tree --format table\n"
    '  grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite\n'
    "  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --table\n"
    "  grafana-util access team list --url http://localhost:3000 --basic-user admin --basic-password admin --table\n"
    "  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json"
)


def _print_unified_group_help() -> None:
    """Print dedicated top-level grouped help for the Python unified CLI."""
    print(UNIFIED_TOP_LEVEL_HELP)


def _print_dashboard_group_help() -> None:
    """Print dedicated dashboard command help for the legacy/top-level entry path."""
    print(
        "Usage: grafana-util dashboard <COMMAND> [OPTIONS]\n\n"
        "Commands:\n"
        "  export             Export dashboards into raw/ and prompt/ variants.\n"
        "  list               List live dashboard summaries from Grafana.\n"
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
            '--token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite\n\n'
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
        aliases=["al"],
        add_help=False,
    )
    # Keep parser topology strictly canonical; aliases and legacy command migration are
    # handled in this module instead of exposing extra commands in the help text.
    subparsers.add_parser(
        "access",
        help="Run the access-management CLI under grafana-util access ...",
        aliases=["ac"],
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
    sync_parser = subparsers.add_parser(
        "sync",
        help="Run the declarative sync planner under grafana-util sync ...",
        aliases=["sy"],
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
        _print_unified_group_help()
        raise SystemExit(0)

    # Let the parser manage direct `-h`/`--help` and keep behavior consistent
    # with other module CLIs.
    if argv == ["-h"] or argv == ["--help"]:
        _print_unified_group_help()
        raise SystemExit(0)

    command = ENTRYPOINT_ALIASES.get(argv[0], argv[0])

    if command == "dashboard":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            _print_dashboard_group_help()
            raise SystemExit(0)
        if argv[1] == "list-data-sources":
            return argparse.Namespace(
                entrypoint="datasource",
                forwarded_argv=["list"] + argv[2:],
            )
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
        raise RuntimeError("Unsupported unified CLI entrypoint: %s" % args.entrypoint)
    return module.main(args.forwarded_argv)


if __name__ == "__main__":
    sys.exit(main())
