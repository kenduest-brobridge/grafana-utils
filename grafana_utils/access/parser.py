"""Argparse wiring for the Python access-management CLI."""

import argparse
import sys
from typing import List, Optional

from .common import DEFAULT_PAGE_SIZE
from .pending_cli_staging import (
    add_service_account_delete_cli_args,
    add_service_account_token_delete_cli_args,
    add_team_delete_cli_args,
    normalize_group_alias_argv,
)

DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_SCOPE = "org"
DEFAULT_SERVICE_ACCOUNT_ROLE = "Viewer"
DEFAULT_ACCESS_USER_EXPORT_DIR = "access-users"
DEFAULT_ACCESS_TEAM_EXPORT_DIR = "access-teams"
DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR = "access-service-accounts"
ACCESS_USER_EXPORT_FILENAME = "users.json"
ACCESS_TEAM_EXPORT_FILENAME = "teams.json"
ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME = "service-accounts.json"
ACCESS_EXPORT_METADATA_FILENAME = "export-metadata.json"
ACCESS_EXPORT_KIND_USERS = "grafana-utils-access-user-export-index"
ACCESS_EXPORT_KIND_TEAMS = "grafana-utils-access-team-export-index"
ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS = "grafana-utils-access-service-account-export-index"
ACCESS_EXPORT_VERSION = 1
SCOPE_CHOICES = ("org", "global")
LIST_OUTPUT_FORMAT_CHOICES = ("text", "table", "csv", "json")
DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")


def positive_int(value):
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("value must be >= 1")
    return parsed


def bool_choice(value):
    normalized = str(value).strip().lower()
    if normalized not in {"true", "false"}:
        raise argparse.ArgumentTypeError("value must be true or false")
    return normalized


def add_list_output_format_arg(parser):
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


def add_access_export_cli_args(parser, default_export_dir, resource="user"):
    payload_name = access_export_filename(resource)
    parser.add_argument(
        "--export-dir",
        default=default_export_dir,
        help=(
            "Directory to write the exported JSON file. The export creates "
            "%s and %s under the directory."
            % (payload_name, ACCESS_EXPORT_METADATA_FILENAME)
        ),
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help=(
            "Overwrite existing export files instead of failing."
        ),
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview export paths without writing files.",
    )


def add_access_import_cli_args(parser, resource, default_scope=DEFAULT_SCOPE):
    parser.add_argument(
        "--import-dir",
        required=True,
        help=(
            "Import directory that contains %s for %s and %s."
            % (
                access_export_filename(resource),
                resource,
                ACCESS_EXPORT_METADATA_FILENAME,
            )
        ),
    )
    if resource == "user":
        parser.add_argument(
            "--scope",
            choices=SCOPE_CHOICES,
            default=default_scope,
            help=(
                "Import match strategy for users: global or org scope (default: %s)."
                % default_scope
            ),
        )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update matching existing items instead of failing import on duplicates.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview what this import would do without writing to Grafana.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Acknowledge destructive import operations (delete/missing sync).",
    )


def add_access_diff_cli_args(parser, resource, default_scope=DEFAULT_SCOPE):
    parser.add_argument(
        "--diff-dir",
        required=True,
        help=(
            "Diff directory that contains %s and %s." % (
                access_export_filename(resource),
                ACCESS_EXPORT_METADATA_FILENAME,
            )
        ),
    )
    if resource == "user":
        parser.add_argument(
            "--scope",
            choices=SCOPE_CHOICES,
            default=default_scope,
            help=(
                "Match against global or org user listing (default: %s)." % default_scope
            ),
        )


def build_parser(prog=None):
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

    user_export_parser = user_subparsers.add_parser(
        "export",
        help="Export Grafana users to JSON files.",
    )
    add_common_cli_args(
        user_export_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_export_cli_args(
        user_export_parser,
        DEFAULT_ACCESS_USER_EXPORT_DIR,
        resource="user",
    )
    user_export_parser.add_argument(
        "--scope",
        choices=SCOPE_CHOICES,
        default=DEFAULT_SCOPE,
        help="Export org-scoped or global users (default: %s)." % DEFAULT_SCOPE,
    )
    user_export_parser.add_argument(
        "--with-teams",
        action="store_true",
        help="Include team memberships in exported user objects.",
    )

    user_import_parser = user_subparsers.add_parser(
        "import",
        help="Import Grafana users from a JSON export.",
    )
    add_common_cli_args(
        user_import_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(user_import_parser, resource="user", default_scope=DEFAULT_SCOPE)

    user_diff_parser = user_subparsers.add_parser(
        "diff",
        help="Diff Grafana users against a previously exported users.json file.",
    )
    add_common_cli_args(
        user_diff_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(user_diff_parser, resource="user", default_scope=DEFAULT_SCOPE)

    add_parser = user_subparsers.add_parser(
        "add",
        help="Create a Grafana user through the global admin API.",
    )
    add_common_cli_args(
        add_parser,
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

    team_export_parser = team_subparsers.add_parser(
        "export",
        help="Export Grafana teams and membership to JSON files.",
    )
    add_common_cli_args(team_export_parser)
    add_access_export_cli_args(
        team_export_parser,
        DEFAULT_ACCESS_TEAM_EXPORT_DIR,
        resource="team",
    )
    team_export_parser.add_argument(
        "--with-members",
        action="store_true",
        default=True,
        help="Include team members and admin identities in exported team objects.",
    )

    team_import_parser = team_subparsers.add_parser(
        "import",
        help="Import Grafana teams and membership from a JSON export.",
    )
    add_common_cli_args(
        team_import_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(team_import_parser, resource="team")

    team_diff_parser = team_subparsers.add_parser(
        "diff",
        help="Diff Grafana teams against a previously exported teams.json file.",
    )
    add_common_cli_args(
        team_diff_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(team_diff_parser, resource="team")

    team_delete_parser = team_subparsers.add_parser(
        "delete",
        help="Delete a Grafana team.",
    )
    add_common_cli_args(team_delete_parser)
    add_team_delete_cli_args(team_delete_parser)

    service_account_parser = subparsers.add_parser(
        "service-account",
        help="List, create, export, import, diff, and delete Grafana service accounts.",
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

    service_account_export_parser = service_account_subparsers.add_parser(
        "export",
        help="Export Grafana service accounts to JSON files.",
    )
    add_common_cli_args(service_account_export_parser)
    add_access_export_cli_args(
        service_account_export_parser,
        DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        resource="service-account",
    )

    service_account_import_parser = service_account_subparsers.add_parser(
        "import",
        help="Import Grafana service accounts from a JSON export.",
    )
    add_common_cli_args(service_account_import_parser)
    add_access_import_cli_args(
        service_account_import_parser,
        resource="service-account",
    )
    service_account_import_parser.add_argument(
        "--table",
        action="store_true",
        help="Render service-account import dry-run output as a table.",
    )
    service_account_import_parser.add_argument(
        "--json",
        action="store_true",
        help="Render service-account import dry-run output as JSON.",
    )
    service_account_import_parser.add_argument(
        "--output-format",
        choices=DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help="Alternative single-flag output selector for --dry-run output. Use text, table, or json.",
    )

    service_account_diff_parser = service_account_subparsers.add_parser(
        "diff",
        help="Diff Grafana service accounts against a previously exported snapshot.",
    )
    add_common_cli_args(service_account_diff_parser)
    add_access_diff_cli_args(
        service_account_diff_parser,
        resource="service-account",
    )

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
    parser,
    allow_token_auth=True,
    username_dest="username",
    password_dest="password",
):
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
        parser.add_argument(
            "--prompt-token",
            action="store_true",
            help=(
                "Prompt for the Grafana API token without echo instead of "
                "passing --token on the command line."
            ),
        )
    parser.add_argument(
        "--basic-user",
        dest=username_dest,
        default=None,
        metavar="USERNAME",
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    parser.add_argument(
        "--basic-password",
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


def add_user_list_cli_args(parser):
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
        help=(
            "Include team memberships. API token auth is not supported here; use "
            "Grafana username/password login."
        ),
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


def add_user_add_cli_args(parser):
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


def add_user_modify_cli_args(parser):
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


def add_user_delete_cli_args(parser):
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


def add_service_account_list_cli_args(parser):
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


def add_team_list_cli_args(parser):
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


def add_team_modify_cli_args(parser):
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


def add_team_add_cli_args(parser):
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


def add_service_account_add_cli_args(parser):
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


def add_service_account_token_add_cli_args(parser):
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


def access_export_filename(resource):
    if resource == "user":
        return ACCESS_USER_EXPORT_FILENAME
    if resource == "team":
        return ACCESS_TEAM_EXPORT_FILENAME
    if resource == "service-account":
        return ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME
    raise ValueError("Unsupported access export resource: %s" % resource)


def parse_args(argv=None):
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


def _normalize_output_format_args(args, parser):
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
