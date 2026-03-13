#!/usr/bin/env python3
"""Unified Python entrypoint for dashboard, alert, access, and datasource CLIs."""

import argparse
import sys
from typing import List, Optional

from . import access_cli, alert_cli, dashboard_cli, datasource_cli


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
DATASOURCE_COMMAND_HELP = {
    "list": "List live Grafana datasource inventory.",
    "export": "Export live Grafana datasource inventory as normalized JSON files.",
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


def _print_dashboard_group_help() -> None:
    print(
        "Usage: grafana-utils dashboard <COMMAND> [OPTIONS]\n\n"
        "Commands:\n"
        "  export             Export dashboards into raw/ and prompt/ variants.\n"
        "  list               List live dashboard summaries from Grafana.\n"
        "  list-data-sources  List live Grafana data sources.\n"
        "  import             Import dashboards from exported raw JSON files.\n"
        "  diff               Compare exported raw dashboards with the current Grafana state.\n"
        "  inspect-export     Analyze a raw dashboard export directory offline.\n"
        "  inspect-live       Analyze live Grafana dashboards without writing a persistent export."
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="grafana-utils",
        description=(
            "Unified Grafana CLI for dashboards, alerting resources, access "
            "management, and datasource inventory."
        ),
        epilog=(
            "Examples:\n\n"
            "  grafana-utils dashboard export --url http://localhost:3000 --export-dir ./dashboards\n"
            "  grafana-utils alert export --url http://localhost:3000 --output-dir ./alerts\n"
            "  grafana-utils access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n"
            "  grafana-utils datasource export --url http://localhost:3000 --export-dir ./datasources"
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
        dashboard_subparsers.add_parser(command, help=help_text, add_help=False)

    for command, help_text in DASHBOARD_COMMAND_HELP.items():
        subparsers.add_parser(command, help="%s (legacy direct form)." % help_text, add_help=False)

    subparsers.add_parser(
        "alert",
        help="Run the alerting resource CLI under grafana-utils alert ...",
        add_help=False,
    )
    for command, help_text in ALERT_COMMAND_HELP.items():
        subparsers.add_parser(command, help="%s (legacy direct form)." % help_text, add_help=False)
    subparsers.add_parser(
        "access",
        help="Run the access-management CLI under grafana-utils access ...",
        add_help=False,
    )
    datasource_parser = subparsers.add_parser(
        "datasource",
        help="Run the datasource inventory CLI under grafana-utils datasource ...",
        add_help=False,
    )
    datasource_subparsers = datasource_parser.add_subparsers(dest="datasource_command")
    datasource_subparsers.required = False
    for command, help_text in DATASOURCE_COMMAND_HELP.items():
        datasource_subparsers.add_parser(command, help=help_text, add_help=False)
    return parser


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = build_parser()
    argv = list(sys.argv[1:] if argv is None else argv)

    if not argv:
        parser.print_help()
        raise SystemExit(0)

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
            return argparse.Namespace(entrypoint="dashboard", forwarded_argv=[mapped] + argv[2:])
        parser.parse_args(argv)
        raise AssertionError("argparse should have exited for unsupported dashboard command")

    if command == "alert":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            alert_cli.build_parser(prog="grafana-utils alert").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="alert", forwarded_argv=argv[1:])

    if command == "access":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            access_cli.build_parser(prog="grafana-utils access").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="access", forwarded_argv=argv[1:])

    if command == "datasource":
        if len(argv) == 1 or argv[1] in ("-h", "--help"):
            datasource_cli.build_parser(prog="grafana-utils datasource").print_help()
            raise SystemExit(0)
        return argparse.Namespace(entrypoint="datasource", forwarded_argv=argv[1:])

    mapped = LEGACY_DASHBOARD_COMMAND_MAP.get(command)
    if mapped:
        return argparse.Namespace(entrypoint="dashboard", forwarded_argv=[mapped] + argv[1:])

    mapped = LEGACY_ALERT_COMMAND_MAP.get(command)
    if mapped:
        return argparse.Namespace(entrypoint="alert", forwarded_argv=[mapped] + argv[1:])

    parser.parse_args(argv)
    raise AssertionError("argparse should have exited for unsupported command")


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    if args.entrypoint == "dashboard":
        return dashboard_cli.main(args.forwarded_argv)
    if args.entrypoint == "alert":
        return alert_cli.main(args.forwarded_argv)
    if args.entrypoint == "access":
        return access_cli.main(args.forwarded_argv)
    if args.entrypoint == "datasource":
        return datasource_cli.main(args.forwarded_argv)
    raise RuntimeError("Unsupported unified CLI entrypoint.")
if __name__ == "__main__":
    sys.exit(main())
