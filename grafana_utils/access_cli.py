#!/usr/bin/env python3
"""List Grafana users, teams, and service accounts through Grafana APIs.

Initial scope:
- `grafana-access-utils user list`
- `grafana-access-utils user add`
- `grafana-access-utils user modify`
- `grafana-access-utils user delete`
- `grafana-access-utils team list`
- `grafana-access-utils team modify`
- `grafana-access-utils service-account list`
- `grafana-access-utils service-account add`
- `grafana-access-utils service-account token add`

Design notes:
- org-scoped listing can use token auth or Basic auth
- global user listing requires Basic auth because Grafana's global user API is
  server-admin oriented
- `--with-teams` also requires Basic auth because it uses per-user team lookup
- output modes intentionally mirror the existing CLI family: compact text,
  table, CSV, and JSON
"""

import argparse
import base64
import csv
import getpass
import json
import sys
from typing import Any, Dict, List, Optional, Tuple
from urllib import parse

from .http_transport import (
    HttpTransportApiError,
    HttpTransportError,
    JsonHttpTransport,
    build_json_http_transport,
)


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_PAGE_SIZE = 100
DEFAULT_SCOPE = "org"
DEFAULT_SERVICE_ACCOUNT_ROLE = "Viewer"
SCOPE_CHOICES = ("org", "global")
OUTPUT_FIELDS = [
    "id",
    "login",
    "email",
    "name",
    "orgRole",
    "grafanaAdmin",
    "scope",
    "teams",
]
TEAM_OUTPUT_FIELDS = [
    "id",
    "name",
    "email",
    "memberCount",
    "members",
]
SERVICE_ACCOUNT_OUTPUT_FIELDS = [
    "id",
    "name",
    "login",
    "role",
    "disabled",
    "tokens",
    "orgId",
]
SERVICE_ACCOUNT_TOKEN_OUTPUT_FIELDS = [
    "serviceAccountId",
    "name",
    "secondsToLive",
    "key",
]


class GrafanaError(RuntimeError):
    """Raised when Grafana returns an unexpected response."""


class GrafanaApiError(GrafanaError):
    """Raised when Grafana returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__(f"Grafana API error {status_code} for {url}: {body}")


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


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
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

    service_account_parser = subparsers.add_parser(
        "service-account",
        help="List and create Grafana service accounts.",
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
    argv = list(sys.argv[1:] if argv is None else argv)

    if not argv:
        parser.print_help()
        raise SystemExit(0)

    if argv == ["user"]:
        parser._subparsers._group_actions[0].choices["user"].print_help()
        raise SystemExit(0)

    if argv == ["team"]:
        parser._subparsers._group_actions[0].choices["team"].print_help()
        raise SystemExit(0)

    if argv == ["service-account"]:
        parser._subparsers._group_actions[0].choices["service-account"].print_help()
        raise SystemExit(0)

    if argv == ["service-account", "token"]:
        parser._subparsers._group_actions[0].choices["service-account"]._subparsers._group_actions[0].choices["token"].print_help()
        raise SystemExit(0)

    return parser.parse_args(argv)


def env_value(name: str) -> Optional[str]:
    import os

    value = os.environ.get(name)
    return value if value else None


def resolve_auth(args: argparse.Namespace) -> Tuple[Dict[str, str], str]:
    cli_token = getattr(args, "api_token", None)
    cli_username = getattr(args, "username", None)
    cli_password = getattr(args, "password", None)
    prompt_password = bool(getattr(args, "prompt_password", False))
    if cli_username is None:
        cli_username = getattr(args, "auth_username", None)
    if cli_password is None:
        cli_password = getattr(args, "auth_password", None)

    if cli_token and (cli_username or cli_password or prompt_password):
        raise GrafanaError(
            "Choose either token auth (--token / --api-token) or Basic auth "
            "(--basic-user / --username with --basic-password / --password / --prompt-password), not both."
        )
    if prompt_password and cli_password:
        raise GrafanaError(
            "Choose either --basic-password / --password or --prompt-password, not both."
        )
    if cli_username and not cli_password:
        if not prompt_password:
            raise GrafanaError(
                "Basic auth requires both --basic-user / --username and "
                "--basic-password / --password or --prompt-password."
            )
    if cli_password and not cli_username:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password or --prompt-password."
        )
    if prompt_password and not cli_username:
        raise GrafanaError(
            "--prompt-password requires --basic-user / --username."
        )

    if cli_token:
        headers = {"Authorization": "Bearer %s" % cli_token}
        return headers, "token"

    if prompt_password and cli_username:
        cli_password = getpass.getpass("Grafana Basic auth password: ")

    if cli_username and cli_password:
        encoded = base64.b64encode(
            ("%s:%s" % (cli_username, cli_password)).encode("utf-8")
        ).decode("ascii")
        headers = {"Authorization": "Basic %s" % encoded}
        return headers, "basic"

    token = env_value("GRAFANA_API_TOKEN")
    if token:
        headers = {"Authorization": "Bearer %s" % token}
        return headers, "token"

    username = env_value("GRAFANA_USERNAME")
    password = env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(
            ("%s:%s" % (username, password)).encode("utf-8")
        ).decode("ascii")
        headers = {"Authorization": "Basic %s" % encoded}
        return headers, "basic"
    if username or password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password or --prompt-password."
        )

    raise GrafanaError(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN "
        "or --basic-user and --basic-password / --prompt-password / "
        "GRAFANA_USERNAME and GRAFANA_PASSWORD."
    )


def build_request_headers(args: argparse.Namespace) -> Tuple[Dict[str, str], str]:
    headers, auth_mode = resolve_auth(args)
    org_id = getattr(args, "org_id", None)
    if org_id:
        headers["X-Grafana-Org-Id"] = str(org_id)
    return headers, auth_mode


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


class GrafanaAccessClient:
    """Minimal HTTP wrapper around the Grafana user APIs used by this CLI."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
        timeout: int,
        verify_ssl: bool,
        transport: Optional[JsonHttpTransport] = None,
    ) -> None:
        self.transport = transport or build_json_http_transport(
            base_url=base_url,
            headers={"Accept": "application/json", **headers},
            timeout=timeout,
            verify_ssl=verify_ssl,
        )

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        try:
            return self.transport.request_json(
                path=path,
                params=params,
                method=method,
                payload=payload,
            )
        except HttpTransportApiError as exc:
            raise GrafanaApiError(exc.status_code, exc.url, exc.body) from exc
        except HttpTransportError as exc:
            raise GrafanaError(str(exc)) from exc

    def list_org_users(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/org/users")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected org user list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def iter_global_users(self, page_size: int) -> List[Dict[str, Any]]:
        users = []
        page = 1
        while True:
            batch = self.request_json(
                "/api/users",
                params={"page": page, "perpage": page_size},
            )
            if not isinstance(batch, list):
                raise GrafanaError("Unexpected global user list response from Grafana.")
            if not batch:
                break
            users.extend(item for item in batch if isinstance(item, dict))
            if len(batch) < page_size:
                break
            page += 1
        return users

    def list_user_teams(self, user_id: Any) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/users/%s/teams" % parse.quote(str(user_id), safe="")
        )
        if not isinstance(data, list):
            raise GrafanaError(
                "Unexpected team list response for Grafana user %s." % user_id
            )
        return [item for item in data if isinstance(item, dict)]

    def get_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/users/%s" % parse.quote(str(user_id), safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected user lookup response for Grafana user %s." % user_id
            )
        return data

    def create_user(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected user create response from Grafana.")
        return data

    def update_user(self, user_id: Any, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/users/%s" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected user update response for Grafana user %s." % user_id
            )
        return data

    def update_user_password(self, user_id: Any, password: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s/password" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload={"password": password},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected password update response for Grafana user %s."
                % user_id
            )
        return data

    def update_user_org_role(self, user_id: Any, role: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/org/users/%s" % parse.quote(str(user_id), safe=""),
            method="PATCH",
            payload={"role": role},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected org-role update response for Grafana user %s." % user_id
            )
        return data

    def update_user_permissions(
        self,
        user_id: Any,
        is_grafana_admin: bool,
    ) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s/permissions" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload={"isGrafanaAdmin": is_grafana_admin},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected permission update response for Grafana user %s."
                % user_id
            )
        return data

    def delete_global_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s" % parse.quote(str(user_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected global delete response for Grafana user %s."
                % user_id
            )
        return data

    def delete_org_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/org/users/%s" % parse.quote(str(user_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected org delete response for Grafana user %s." % user_id
            )
        return data

    def list_service_accounts(
        self,
        query: Optional[str],
        page: int,
        per_page: int,
    ) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/serviceaccounts/search",
            params={
                "query": query or "",
                "page": page,
                "perpage": per_page,
            },
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account list response from Grafana."
            )
        items = data.get("serviceAccounts", [])
        if not isinstance(items, list):
            raise GrafanaError(
                "Unexpected service-account list response from Grafana."
            )
        return [item for item in items if isinstance(item, dict)]

    def list_teams(
        self,
        query: Optional[str],
        page: int,
        per_page: int,
    ) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/teams/search",
            params={
                "query": query or "",
                "page": page,
                "perpage": per_page,
            },
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected team list response from Grafana.")
        items = data.get("teams", [])
        if not isinstance(items, list):
            raise GrafanaError("Unexpected team list response from Grafana.")
        return [item for item in items if isinstance(item, dict)]

    def iter_teams(
        self,
        query: Optional[str],
        page_size: int,
    ) -> List[Dict[str, Any]]:
        teams = []
        page = 1
        while True:
            batch = self.list_teams(
                query=query,
                page=page,
                per_page=page_size,
            )
            if not batch:
                break
            teams.extend(batch)
            if len(batch) < page_size:
                break
            page += 1
        return teams

    def list_team_members(self, team_id: Any) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe="")
        )
        if not isinstance(data, list):
            raise GrafanaError(
                "Unexpected member list response for Grafana team %s." % team_id
            )
        return [item for item in data if isinstance(item, dict)]

    def get_team(self, team_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s" % parse.quote(str(team_id), safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected team lookup response for Grafana team %s." % team_id
            )
        return data

    def create_team(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected team create response from Grafana.")
        return data

    def add_team_member(self, team_id: Any, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe=""),
            method="POST",
            payload={"userId": user_id},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected add-member response for Grafana team %s." % team_id
            )
        return data

    def remove_team_member(self, team_id: Any, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members/%s"
            % (
                parse.quote(str(team_id), safe=""),
                parse.quote(str(user_id), safe=""),
            ),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected remove-member response for Grafana team %s." % team_id
            )
        return data

    def update_team_members(self, team_id: Any, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected team member update response for Grafana team %s."
                % team_id
            )
        return data

    def create_service_account(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account create response from Grafana."
            )
        return data

    def create_service_account_token(
        self,
        service_account_id: Any,
        payload: Dict[str, Any],
    ) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts/%s/tokens"
            % parse.quote(str(service_account_id), safe=""),
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account token create response from Grafana."
            )
        return data


def normalize_org_role(value: Any) -> str:
    text = str(value or "").strip()
    if not text:
        return ""
    lowered = text.lower()
    if lowered == "none":
        return "None"
    if lowered == "nobasicrole":
        return "None"
    return lowered[:1].upper() + lowered[1:]


def service_account_role_to_api(value: str) -> str:
    normalized = normalize_org_role(value)
    if normalized == "None":
        return "NoBasicRole"
    return normalized


def normalize_bool(value: Any) -> Optional[bool]:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        lowered = value.strip().lower()
        if lowered in {"true", "1", "yes"}:
            return True
        if lowered in {"false", "0", "no"}:
            return False
    return None


def bool_label(value: Optional[bool]) -> str:
    if value is True:
        return "true"
    if value is False:
        return "false"
    return ""


def normalize_service_account(item: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": str(item.get("id") or ""),
        "name": str(item.get("name") or ""),
        "login": str(item.get("login") or ""),
        "role": normalize_org_role(item.get("role")),
        "disabled": normalize_bool(item.get("isDisabled")),
        "tokens": str(item.get("tokens") or 0),
        "orgId": str(item.get("orgId") or ""),
    }


def normalize_team(item: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": str(item.get("id") or ""),
        "name": str(item.get("name") or ""),
        "email": str(item.get("email") or ""),
        "memberCount": str(item.get("memberCount") or 0),
        "members": [],
    }


def normalize_org_user(item: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": item.get("userId") or item.get("id") or "",
        "login": str(item.get("login") or ""),
        "email": str(item.get("email") or ""),
        "name": str(item.get("name") or ""),
        "orgRole": normalize_org_role(item.get("role")),
        "grafanaAdmin": normalize_bool(item.get("isGrafanaAdmin")),
        "scope": "org",
        "teams": [],
    }


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


def normalize_global_user(item: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": item.get("id") or "",
        "login": str(item.get("login") or ""),
        "email": str(item.get("email") or ""),
        "name": str(item.get("name") or ""),
        "orgRole": normalize_org_role(item.get("orgRole") or item.get("role")),
        "grafanaAdmin": normalize_bool(
            item.get("isGrafanaAdmin", item.get("isAdmin"))
        ),
        "scope": "global",
        "teams": [],
    }


def user_matches_filters(user: Dict[str, Any], args: argparse.Namespace) -> bool:
    query = (args.query or "").strip().lower()
    if query:
        haystacks = [
            str(user.get("login") or "").lower(),
            str(user.get("email") or "").lower(),
            str(user.get("name") or "").lower(),
        ]
        if not any(query in haystack for haystack in haystacks):
            return False

    if args.login and str(user.get("login") or "") != args.login:
        return False
    if args.email and str(user.get("email") or "") != args.email:
        return False
    if args.org_role and normalize_org_role(user.get("orgRole")) != args.org_role:
        return False
    if args.grafana_admin is not None:
        expected = args.grafana_admin == "true"
        if normalize_bool(user.get("grafanaAdmin")) is not expected:
            return False
    return True


def paginate_users(
    users: List[Dict[str, Any]],
    page: int,
    per_page: int,
) -> List[Dict[str, Any]]:
    start = (page - 1) * per_page
    end = start + per_page
    return users[start:end]


def attach_team_memberships(
    users: List[Dict[str, Any]],
    client: GrafanaAccessClient,
) -> None:
    for user in users:
        user_id = user.get("id")
        if not user_id:
            continue
        teams = client.list_user_teams(user_id)
        team_names = []
        for team in teams:
            name = str(team.get("name") or "").strip()
            if name:
                team_names.append(name)
        user["teams"] = sorted(team_names)


def build_user_rows(
    client: GrafanaAccessClient,
    args: argparse.Namespace,
) -> List[Dict[str, Any]]:
    if args.scope == "global":
        raw_users = client.iter_global_users(max(args.per_page, DEFAULT_PAGE_SIZE))
        users = [normalize_global_user(item) for item in raw_users]
    else:
        raw_users = client.list_org_users()
        users = [normalize_org_user(item) for item in raw_users]

    users = [user for user in users if user_matches_filters(user, args)]
    users.sort(key=lambda item: (str(item.get("login") or ""), str(item.get("email") or "")))
    if args.with_teams:
        attach_team_memberships(users, client)
    return paginate_users(users, args.page, args.per_page)


def serialize_user_row(user: Dict[str, Any]) -> Dict[str, Any]:
    row = {}
    for field in OUTPUT_FIELDS:
        value = user.get(field)
        if field == "grafanaAdmin":
            row[field] = bool_label(normalize_bool(value))
        elif field == "teams":
            row[field] = list(value or [])
        else:
            row[field] = str(value or "")
    return row


def render_user_json(users: List[Dict[str, Any]]) -> str:
    payload = [serialize_user_row(user) for user in users]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_user_csv(users: List[Dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=OUTPUT_FIELDS)
    writer.writeheader()
    for user in users:
        row = serialize_user_row(user)
        row["teams"] = ",".join(row["teams"])
        writer.writerow(row)


def render_user_table(users: List[Dict[str, Any]]) -> List[str]:
    headers = {
        "id": "ID",
        "login": "Login",
        "email": "Email",
        "name": "Name",
        "orgRole": "Org Role",
        "grafanaAdmin": "Grafana Admin",
        "scope": "Scope",
        "teams": "Teams",
    }
    rows = []
    for user in users:
        serialized = serialize_user_row(user)
        serialized["teams"] = ", ".join(serialized["teams"])
        rows.append(serialized)

    widths = {}
    for field in OUTPUT_FIELDS:
        widths[field] = len(headers[field])
        for row in rows:
            widths[field] = max(widths[field], len(str(row.get(field) or "")))

    def build_row(values: Dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field]) for field in OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join("-" * widths[field] for field in OUTPUT_FIELDS)
    return [header_row, separator_row] + [build_row(row) for row in rows]


def service_account_matches_query(
    service_account: Dict[str, Any],
    query: Optional[str],
) -> bool:
    text = str(query or "").strip().lower()
    if not text:
        return True
    haystacks = [
        str(service_account.get("name") or "").lower(),
        str(service_account.get("login") or "").lower(),
    ]
    return any(text in haystack for haystack in haystacks)


def team_matches_filters(team: Dict[str, Any], args: argparse.Namespace) -> bool:
    query = str(args.query or "").strip().lower()
    if query:
        haystacks = [
            str(team.get("name") or "").lower(),
            str(team.get("email") or "").lower(),
        ]
        if not any(query in haystack for haystack in haystacks):
            return False
    if args.name and str(team.get("name") or "") != args.name:
        return False
    return True


def paginate_teams(
    teams: List[Dict[str, Any]],
    page: int,
    per_page: int,
) -> List[Dict[str, Any]]:
    start = (page - 1) * per_page
    end = start + per_page
    return teams[start:end]


def attach_team_members(
    teams: List[Dict[str, Any]],
    client: GrafanaAccessClient,
) -> None:
    for team in teams:
        team_id = team.get("id")
        if not team_id:
            continue
        raw_members = client.list_team_members(team_id)
        member_names = []
        for member in raw_members:
            login = str(member.get("login") or "").strip()
            if login:
                member_names.append(login)
        team["members"] = sorted(member_names)


def serialize_service_account_row(
    service_account: Dict[str, Any],
) -> Dict[str, Any]:
    row = {}
    for field in SERVICE_ACCOUNT_OUTPUT_FIELDS:
        value = service_account.get(field)
        if field == "disabled":
            row[field] = bool_label(normalize_bool(value))
        else:
            row[field] = str(value or "")
    return row


def build_team_rows(
    client: GrafanaAccessClient,
    args: argparse.Namespace,
) -> List[Dict[str, Any]]:
    raw_teams = client.iter_teams(
        query=args.query,
        page_size=max(args.per_page, DEFAULT_PAGE_SIZE),
    )
    teams = [normalize_team(item) for item in raw_teams]
    teams = [team for team in teams if team_matches_filters(team, args)]
    teams.sort(key=lambda item: (str(item.get("name") or ""), str(item.get("email") or "")))
    if args.with_members:
        attach_team_members(teams, client)
    return paginate_teams(teams, args.page, args.per_page)


def serialize_team_row(team: Dict[str, Any]) -> Dict[str, Any]:
    row = {}
    for field in TEAM_OUTPUT_FIELDS:
        value = team.get(field)
        if field == "members":
            row[field] = list(value or [])
        else:
            row[field] = str(value or "")
    return row


def render_team_json(teams: List[Dict[str, Any]]) -> str:
    payload = [serialize_team_row(team) for team in teams]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_team_csv(teams: List[Dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=TEAM_OUTPUT_FIELDS)
    writer.writeheader()
    for team in teams:
        row = serialize_team_row(team)
        row["members"] = ",".join(row["members"])
        writer.writerow(row)


def render_team_table(teams: List[Dict[str, Any]]) -> List[str]:
    headers = {
        "id": "ID",
        "name": "Name",
        "email": "Email",
        "memberCount": "Members",
        "members": "Member Logins",
    }
    rows = []
    for team in teams:
        serialized = serialize_team_row(team)
        serialized["members"] = ", ".join(serialized["members"])
        rows.append(serialized)

    widths = {}
    for field in TEAM_OUTPUT_FIELDS:
        widths[field] = len(headers[field])
        for row in rows:
            widths[field] = max(widths[field], len(str(row.get(field) or "")))

    def build_row(values: Dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field])
            for field in TEAM_OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join("-" * widths[field] for field in TEAM_OUTPUT_FIELDS)
    return [header_row, separator_row] + [build_row(row) for row in rows]


def format_team_summary_line(team: Dict[str, Any]) -> str:
    parts = [
        "id=%s" % (team.get("id") or ""),
        "name=%s" % (team.get("name") or ""),
    ]
    email = team.get("email") or ""
    if email:
        parts.append("email=%s" % email)
    parts.append("memberCount=%s" % (team.get("memberCount") or "0"))
    members = team.get("members") or []
    if members:
        parts.append("members=%s" % ",".join(members))
    return " ".join(parts)


def format_team_modify_summary_line(payload: Dict[str, Any]) -> str:
    parts = [
        "teamId=%s" % (payload.get("teamId") or ""),
        "name=%s" % (payload.get("name") or ""),
    ]
    for field in (
        "addedMembers",
        "removedMembers",
        "addedAdmins",
        "removedAdmins",
    ):
        values = payload.get(field) or []
        if values:
            parts.append("%s=%s" % (field, ",".join(values)))
    return " ".join(parts)


def format_team_add_summary_line(payload: Dict[str, Any]) -> str:
    parts = [
        "teamId=%s" % (payload.get("teamId") or ""),
        "name=%s" % (payload.get("name") or ""),
    ]
    email = payload.get("email") or ""
    if email:
        parts.append("email=%s" % email)
    for field in ("addedMembers", "addedAdmins"):
        values = payload.get(field) or []
        if values:
            parts.append("%s=%s" % (field, ",".join(values)))
    return " ".join(parts)


def render_service_account_json(service_accounts: List[Dict[str, Any]]) -> str:
    payload = [
        serialize_service_account_row(service_account)
        for service_account in service_accounts
    ]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_service_account_csv(service_accounts: List[Dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=SERVICE_ACCOUNT_OUTPUT_FIELDS)
    writer.writeheader()
    for service_account in service_accounts:
        writer.writerow(serialize_service_account_row(service_account))


def render_service_account_table(
    service_accounts: List[Dict[str, Any]],
) -> List[str]:
    headers = {
        "id": "ID",
        "name": "Name",
        "login": "Login",
        "role": "Role",
        "disabled": "Disabled",
        "tokens": "Tokens",
        "orgId": "Org ID",
    }
    rows = [
        serialize_service_account_row(service_account)
        for service_account in service_accounts
    ]
    widths = {}
    for field in SERVICE_ACCOUNT_OUTPUT_FIELDS:
        widths[field] = len(headers[field])
        for row in rows:
            widths[field] = max(widths[field], len(str(row.get(field) or "")))

    def build_row(values):
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field])
            for field in SERVICE_ACCOUNT_OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join(
        "-" * widths[field] for field in SERVICE_ACCOUNT_OUTPUT_FIELDS
    )
    return [header_row, separator_row] + [build_row(row) for row in rows]


def format_service_account_summary_line(service_account: Dict[str, Any]) -> str:
    return " ".join(
        [
            "id=%s" % (service_account.get("id") or ""),
            "name=%s" % (service_account.get("name") or ""),
            "login=%s" % (service_account.get("login") or ""),
            "role=%s" % (service_account.get("role") or ""),
            "disabled=%s"
            % bool_label(normalize_bool(service_account.get("disabled"))),
            "tokens=%s" % (service_account.get("tokens") or "0"),
            "orgId=%s" % (service_account.get("orgId") or ""),
        ]
    )


def serialize_service_account_token_row(payload: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "serviceAccountId": str(payload.get("serviceAccountId") or ""),
        "name": str(payload.get("name") or ""),
        "secondsToLive": str(payload.get("secondsToLive") or ""),
        "key": str(payload.get("key") or ""),
    }


def render_service_account_token_json(payload: Dict[str, Any]) -> str:
    return json.dumps(
        serialize_service_account_token_row(payload),
        indent=2,
        ensure_ascii=False,
    )


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
    if args.resource == "service-account" and args.command == "list":
        return list_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "add":
        return add_service_account_with_client(args, client)
    if (
        args.resource == "service-account"
        and args.command == "token"
        and args.token_command == "add"
    ):
        return add_service_account_token_with_client(args, client)
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
