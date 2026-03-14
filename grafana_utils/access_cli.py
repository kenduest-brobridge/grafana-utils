#!/usr/bin/env python3
"""List Grafana users, teams, and service accounts through Grafana APIs.

Primary command surface:
- `grafana-utils access user list`
- `grafana-utils access user add`
- `grafana-utils access user modify`
- `grafana-utils access user delete`
- `grafana-utils access team list`
- `grafana-utils access team add`
- `grafana-utils access team modify`
- `grafana-utils access team delete`
- `grafana-utils access group ...`
- `grafana-utils access service-account list`
- `grafana-utils access service-account add`
- `grafana-utils access service-account delete`
- `grafana-utils access service-account token add`
- `grafana-utils access service-account token delete`

Design notes:
- org-scoped listing can use token auth or Basic auth
- global user listing requires Basic auth because Grafana's global user API is
  server-admin oriented
- `--with-teams` also requires Basic auth because it uses per-user team lookup
- output modes intentionally mirror the existing CLI family: compact text,
  table, CSV, and JSON
"""

import argparse
import csv
import getpass
import json
import sys
from typing import Any, Dict, List, Optional, Tuple

from .access.common import (
    DEFAULT_PAGE_SIZE,
    OUTPUT_FIELDS,
    SERVICE_ACCOUNT_OUTPUT_FIELDS,
    SERVICE_ACCOUNT_TOKEN_OUTPUT_FIELDS,
    TEAM_OUTPUT_FIELDS,
    GrafanaApiError,
    GrafanaError,
)
from .access.models import (
    bool_label,
    build_team_rows,
    build_user_rows,
    format_service_account_summary_line,
    format_team_add_summary_line,
    format_team_modify_summary_line,
    format_team_summary_line,
    normalize_bool,
    normalize_global_user,
    normalize_org_role,
    normalize_org_user,
    normalize_service_account,
    render_service_account_csv,
    render_service_account_json,
    render_service_account_table,
    render_service_account_token_json,
    render_team_csv,
    render_team_json,
    render_team_table,
    render_user_csv,
    render_user_json,
    render_user_table,
    service_account_matches_query,
    serialize_service_account_row,
    serialize_service_account_token_row,
    serialize_user_row,
)
from .access.pending_cli_staging import (
    add_service_account_delete_cli_args,
    add_service_account_token_delete_cli_args,
    add_team_delete_cli_args,
    normalize_group_alias_argv,
    resolve_service_account_id,
    resolve_service_account_token_record,
    resolve_team_id,
    validate_destructive_confirmed,
)
from .auth_staging import AuthConfigError, resolve_auth_from_namespace
from .clients.access_client import GrafanaAccessClient


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_SCOPE = "org"
DEFAULT_SERVICE_ACCOUNT_ROLE = "Viewer"
SCOPE_CHOICES = ("org", "global")
LIST_OUTPUT_FORMAT_CHOICES = ("text", "table", "csv", "json")


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("value must be >= 1")
    return parsed


def bool_choice(value: str) -> str:
    normalized = str(value).strip().lower()
    if normalized not in {"true", "false"}:
        raise argparse.ArgumentTypeError("value must be true or false")
    return normalized


def add_list_output_format_arg(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--output-format",
        choices=LIST_OUTPUT_FORMAT_CHOICES,
        default=None,
        help=(
            "Alternative single-flag output selector for list output. "
            "Use text, table, csv, or json. This cannot be combined with "
            "--table, --csv, or --json."
        ),
    )


def build_parser(prog: Optional[str] = None) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=prog,
        description="List Grafana users, teams, and manage Grafana service accounts."
    )
    subparsers = parser.add_subparsers(dest="resource")
    subparsers.required = True

    user_parser = subparsers.add_parser(
        "user",
        help="List Grafana users.",
    )
    user_subparsers = user_parser.add_subparsers(dest="command")
    user_subparsers.required = True

    list_parser = user_subparsers.add_parser(
        "list",
        help="List Grafana users from org-scoped or global APIs.",
    )
    add_common_cli_args(list_parser)
    add_user_list_cli_args(list_parser)

    add_parser = user_subparsers.add_parser(
        "add",
        help="Create a Grafana user through the global admin API.",
    )
    add_common_cli_args(
        add_parser,
        allow_legacy_auth_aliases=False,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_add_cli_args(add_parser)

    modify_parser = user_subparsers.add_parser(
        "modify",
        help="Modify a Grafana user through the global admin APIs.",
    )
    add_common_cli_args(
        modify_parser,
        allow_legacy_auth_aliases=False,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_modify_cli_args(modify_parser)

    delete_parser = user_subparsers.add_parser(
        "delete",
        help="Delete a Grafana user from the org or globally.",
    )
    add_common_cli_args(
        delete_parser,
        allow_legacy_auth_aliases=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_delete_cli_args(delete_parser)

    team_parser = subparsers.add_parser(
        "team",
        help="List Grafana teams.",
    )
    team_subparsers = team_parser.add_subparsers(dest="command")
    team_subparsers.required = True

    team_list_parser = team_subparsers.add_parser(
        "list",
        help="List Grafana teams from the org-scoped API.",
    )
    add_common_cli_args(team_list_parser)
    add_team_list_cli_args(team_list_parser)

    team_add_parser = team_subparsers.add_parser(
        "add",
        help="Create a Grafana team and optionally seed members and admins.",
    )
    add_common_cli_args(team_add_parser)
    add_team_add_cli_args(team_add_parser)

    team_modify_parser = team_subparsers.add_parser(
        "modify",
        help="Modify Grafana team members and team admins.",
    )
    add_common_cli_args(team_modify_parser)
    add_team_modify_cli_args(team_modify_parser)

    team_delete_parser = team_subparsers.add_parser(
        "delete",
        help="Delete a Grafana team.",
    )
    add_common_cli_args(team_delete_parser)
    add_team_delete_cli_args(team_delete_parser)

    service_account_parser = subparsers.add_parser(
        "service-account",
        help="List, create, and delete Grafana service accounts.",
    )
    service_account_subparsers = service_account_parser.add_subparsers(dest="command")
    service_account_subparsers.required = True

    service_account_list_parser = service_account_subparsers.add_parser(
        "list",
        help="List Grafana service accounts.",
    )
    add_common_cli_args(service_account_list_parser)
    add_service_account_list_cli_args(service_account_list_parser)

    service_account_add_parser = service_account_subparsers.add_parser(
        "add",
        help="Create a Grafana service account.",
    )
    add_common_cli_args(service_account_add_parser)
    add_service_account_add_cli_args(service_account_add_parser)

    service_account_delete_parser = service_account_subparsers.add_parser(
        "delete",
        help="Delete a Grafana service account.",
    )
    add_common_cli_args(service_account_delete_parser)
    add_service_account_delete_cli_args(service_account_delete_parser)

    service_account_token_parser = service_account_subparsers.add_parser(
        "token",
        help="Manage Grafana service-account tokens.",
    )
    service_account_token_subparsers = service_account_token_parser.add_subparsers(
        dest="token_command"
    )
    service_account_token_subparsers.required = True

    service_account_token_add_parser = service_account_token_subparsers.add_parser(
        "add",
        help="Create a Grafana service-account token.",
    )
    add_common_cli_args(service_account_token_add_parser)
    add_service_account_token_add_cli_args(service_account_token_add_parser)

    service_account_token_delete_parser = service_account_token_subparsers.add_parser(
        "delete",
        help="Delete a Grafana service-account token.",
    )
    add_common_cli_args(service_account_token_delete_parser)
    add_service_account_token_delete_cli_args(service_account_token_delete_parser)
    return parser


def add_common_cli_args(
    parser: argparse.ArgumentParser,
    allow_legacy_auth_aliases: bool = True,
    allow_token_auth: bool = True,
    username_dest: str = "username",
    password_dest: str = "password",
) -> None:
    parser.add_argument(
        "--url",
        default=DEFAULT_URL,
        help="Grafana base URL (default: %s)" % DEFAULT_URL,
    )
    if allow_token_auth:
        parser.add_argument(
            "--token",
            "--api-token",
            dest="api_token",
            default=None,
            metavar="TOKEN",
            help=(
                "Grafana API token. Preferred flag: --token. "
                "Falls back to GRAFANA_API_TOKEN."
            ),
        )
    basic_user_flags = ["--basic-user"]
    basic_password_flags = ["--basic-password"]
    if allow_legacy_auth_aliases:
        basic_user_flags.append("--username")
        basic_password_flags.append("--password")
    parser.add_argument(
        *basic_user_flags,
        dest=username_dest,
        default=None,
        metavar="USERNAME",
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    parser.add_argument(
        *basic_password_flags,
        dest=password_dest,
        default=None,
        metavar="PASSWORD",
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
        ),
    )
    parser.add_argument(
        "--prompt-password",
        action="store_true",
        help=(
            "Prompt for the Grafana Basic auth password without echo instead of "
            "passing --basic-password on the command line."
        ),
    )
    parser.add_argument(
        "--org-id",
        default=None,
        help="Grafana organization id to send through X-Grafana-Org-Id.",
    )
    parser.add_argument(
        "--timeout",
        type=positive_int,
        default=DEFAULT_TIMEOUT,
        help="HTTP timeout in seconds (default: %s)." % DEFAULT_TIMEOUT,
    )
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )


def add_user_list_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--scope",
        choices=SCOPE_CHOICES,
        default=DEFAULT_SCOPE,
        help="Choose org-scoped or global user listing (default: %s)." % DEFAULT_SCOPE,
    )
    parser.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match across login, email, and name.",
    )
    parser.add_argument(
        "--login",
        default=None,
        help="Filter to one exact login.",
    )
    parser.add_argument(
        "--email",
        default=None,
        help="Filter to one exact email.",
    )
    parser.add_argument(
        "--org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Filter by Grafana organization role.",
    )
    parser.add_argument(
        "--grafana-admin",
        default=None,
        type=bool_choice,
        help="Filter by Grafana server-admin state: true or false.",
    )
    parser.add_argument(
        "--with-teams",
        action="store_true",
        help="Include team memberships. Requires Basic auth.",
    )
    parser.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Page number after filtering (default: 1).",
    )
    parser.add_argument(
        "--per-page",
        type=positive_int,
        default=DEFAULT_PAGE_SIZE,
        help="Items per page after filtering (default: %s)." % DEFAULT_PAGE_SIZE,
    )
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render users as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render users as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render users as JSON.",
    )
    add_list_output_format_arg(parser)


def add_user_add_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--login",
        required=True,
        help="Login name for the new Grafana user.",
    )
    parser.add_argument(
        "--email",
        required=True,
        help="Email address for the new Grafana user.",
    )
    parser.add_argument(
        "--name",
        required=True,
        help="Display name for the new Grafana user.",
    )
    parser.add_argument(
        "--password",
        dest="new_user_password",
        required=True,
        help="Password for the new local Grafana user.",
    )
    parser.add_argument(
        "--org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Optional Grafana organization role to set after user creation.",
    )
    parser.add_argument(
        "--grafana-admin",
        default=None,
        type=bool_choice,
        help="Optional Grafana server-admin state to set after user creation: true or false.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the created user as JSON.",
    )


def add_user_modify_cli_args(parser: argparse.ArgumentParser) -> None:
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--user-id",
        default=None,
        help="Modify the user identified by this Grafana user id.",
    )
    identity_group.add_argument(
        "--login",
        default=None,
        help="Resolve the user by exact login before modifying it.",
    )
    identity_group.add_argument(
        "--email",
        default=None,
        help="Resolve the user by exact email before modifying it.",
    )
    parser.add_argument(
        "--set-login",
        default=None,
        help="Set a new login for the target user.",
    )
    parser.add_argument(
        "--set-email",
        default=None,
        help="Set a new email address for the target user.",
    )
    parser.add_argument(
        "--set-name",
        default=None,
        help="Set a new display name for the target user.",
    )
    parser.add_argument(
        "--set-password",
        default=None,
        help="Set a new local password for the target user.",
    )
    parser.add_argument(
        "--set-org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Optional Grafana organization role to set after profile changes.",
    )
    parser.add_argument(
        "--set-grafana-admin",
        default=None,
        type=bool_choice,
        help="Optional Grafana server-admin state to set after profile changes: true or false.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the modified user as JSON.",
    )


def add_user_delete_cli_args(parser: argparse.ArgumentParser) -> None:
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--user-id",
        default=None,
        help="Delete the user identified by this Grafana user id.",
    )
    identity_group.add_argument(
        "--login",
        default=None,
        help="Resolve the user by exact login before deleting it.",
    )
    identity_group.add_argument(
        "--email",
        default=None,
        help="Resolve the user by exact email before deleting it.",
    )
    parser.add_argument(
        "--scope",
        choices=SCOPE_CHOICES,
        default="global",
        help="Choose org-scoped removal or global deletion (default: global).",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm that the target user should be deleted or removed.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the deleted user summary as JSON.",
    )


def add_service_account_list_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against service-account name or login.",
    )
    parser.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Grafana search page number (default: 1).",
    )
    parser.add_argument(
        "--per-page",
        type=positive_int,
        default=DEFAULT_PAGE_SIZE,
        help="Grafana search page size (default: %s)." % DEFAULT_PAGE_SIZE,
    )
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render service accounts as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render service accounts as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render service accounts as JSON.",
    )
    add_list_output_format_arg(parser)


def add_team_list_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against team name or email.",
    )
    parser.add_argument(
        "--name",
        default=None,
        help="Filter to one exact team name.",
    )
    parser.add_argument(
        "--with-members",
        action="store_true",
        help="Include team member login names when the API returns them.",
    )
    parser.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Page number after filtering (default: 1).",
    )
    parser.add_argument(
        "--per-page",
        type=positive_int,
        default=DEFAULT_PAGE_SIZE,
        help="Items per page after filtering (default: %s)." % DEFAULT_PAGE_SIZE,
    )
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render teams as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render teams as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render teams as JSON.",
    )
    add_list_output_format_arg(parser)


def add_team_modify_cli_args(parser: argparse.ArgumentParser) -> None:
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--team-id",
        default=None,
        help="Modify the team identified by this Grafana team id.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the team by exact name before modifying memberships.",
    )
    parser.add_argument(
        "--add-member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one team member by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--remove-member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Remove one team member by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--add-admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Promote one user to team admin by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--remove-admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Demote one team admin to regular team member by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the team modification result as JSON.",
    )


def add_team_add_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--name",
        required=True,
        help="Team name to create.",
    )
    parser.add_argument(
        "--email",
        default=None,
        help="Optional team email address to store in Grafana.",
    )
    parser.add_argument(
        "--member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one initial team member by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one initial team admin by exact login or exact email. Repeat as needed.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the created team as JSON.",
    )


def add_service_account_add_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--name",
        required=True,
        help="Service-account name to create.",
    )
    parser.add_argument(
        "--role",
        default=DEFAULT_SERVICE_ACCOUNT_ROLE,
        choices=["Viewer", "Editor", "Admin", "None"],
        help=(
            "Service-account org role (default: %s)." % DEFAULT_SERVICE_ACCOUNT_ROLE
        ),
    )
    parser.add_argument(
        "--disabled",
        default="false",
        type=bool_choice,
        help="Create the service account in disabled state: true or false.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the created service account as JSON.",
    )


def add_service_account_token_add_cli_args(parser: argparse.ArgumentParser) -> None:
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--service-account-id",
        default=None,
        help="Service-account id that should own the new token.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the service account by exact name before creating the token.",
    )
    parser.add_argument(
        "--token-name",
        required=True,
        help="Token name to create under the target service account.",
    )
    parser.add_argument(
        "--seconds-to-live",
        type=positive_int,
        default=None,
        help="Optional token lifetime in seconds.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the created token payload as JSON.",
    )


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = build_parser()
    argv = normalize_group_alias_argv(
        list(sys.argv[1:] if argv is None else argv)
    )

    if not argv:
        parser.print_help()
        raise SystemExit(0)

    if argv == ["user"]:
        parser._subparsers._group_actions[0].choices["user"].print_help()
        raise SystemExit(0)

    if argv == ["team"]:
        parser._subparsers._group_actions[0].choices["team"].print_help()
        raise SystemExit(0)

    if argv == ["group"]:
        parser._subparsers._group_actions[0].choices["team"].print_help()
        raise SystemExit(0)

    if argv == ["service-account"]:
        parser._subparsers._group_actions[0].choices["service-account"].print_help()
        raise SystemExit(0)

    if argv == ["service-account", "token"]:
        parser._subparsers._group_actions[0].choices["service-account"]._subparsers._group_actions[0].choices["token"].print_help()
        raise SystemExit(0)

    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    return args


def _normalize_output_format_args(
    args: argparse.Namespace,
    parser: argparse.ArgumentParser,
) -> None:
    output_format = getattr(args, "output_format", None)
    if output_format is None:
        return
    if bool(getattr(args, "table", False)) or bool(getattr(args, "csv", False)) or bool(
        getattr(args, "json", False)
    ):
        parser.error(
            "--output-format cannot be combined with --table, --csv, or --json for access list commands."
        )
    args.table = output_format == "table"
    args.csv = output_format == "csv"
    args.json = output_format == "json"


def resolve_auth(args: argparse.Namespace) -> Tuple[Dict[str, str], str]:
    try:
        return resolve_auth_from_namespace(
            args,
            prompt_reader=getpass.getpass,
        )
    except AuthConfigError as exc:
        message = str(exc)
        if message == "Choose either token auth or Basic auth, not both.":
            message = (
                "Choose either token auth (--token / --api-token) or Basic auth "
                "(--basic-user / --username with --basic-password / --password / "
                "--prompt-password), not both."
            )
        elif (
            message
            == "Choose either an explicit Basic auth password or --prompt-password, not both."
        ):
            message = (
                "Choose either --basic-password / --password or "
                "--prompt-password, not both."
            )
        elif (
            message
            == "Basic auth requires both username and password or --prompt-password."
        ):
            message = (
                "Basic auth requires both --basic-user / --username and "
                "--basic-password / --password or --prompt-password."
            )
        elif message == "--prompt-password requires a Basic auth username.":
            message = "--prompt-password requires --basic-user / --username."
        elif (
            message
            == "Basic auth environment configuration requires both GRAFANA_USERNAME and GRAFANA_PASSWORD."
        ):
            message = (
                "Basic auth requires both --basic-user / --username and "
                "--basic-password / --password or --prompt-password."
            )
        elif (
            message
            == "Authentication required. Provide a token or Basic auth credentials."
        ):
            message = (
                "Authentication required. Set --token / --api-token / "
                "GRAFANA_API_TOKEN or --basic-user and --basic-password / "
                "--prompt-password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
            )
        raise GrafanaError(message)


def build_request_headers(args: argparse.Namespace) -> Tuple[Dict[str, str], str]:
    return resolve_auth(args)


def validate_user_list_auth(args: argparse.Namespace, auth_mode: str) -> None:
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User list with --scope global requires Basic auth "
            "(--basic-user / --basic-password)."
        )
    if args.with_teams and auth_mode != "basic":
        raise GrafanaError("--with-teams requires Basic auth.")


def validate_user_add_auth(auth_mode: str) -> None:
    if auth_mode != "basic":
        raise GrafanaError(
            "User add requires Basic auth (--basic-user / --basic-password)."
        )


def validate_user_modify_args(args: argparse.Namespace) -> None:
    if not (
        args.set_login
        or args.set_email
        or args.set_name
        or args.set_password
        or args.set_org_role
        or args.set_grafana_admin is not None
    ):
        raise GrafanaError(
            "User modify requires at least one of --set-login, --set-email, "
            "--set-name, --set-password, --set-org-role, or --set-grafana-admin."
        )


def validate_user_modify_auth(auth_mode: str) -> None:
    if auth_mode != "basic":
        raise GrafanaError(
            "User modify requires Basic auth (--basic-user / --basic-password)."
        )


def validate_user_delete_args(args: argparse.Namespace) -> None:
    if not args.yes:
        raise GrafanaError("User delete requires --yes.")


def validate_user_delete_auth(args: argparse.Namespace, auth_mode: str) -> None:
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User delete with --scope global requires Basic auth "
            "(--basic-user / --basic-password)."
        )


def validate_team_modify_args(args: argparse.Namespace) -> None:
    if not (
        args.add_member
        or args.remove_member
        or args.add_admin
        or args.remove_admin
    ):
        raise GrafanaError(
            "Team modify requires at least one of --add-member, --remove-member, "
            "--add-admin, or --remove-admin."
        )


def validate_team_delete_auth(_auth_mode: str) -> None:
    return None


def validate_service_account_delete_auth(_auth_mode: str) -> None:
    return None


def validate_service_account_token_delete_auth(_auth_mode: str) -> None:
    return None


def service_account_role_to_api(value: str) -> str:
    normalized = normalize_org_role(value)
    if normalized == "None":
        return "NoBasicRole"
    return normalized


def normalize_created_user(
    user_id: Any,
    args: argparse.Namespace,
) -> Dict[str, Any]:
    return {
        "id": str(user_id or ""),
        "login": str(args.login or ""),
        "email": str(args.email or ""),
        "name": str(args.name or ""),
        "orgRole": normalize_org_role(args.org_role),
        "grafanaAdmin": normalize_bool(args.grafana_admin),
        "scope": "global",
        "teams": [],
    }


def lookup_service_account_id_by_name(
    client: GrafanaAccessClient,
    service_account_name: str,
) -> str:
    candidates = client.list_service_accounts(
        query=service_account_name,
        page=1,
        per_page=DEFAULT_PAGE_SIZE,
    )
    exact_matches = []
    for item in candidates:
        if str(item.get("name") or "") == service_account_name:
            exact_matches.append(item)
    if not exact_matches:
        raise GrafanaError(
            "Service account not found by name: %s" % service_account_name
        )
    if len(exact_matches) > 1:
        raise GrafanaError(
            "Service account name matched multiple items: %s"
            % service_account_name
        )
    service_account_id = exact_matches[0].get("id")
    if not service_account_id:
        raise GrafanaError(
            "Service account lookup response did not include an id for %s."
            % service_account_name
        )
    return str(service_account_id)


def lookup_team_by_name(
    client: GrafanaAccessClient,
    team_name: str,
) -> Dict[str, Any]:
    candidates = client.iter_teams(
        query=team_name,
        page_size=DEFAULT_PAGE_SIZE,
    )
    exact_matches = []
    for item in candidates:
        if str(item.get("name") or "") == team_name:
            exact_matches.append(item)
    if not exact_matches:
        raise GrafanaError("Team not found by name: %s" % team_name)
    if len(exact_matches) > 1:
        raise GrafanaError("Team name matched multiple items: %s" % team_name)
    return dict(exact_matches[0])


def lookup_org_user_by_identity(
    client: GrafanaAccessClient,
    identity: str,
) -> Dict[str, Any]:
    target = str(identity or "").strip()
    if not target:
        raise GrafanaError("User target cannot be empty.")

    exact_matches = []
    for item in client.list_org_users():
        login = str(item.get("login") or "")
        email = str(item.get("email") or "")
        if login == target or email == target:
            exact_matches.append(item)

    if not exact_matches:
        raise GrafanaError("User not found by login or email: %s" % target)
    if len(exact_matches) > 1:
        raise GrafanaError(
            "User identity matched multiple org users: %s" % target
        )
    return dict(exact_matches[0])


def lookup_global_user_by_identity(
    client: GrafanaAccessClient,
    login: Optional[str] = None,
    email: Optional[str] = None,
) -> Dict[str, Any]:
    target_login = str(login or "").strip()
    target_email = str(email or "").strip()
    if not target_login and not target_email:
        raise GrafanaError("User identity lookup requires a login or email.")

    exact_matches = []
    for item in client.iter_global_users(DEFAULT_PAGE_SIZE):
        item_login = str(item.get("login") or "")
        item_email = str(item.get("email") or "")
        if target_login and item_login == target_login:
            exact_matches.append(item)
        elif target_email and item_email == target_email:
            exact_matches.append(item)

    if not exact_matches:
        target = target_login or target_email
        raise GrafanaError("User not found by login or email: %s" % target)
    if len(exact_matches) > 1:
        target = target_login or target_email
        raise GrafanaError(
            "User identity matched multiple global users: %s" % target
        )
    return dict(exact_matches[0])


def lookup_org_user_by_user_id(
    client: GrafanaAccessClient,
    user_id: Any,
) -> Dict[str, Any]:
    target = str(user_id or "").strip()
    if not target:
        raise GrafanaError("User id cannot be empty.")

    exact_matches = []
    for item in client.list_org_users():
        item_id = str(item.get("userId") or item.get("id") or "")
        if item_id == target:
            exact_matches.append(item)

    if not exact_matches:
        raise GrafanaError("Org user not found by id: %s" % target)
    if len(exact_matches) > 1:
        raise GrafanaError("Org user id matched multiple users: %s" % target)
    return dict(exact_matches[0])


def normalize_modified_user(
    base_user: Dict[str, Any],
    args: argparse.Namespace,
) -> Dict[str, Any]:
    return {
        "id": str(base_user.get("id") or ""),
        "login": str(args.set_login or base_user.get("login") or ""),
        "email": str(args.set_email or base_user.get("email") or ""),
        "name": str(args.set_name or base_user.get("name") or ""),
        "orgRole": normalize_org_role(
            args.set_org_role or base_user.get("orgRole") or base_user.get("role")
        ),
        "grafanaAdmin": normalize_bool(
            args.set_grafana_admin
            if args.set_grafana_admin is not None
            else base_user.get("isGrafanaAdmin", base_user.get("isAdmin"))
        ),
        "scope": "global",
        "teams": [],
    }


def normalize_deleted_user(
    base_user: Dict[str, Any],
    scope: str,
) -> Dict[str, Any]:
    if scope == "org":
        return normalize_org_user(base_user)

    return {
        "id": str(base_user.get("id") or ""),
        "login": str(base_user.get("login") or ""),
        "email": str(base_user.get("email") or ""),
        "name": str(base_user.get("name") or ""),
        "orgRole": normalize_org_role(
            base_user.get("orgRole") or base_user.get("role")
        ),
        "grafanaAdmin": normalize_bool(
            base_user.get("isGrafanaAdmin", base_user.get("isAdmin"))
        ),
        "scope": "global",
        "teams": [],
    }


def normalize_identity_list(values: List[str]) -> List[str]:
    identities = []
    seen = set()
    for value in values:
        normalized = str(value or "").strip()
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        identities.append(normalized)
    return identities


def validate_conflicting_identity_sets(
    added: List[str],
    removed: List[str],
    add_flag: str,
    remove_flag: str,
) -> None:
    overlap = sorted(set(added) & set(removed))
    if overlap:
        raise GrafanaError(
            "%s and %s cannot target the same identities: %s"
            % (add_flag, remove_flag, ", ".join(overlap))
        )


def team_member_admin_state(member: Dict[str, Any]) -> Optional[bool]:
    for key in ("isAdmin", "admin"):
        value = member.get(key)
        normalized = normalize_bool(value)
        if normalized is not None:
            return normalized

    for key in ("role", "teamRole", "permissionName"):
        value = str(member.get(key) or "").strip().lower()
        if value in {"member", "viewer"}:
            return False
        if value in {"admin", "teamadmin", "team-admin", "administrator"}:
            return True

    permission = member.get("permission")
    try:
        if permission is not None and int(permission) == 4:
            return True
        if permission is not None and int(permission) == 0:
            return False
    except (TypeError, ValueError):
        pass
    return None


def extract_member_identity(member: Dict[str, Any]) -> str:
    email = str(member.get("email") or "").strip()
    if email:
        return email
    return str(member.get("login") or "").strip()


def format_user_summary_line(user: Dict[str, Any]) -> str:
    parts = [
        "id=%s" % (user.get("id") or ""),
        "login=%s" % (user.get("login") or ""),
    ]
    email = user.get("email") or ""
    if email:
        parts.append("email=%s" % email)
    name = user.get("name") or ""
    if name:
        parts.append("name=%s" % name)
    org_role = user.get("orgRole") or ""
    if org_role:
        parts.append("orgRole=%s" % org_role)
    grafana_admin = bool_label(normalize_bool(user.get("grafanaAdmin")))
    if grafana_admin:
        parts.append("grafanaAdmin=%s" % grafana_admin)
    teams = user.get("teams") or []
    if teams:
        parts.append("teams=%s" % ",".join(teams))
    parts.append("scope=%s" % (user.get("scope") or ""))
    return " ".join(parts)


def format_deleted_team_summary_line(team: Dict[str, Any]) -> str:
    parts = [
        "teamId=%s" % (team.get("teamId") or ""),
        "name=%s" % (team.get("name") or ""),
    ]
    email = team.get("email") or ""
    if email:
        parts.append("email=%s" % email)
    message = team.get("message") or ""
    if message:
        parts.append("message=%s" % message)
    return " ".join(parts)


def format_deleted_service_account_summary_line(
    service_account: Dict[str, Any],
) -> str:
    parts = [
        "serviceAccountId=%s" % (service_account.get("serviceAccountId") or ""),
        "name=%s" % (service_account.get("name") or ""),
    ]
    login = service_account.get("login") or ""
    if login:
        parts.append("login=%s" % login)
    role = service_account.get("role") or ""
    if role:
        parts.append("role=%s" % role)
    message = service_account.get("message") or ""
    if message:
        parts.append("message=%s" % message)
    return " ".join(parts)


def format_deleted_service_account_token_summary_line(
    token_payload: Dict[str, Any],
) -> str:
    parts = [
        "serviceAccountId=%s" % (token_payload.get("serviceAccountId") or ""),
        "serviceAccountName=%s" % (token_payload.get("serviceAccountName") or ""),
        "tokenId=%s" % (token_payload.get("tokenId") or ""),
        "tokenName=%s" % (token_payload.get("tokenName") or ""),
    ]
    message = token_payload.get("message") or ""
    if message:
        parts.append("message=%s" % message)
    return " ".join(parts)


def list_users_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    users = build_user_rows(client, args)

    if args.csv:
        render_user_csv(users)
        return 0
    if args.json:
        print(render_user_json(users))
        return 0
    if args.table:
        for line in render_user_table(users):
            print(line)
    else:
        for user in users:
            print(format_user_summary_line(user))

    print("")
    print(
        "Listed %s user(s) from %s scope at %s"
        % (len(users), args.scope, args.url)
    )
    return 0


def list_service_accounts_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    raw_service_accounts = client.list_service_accounts(
        query=args.query,
        page=args.page,
        per_page=args.per_page,
    )
    service_accounts = []
    for item in raw_service_accounts:
        normalized = normalize_service_account(item)
        if service_account_matches_query(normalized, args.query):
            service_accounts.append(normalized)

    if args.csv:
        render_service_account_csv(service_accounts)
        return 0
    if args.json:
        print(render_service_account_json(service_accounts))
        return 0
    if args.table:
        for line in render_service_account_table(service_accounts):
            print(line)
    else:
        for service_account in service_accounts:
            print(format_service_account_summary_line(service_account))

    print("")
    print(
        "Listed %s service account(s) at %s"
        % (len(service_accounts), args.url)
    )
    return 0


def list_teams_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    teams = build_team_rows(client, args)

    if args.csv:
        render_team_csv(teams)
        return 0
    if args.json:
        print(render_team_json(teams))
        return 0
    if args.table:
        for line in render_team_table(teams):
            print(line)
    else:
        for team in teams:
            print(format_team_summary_line(team))

    print("")
    print("Listed %s team(s) at %s" % (len(teams), args.url))
    return 0


def add_service_account_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    payload = {
        "name": args.name,
        "role": service_account_role_to_api(args.role),
        "isDisabled": args.disabled == "true",
    }
    service_account = normalize_service_account(
        client.create_service_account(payload)
    )
    if args.json:
        print(
            json.dumps(
                serialize_service_account_row(service_account),
                indent=2,
                ensure_ascii=False,
            )
        )
    else:
        print(
            "Created service-account %s -> id=%s role=%s disabled=%s"
            % (
                service_account.get("name") or "",
                service_account.get("id") or "",
                service_account.get("role") or "",
                bool_label(normalize_bool(service_account.get("disabled"))),
            )
        )
    return 0


def add_user_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    payload = {
        "name": args.name,
        "email": args.email,
        "login": args.login,
        "password": args.new_user_password,
    }
    if args.org_id is not None:
        payload["OrgId"] = args.org_id

    created_payload = client.create_user(payload)
    user_id = created_payload.get("id")
    if not user_id:
        raise GrafanaError("Grafana user create response did not include an id.")

    if args.org_role:
        client.update_user_org_role(user_id, args.org_role)
    if args.grafana_admin is not None:
        client.update_user_permissions(user_id, args.grafana_admin == "true")

    created_user = normalize_created_user(user_id, args)
    if args.json:
        print(
            json.dumps(
                serialize_user_row(created_user),
                indent=2,
                ensure_ascii=False,
            )
        )
    else:
        print(
            "Created user %s -> id=%s orgRole=%s grafanaAdmin=%s"
            % (
                created_user.get("login") or "",
                created_user.get("id") or "",
                created_user.get("orgRole") or "",
                bool_label(normalize_bool(created_user.get("grafanaAdmin"))),
            )
        )
    return 0


def modify_user_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_user_modify_args(args)

    if args.user_id:
        base_user = client.get_user(args.user_id)
    else:
        base_user = lookup_global_user_by_identity(
            client,
            login=args.login,
            email=args.email,
        )

    user_id = base_user.get("id") or args.user_id
    if not user_id:
        raise GrafanaError("User lookup did not return an id.")

    profile_payload = {}
    if args.set_login is not None:
        profile_payload["login"] = args.set_login
    if args.set_email is not None:
        profile_payload["email"] = args.set_email
    if args.set_name is not None:
        profile_payload["name"] = args.set_name
    if profile_payload:
        client.update_user(user_id, profile_payload)
    if args.set_password is not None:
        client.update_user_password(user_id, args.set_password)
    if args.set_org_role is not None:
        client.update_user_org_role(user_id, args.set_org_role)
    if args.set_grafana_admin is not None:
        client.update_user_permissions(user_id, args.set_grafana_admin == "true")

    modified_user = normalize_modified_user(base_user, args)
    if args.json:
        print(
            json.dumps(
                serialize_user_row(modified_user),
                indent=2,
                ensure_ascii=False,
            )
        )
    else:
        print(
            "Modified user %s -> id=%s orgRole=%s grafanaAdmin=%s"
            % (
                modified_user.get("login") or "",
                modified_user.get("id") or "",
                modified_user.get("orgRole") or "",
                bool_label(normalize_bool(modified_user.get("grafanaAdmin"))),
            )
        )
    return 0


def delete_user_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_user_delete_args(args)

    if args.scope == "org":
        if args.user_id:
            base_user = lookup_org_user_by_user_id(client, args.user_id)
        else:
            base_user = lookup_org_user_by_identity(
                client,
                args.login or args.email,
            )
        user_id = base_user.get("userId") or base_user.get("id")
        if not user_id:
            raise GrafanaError("Org user lookup did not return an id.")
        client.delete_org_user(user_id)
    else:
        if args.user_id:
            base_user = client.get_user(args.user_id)
        else:
            base_user = lookup_global_user_by_identity(
                client,
                login=args.login,
                email=args.email,
            )
        user_id = base_user.get("id") or args.user_id
        if not user_id:
            raise GrafanaError("User lookup did not return an id.")
        client.delete_global_user(user_id)

    deleted_user = normalize_deleted_user(base_user, args.scope)
    if args.json:
        print(
            json.dumps(
                serialize_user_row(deleted_user),
                indent=2,
                ensure_ascii=False,
            )
        )
    else:
        print(
            "Deleted user %s -> id=%s scope=%s"
            % (
                deleted_user.get("login") or "",
                deleted_user.get("id") or "",
                deleted_user.get("scope") or "",
            )
        )
    return 0


def modify_team_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_team_modify_args(args)

    if args.team_id:
        team_payload = client.get_team(args.team_id)
    else:
        team_payload = lookup_team_by_name(client, args.name)

    team_id = str(team_payload.get("id") or args.team_id or "")
    if not team_id:
        raise GrafanaError("Team lookup did not return an id.")
    team_name = str(team_payload.get("name") or args.name or "")

    payload = apply_team_membership_changes(
        client,
        team_id,
        team_name,
        add_member=getattr(args, "add_member", []),
        remove_member=getattr(args, "remove_member", []),
        add_admin=getattr(args, "add_admin", []),
        remove_admin=getattr(args, "remove_admin", []),
        fetch_existing_members=True,
    )
    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        print(format_team_modify_summary_line(payload))
    return 0


def apply_team_membership_changes(
    client: GrafanaAccessClient,
    team_id: str,
    team_name: str,
    add_member: List[str],
    remove_member: List[str],
    add_admin: List[str],
    remove_admin: List[str],
    fetch_existing_members: bool,
) -> Dict[str, Any]:
    add_member_targets = normalize_identity_list(add_member)
    remove_member_targets = normalize_identity_list(remove_member)
    add_admin_targets = normalize_identity_list(add_admin)
    remove_admin_targets = normalize_identity_list(remove_admin)

    validate_conflicting_identity_sets(
        add_member_targets,
        remove_member_targets,
        "--add-member",
        "--remove-member",
    )
    validate_conflicting_identity_sets(
        add_admin_targets,
        remove_admin_targets,
        "--add-admin",
        "--remove-admin",
    )

    raw_members = []
    if fetch_existing_members:
        raw_members = client.list_team_members(team_id)
    members_by_identity = {}
    member_user_ids = {}
    admin_identities = set()
    saw_admin_metadata = False
    for member in raw_members:
        identity = extract_member_identity(member)
        if not identity:
            continue
        members_by_identity[identity] = dict(member)
        user_id = member.get("userId") or member.get("id")
        if user_id is not None:
            member_user_ids[identity] = str(user_id)
        admin_state = team_member_admin_state(member)
        if admin_state is not None:
            saw_admin_metadata = True
            if admin_state:
                admin_identities.add(identity)

    added_members = []
    removed_members = []
    for target in add_member_targets:
        user = lookup_org_user_by_identity(client, target)
        identity = str(user.get("email") or user.get("login") or "").strip()
        if not identity:
            raise GrafanaError(
                "Resolved user did not include a login or email for %s." % target
            )
        if identity in members_by_identity:
            continue
        user_id = user.get("userId") or user.get("id")
        if user_id is None:
            raise GrafanaError(
                "Resolved user did not include an id for %s." % target
            )
        client.add_team_member(team_id, user_id)
        members_by_identity[identity] = dict(user)
        member_user_ids[identity] = str(user_id)
        added_members.append(identity)

    for target in remove_member_targets:
        user = lookup_org_user_by_identity(client, target)
        identity = str(user.get("email") or user.get("login") or "").strip()
        if not identity:
            raise GrafanaError(
                "Resolved user did not include a login or email for %s." % target
            )
        user_id = member_user_ids.get(identity)
        if not user_id:
            continue
        client.remove_team_member(team_id, user_id)
        members_by_identity.pop(identity, None)
        member_user_ids.pop(identity, None)
        admin_identities.discard(identity)
        removed_members.append(identity)

    added_admins = []
    removed_admins = []
    if add_admin_targets or remove_admin_targets:
        if raw_members and not saw_admin_metadata:
            raise GrafanaError(
                "Team modify admin operations require Grafana team member responses "
                "to include admin state metadata."
            )

        for target in add_admin_targets:
            user = lookup_org_user_by_identity(client, target)
            identity = str(user.get("email") or user.get("login") or "").strip()
            if not identity:
                raise GrafanaError(
                    "Resolved user did not include a login or email for %s." % target
                )
            if identity not in members_by_identity:
                members_by_identity[identity] = dict(user)
            if identity not in admin_identities:
                admin_identities.add(identity)
                added_admins.append(identity)

        for target in remove_admin_targets:
            user = lookup_org_user_by_identity(client, target)
            identity = str(user.get("email") or user.get("login") or "").strip()
            if identity in admin_identities:
                admin_identities.discard(identity)
                removed_admins.append(identity)

        regular_members = sorted(
            identity
            for identity in members_by_identity
            if identity not in admin_identities
        )
        admin_members = sorted(admin_identities)
        client.update_team_members(
            team_id,
            {"members": regular_members, "admins": admin_members},
        )

    return {
        "teamId": team_id,
        "name": team_name,
        "addedMembers": added_members,
        "removedMembers": removed_members,
        "addedAdmins": added_admins,
        "removedAdmins": removed_admins,
    }


def add_team_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    payload = {"name": args.name}
    if args.email:
        payload["email"] = args.email

    created_payload = client.create_team(payload)
    team_id = created_payload.get("teamId") or created_payload.get("id")
    if not team_id:
        raise GrafanaError("Grafana team create response did not include a team id.")

    team_payload = client.get_team(team_id)
    team_name = str(team_payload.get("name") or args.name or "")
    team_email = str(team_payload.get("email") or args.email or "")

    membership_payload = apply_team_membership_changes(
        client,
        str(team_id),
        team_name,
        add_member=getattr(args, "member", []),
        remove_member=[],
        add_admin=getattr(args, "admin", []),
        remove_admin=[],
        fetch_existing_members=False,
    )
    membership_payload["email"] = team_email

    if args.json:
        print(json.dumps(membership_payload, indent=2, ensure_ascii=False))
    else:
        print(format_team_add_summary_line(membership_payload))
    return 0


def add_service_account_token_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    service_account_id = args.service_account_id
    if not service_account_id:
        service_account_id = lookup_service_account_id_by_name(client, args.name)

    payload = {"name": args.token_name}
    if args.seconds_to_live is not None:
        payload["secondsToLive"] = args.seconds_to_live

    token_payload = client.create_service_account_token(
        service_account_id,
        payload,
    )
    token_payload["serviceAccountId"] = str(service_account_id)

    if args.json:
        print(render_service_account_token_json(token_payload))
    else:
        print(
            "Created service-account token %s -> serviceAccountId=%s"
            % (args.token_name, service_account_id)
        )
    return 0


def delete_team_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_destructive_confirmed(args, "Team delete")
    team_id = resolve_team_id(client, args.team_id, args.name)
    team_payload = client.get_team(team_id)
    delete_payload = client.delete_team(team_id)
    result = {
        "teamId": str(team_payload.get("id") or team_id),
        "name": str(team_payload.get("name") or args.name or ""),
        "email": str(team_payload.get("email") or ""),
        "message": str(delete_payload.get("message") or "Team deleted."),
    }

    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(format_deleted_team_summary_line(result))
    return 0


def delete_service_account_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_destructive_confirmed(args, "Service-account delete")
    service_account_id = resolve_service_account_id(
        client,
        args.service_account_id,
        args.name,
    )
    service_account = normalize_service_account(
        client.get_service_account(service_account_id)
    )
    delete_payload = client.delete_service_account(service_account_id)
    result = serialize_service_account_row(service_account)
    result["serviceAccountId"] = str(
        service_account.get("id") or service_account_id
    )
    result["message"] = str(
        delete_payload.get("message") or "Service account deleted."
    )

    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(format_deleted_service_account_summary_line(result))
    return 0


def delete_service_account_token_with_client(
    args: argparse.Namespace,
    client: GrafanaAccessClient,
) -> int:
    validate_destructive_confirmed(args, "Service-account token delete")
    service_account_id = resolve_service_account_id(
        client,
        args.service_account_id,
        args.name,
    )
    service_account = client.get_service_account(service_account_id)
    token_items = client.list_service_account_tokens(service_account_id)
    token_record = resolve_service_account_token_record(
        token_items,
        token_id=args.token_id,
        token_name=args.token_name,
    )
    token_id = str(token_record.get("id") or "")
    if not token_id:
        raise GrafanaError("Service-account token lookup did not return an id.")

    delete_payload = client.delete_service_account_token(
        service_account_id,
        token_id,
    )
    result = {
        "serviceAccountId": str(service_account.get("id") or service_account_id),
        "serviceAccountName": str(service_account.get("name") or ""),
        "tokenId": token_id,
        "tokenName": str(token_record.get("name") or ""),
        "message": str(
            delete_payload.get("message") or "Service-account token deleted."
        ),
    }

    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(format_deleted_service_account_token_summary_line(result))
    return 0


def run(args: argparse.Namespace) -> int:
    headers, auth_mode = build_request_headers(args)
    client = GrafanaAccessClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )
    if args.resource == "user" and args.command == "list":
        validate_user_list_auth(args, auth_mode)
        return list_users_with_client(args, client)
    if args.resource == "user" and args.command == "add":
        validate_user_add_auth(auth_mode)
        return add_user_with_client(args, client)
    if args.resource == "user" and args.command == "modify":
        validate_user_modify_auth(auth_mode)
        return modify_user_with_client(args, client)
    if args.resource == "user" and args.command == "delete":
        validate_user_delete_auth(args, auth_mode)
        return delete_user_with_client(args, client)
    if args.resource == "team" and args.command == "list":
        return list_teams_with_client(args, client)
    if args.resource == "team" and args.command == "add":
        return add_team_with_client(args, client)
    if args.resource == "team" and args.command == "modify":
        return modify_team_with_client(args, client)
    if args.resource == "team" and args.command == "delete":
        validate_team_delete_auth(auth_mode)
        return delete_team_with_client(args, client)
    if args.resource == "service-account" and args.command == "list":
        return list_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "add":
        return add_service_account_with_client(args, client)
    if args.resource == "service-account" and args.command == "delete":
        validate_service_account_delete_auth(auth_mode)
        return delete_service_account_with_client(args, client)
    if (
        args.resource == "service-account"
        and args.command == "token"
        and args.token_command == "add"
    ):
        return add_service_account_token_with_client(args, client)
    if (
        args.resource == "service-account"
        and args.command == "token"
        and args.token_command == "delete"
    ):
        validate_service_account_token_delete_auth(auth_mode)
        return delete_service_account_token_with_client(args, client)
    raise GrafanaError("Unsupported command.")


def main(argv: Optional[List[str]] = None) -> int:
    try:
        args = parse_args(argv)
        return run(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
