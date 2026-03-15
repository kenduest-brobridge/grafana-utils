"""Argparse wiring for the Python access-management CLI."""

import argparse
import sys

from ..batch_error_policy import add_error_policy_argument
from ..http_transport import DEFAULT_HTTP_TRANSPORT, HTTP_TRANSPORT_CHOICES
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
DEFAULT_ACCESS_ORG_EXPORT_DIR = "access-orgs"
DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR = "access-service-accounts"
ACCESS_USER_EXPORT_FILENAME = "users.json"
ACCESS_TEAM_EXPORT_FILENAME = "teams.json"
ACCESS_ORG_EXPORT_FILENAME = "orgs.json"
ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME = "service-accounts.json"
ACCESS_EXPORT_METADATA_FILENAME = "export-metadata.json"
ACCESS_EXPORT_KIND_USERS = "grafana-utils-access-user-export-index"
ACCESS_EXPORT_KIND_TEAMS = "grafana-utils-access-team-export-index"
ACCESS_EXPORT_KIND_ORGS = "grafana-utils-access-org-export-index"
ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS = (
    "grafana-utils-access-service-account-export-index"
)
ACCESS_EXPORT_VERSION = 1
SCOPE_CHOICES = ("org", "global")
LIST_OUTPUT_FORMAT_CHOICES = ("text", "table", "csv", "json")
DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")


def _build_help_examples(*sections):
    chunks = []
    for title, commands in sections:
        lines = [title + ":"]
        for command in commands:
            lines.append("  " + command)
        chunks.append("\n".join(lines))
    return "Examples:\n\n" + "\n\n".join(chunks)


def _add_parser_with_examples(subparsers, name, help_text, *sections):
    return subparsers.add_parser(
        name,
        help=help_text,
        epilog=_build_help_examples(*sections),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )


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
        help=("Overwrite existing export files instead of failing."),
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
    add_error_policy_argument(parser, "%s import" % resource)


def add_access_diff_cli_args(parser, resource, default_scope=DEFAULT_SCOPE):
    parser.add_argument(
        "--diff-dir",
        required=True,
        help=(
            "Diff directory that contains %s and %s."
            % (
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
                "Match against global or org user listing (default: %s)."
                % default_scope
            ),
        )
    add_error_policy_argument(parser, "%s diff" % resource)


def build_parser(prog=None):
    parser = argparse.ArgumentParser(
        prog=prog,
        description="List and manage Grafana users, teams, organizations, and service accounts.",
        epilog=_build_help_examples(
            (
                "List org users as a table",
                ["grafana-util access user list --url http://localhost:3000 --table"],
            ),
            (
                "Export organizations with memberships",
                [
                    "grafana-util access org export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-orgs --with-users --overwrite"
                ],
            ),
            (
                "Preview a service-account import as JSON",
                [
                    "grafana-util access service-account import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-service-accounts --replace-existing --dry-run --output-format json"
                ],
            ),
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="resource")
    subparsers.required = True

    user_parser = _add_parser_with_examples(
        subparsers,
        "user",
        "List Grafana users.",
        (
            "List users through the org-scoped API",
            ["grafana-util access user list --url http://localhost:3000 --table"],
        ),
        (
            "Export global users with team memberships",
            [
                "grafana-util access user export --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --with-teams --export-dir ./access-users --overwrite"
            ],
        ),
    )
    user_subparsers = user_parser.add_subparsers(dest="command")
    user_subparsers.required = True

    list_parser = _add_parser_with_examples(
        user_subparsers,
        "list",
        "List Grafana users from org-scoped or global APIs.",
        (
            "List org users in table form",
            ["grafana-util access user list --url http://localhost:3000 --table"],
        ),
        (
            "List global users with team memberships as JSON",
            [
                "grafana-util access user list --url http://localhost:3000 --scope global --with-teams --output-format json"
            ],
        ),
    )
    add_common_cli_args(list_parser)
    add_user_list_cli_args(list_parser)

    user_export_parser = _add_parser_with_examples(
        user_subparsers,
        "export",
        "Export Grafana users to JSON files.",
        (
            "Export global users into a dedicated directory",
            [
                "grafana-util access user export --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --export-dir ./access-users --overwrite"
            ],
        ),
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

    user_import_parser = _add_parser_with_examples(
        user_subparsers,
        "import",
        "Import Grafana users from a JSON export.",
        (
            "Preview a global-user import before writing",
            [
                "grafana-util access user import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-users --scope global --replace-existing --dry-run"
            ],
        ),
    )
    add_common_cli_args(
        user_import_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(
        user_import_parser, resource="user", default_scope=DEFAULT_SCOPE
    )

    user_diff_parser = _add_parser_with_examples(
        user_subparsers,
        "diff",
        "Diff Grafana users against a previously exported users.json file.",
        (
            "Compare exported users with live Grafana",
            [
                "grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global"
            ],
        ),
    )
    add_common_cli_args(
        user_diff_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(
        user_diff_parser, resource="user", default_scope=DEFAULT_SCOPE
    )

    add_parser = _add_parser_with_examples(
        user_subparsers,
        "add",
        "Create a Grafana user through the global admin API.",
        (
            "Create one global user and assign an org role",
            [
                'grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name "Alice Example" --password secret123 --org-role Editor'
            ],
        ),
    )
    add_common_cli_args(
        add_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_add_cli_args(add_parser)

    modify_parser = _add_parser_with_examples(
        user_subparsers,
        "modify",
        "Modify a Grafana user through the global admin APIs.",
        (
            "Rename a user and update the org role",
            [
                'grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-email alice2@example.com --set-name "Alice Two" --set-org-role Admin'
            ],
        ),
    )
    add_common_cli_args(
        modify_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_modify_cli_args(modify_parser)

    delete_parser = _add_parser_with_examples(
        user_subparsers,
        "delete",
        "Delete a Grafana user from the org or globally.",
        (
            "Remove a user from the current org",
            [
                "grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --email alice@example.com --scope org --yes"
            ],
        ),
        (
            "Delete a global user account",
            [
                "grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --scope global --yes"
            ],
        ),
    )
    add_common_cli_args(
        delete_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_delete_cli_args(delete_parser)

    team_parser = _add_parser_with_examples(
        subparsers,
        "team",
        "List Grafana teams.",
        (
            "List teams with member details",
            [
                "grafana-util access team list --url http://localhost:3000 --with-members --table"
            ],
        ),
        (
            "Preview a team import",
            [
                "grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run"
            ],
        ),
    )
    team_subparsers = team_parser.add_subparsers(dest="command")
    team_subparsers.required = True

    team_list_parser = _add_parser_with_examples(
        team_subparsers,
        "list",
        "List Grafana teams from the org-scoped API.",
        (
            "List teams with members in table form",
            [
                "grafana-util access team list --url http://localhost:3000 --with-members --table"
            ],
        ),
    )
    add_common_cli_args(team_list_parser)
    add_team_list_cli_args(team_list_parser)

    team_add_parser = _add_parser_with_examples(
        team_subparsers,
        "add",
        "Create a Grafana team and optionally seed members and admins.",
        (
            "Create a team and seed one member",
            [
                'grafana-util access team add --url http://localhost:3000 --basic-user admin --basic-password admin --name "Platform Ops" --email platform@example.com --member alice@example.com'
            ],
        ),
    )
    add_common_cli_args(team_add_parser)
    add_team_add_cli_args(team_add_parser)

    team_modify_parser = _add_parser_with_examples(
        team_subparsers,
        "modify",
        "Modify Grafana team members and team admins.",
        (
            "Replace one team membership set",
            [
                'grafana-util access team modify --url http://localhost:3000 --basic-user admin --basic-password admin --name "Platform Ops" --add-member alice@example.com --add-admin lead@example.com'
            ],
        ),
    )
    add_common_cli_args(team_modify_parser)
    add_team_modify_cli_args(team_modify_parser)

    team_export_parser = _add_parser_with_examples(
        team_subparsers,
        "export",
        "Export Grafana teams and membership to JSON files.",
        (
            "Export teams with member details",
            [
                "grafana-util access team export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-teams --overwrite"
            ],
        ),
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

    team_import_parser = _add_parser_with_examples(
        team_subparsers,
        "import",
        "Import Grafana teams and membership from a JSON export.",
        (
            "Preview a team import",
            [
                "grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run"
            ],
        ),
    )
    add_common_cli_args(
        team_import_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(team_import_parser, resource="team")

    team_diff_parser = _add_parser_with_examples(
        team_subparsers,
        "diff",
        "Diff Grafana teams against a previously exported teams.json file.",
        (
            "Compare exported teams with Grafana",
            [
                "grafana-util access team diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-teams"
            ],
        ),
    )
    add_common_cli_args(
        team_diff_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(team_diff_parser, resource="team")

    team_delete_parser = _add_parser_with_examples(
        team_subparsers,
        "delete",
        "Delete a Grafana team.",
        (
            "Delete one team by name",
            [
                'grafana-util access team delete --url http://localhost:3000 --basic-user admin --basic-password admin --name "Platform Ops" --yes'
            ],
        ),
    )
    add_common_cli_args(team_delete_parser)
    add_team_delete_cli_args(team_delete_parser)

    org_parser = _add_parser_with_examples(
        subparsers,
        "org",
        "List and manage Grafana organizations.",
        (
            "List organizations with org-user memberships",
            [
                "grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --table"
            ],
        ),
        (
            "Preview an organization import",
            [
                "grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-orgs --replace-existing --dry-run"
            ],
        ),
    )
    org_subparsers = org_parser.add_subparsers(dest="command")
    org_subparsers.required = True

    org_list_parser = _add_parser_with_examples(
        org_subparsers,
        "list",
        "List Grafana organizations from the admin API.",
        (
            "List organizations with memberships",
            [
                "grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --table"
            ],
        ),
    )
    add_common_cli_args(
        org_list_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_list_cli_args(org_list_parser)

    org_add_parser = _add_parser_with_examples(
        org_subparsers,
        "add",
        "Create a Grafana organization.",
        (
            "Create one organization",
            [
                'grafana-util access org add --url http://localhost:3000 --basic-user admin --basic-password admin --name "Org Two"'
            ],
        ),
    )
    add_common_cli_args(
        org_add_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_add_cli_args(org_add_parser)

    org_modify_parser = _add_parser_with_examples(
        org_subparsers,
        "modify",
        "Rename a Grafana organization.",
        (
            "Rename one organization by id",
            [
                'grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --set-name "Org Two Renamed"'
            ],
        ),
    )
    add_common_cli_args(
        org_modify_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_modify_cli_args(org_modify_parser)

    org_delete_parser = _add_parser_with_examples(
        org_subparsers,
        "delete",
        "Delete a Grafana organization.",
        (
            "Delete one organization by id",
            [
                "grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 4 --yes"
            ],
        ),
    )
    add_common_cli_args(
        org_delete_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_delete_cli_args(org_delete_parser)

    org_export_parser = _add_parser_with_examples(
        org_subparsers,
        "export",
        "Export Grafana organizations to JSON files.",
        (
            "Export organizations with org-user memberships",
            [
                "grafana-util access org export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-orgs --with-users --overwrite"
            ],
        ),
    )
    add_common_cli_args(
        org_export_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_access_export_cli_args(
        org_export_parser,
        DEFAULT_ACCESS_ORG_EXPORT_DIR,
        resource="org",
    )
    add_org_export_cli_args(org_export_parser)

    org_import_parser = _add_parser_with_examples(
        org_subparsers,
        "import",
        "Import Grafana organizations from a JSON export.",
        (
            "Preview an organization import",
            [
                "grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-orgs --replace-existing --dry-run"
            ],
        ),
    )
    add_common_cli_args(
        org_import_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_access_import_cli_args(org_import_parser, resource="org")

    service_account_parser = _add_parser_with_examples(
        subparsers,
        "service-account",
        "List, create, export, import, diff, and delete Grafana service accounts.",
        (
            "List service accounts as a table",
            [
                "grafana-util access service-account list --url http://localhost:3000 --table"
            ],
        ),
        (
            "Create a service-account token",
            [
                "grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name ci-token --seconds-to-live 86400"
            ],
        ),
    )
    service_account_subparsers = service_account_parser.add_subparsers(dest="command")
    service_account_subparsers.required = True

    service_account_list_parser = _add_parser_with_examples(
        service_account_subparsers,
        "list",
        "List Grafana service accounts.",
        (
            "List service accounts as a table",
            [
                "grafana-util access service-account list --url http://localhost:3000 --table"
            ],
        ),
    )
    add_common_cli_args(service_account_list_parser)
    add_service_account_list_cli_args(service_account_list_parser)

    service_account_add_parser = _add_parser_with_examples(
        service_account_subparsers,
        "add",
        "Create a Grafana service account.",
        (
            "Create an editor service account",
            [
                "grafana-util access service-account add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --role Editor"
            ],
        ),
    )
    add_common_cli_args(service_account_add_parser)
    add_service_account_add_cli_args(service_account_add_parser)

    service_account_export_parser = _add_parser_with_examples(
        service_account_subparsers,
        "export",
        "Export Grafana service accounts to JSON files.",
        (
            "Export service accounts",
            [
                "grafana-util access service-account export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-service-accounts --overwrite"
            ],
        ),
    )
    add_common_cli_args(service_account_export_parser)
    add_access_export_cli_args(
        service_account_export_parser,
        DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        resource="service-account",
    )

    service_account_import_parser = _add_parser_with_examples(
        service_account_subparsers,
        "import",
        "Import Grafana service accounts from a JSON export.",
        (
            "Preview a service-account import as a table",
            [
                "grafana-util access service-account import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-service-accounts --replace-existing --dry-run --output-format table"
            ],
        ),
    )
    add_common_cli_args(service_account_import_parser)
    add_access_import_cli_args(
        service_account_import_parser,
        resource="service-account",
    )
    output_group = service_account_import_parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render service-account import dry-run output as a table.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render service-account import dry-run output as JSON.",
    )
    output_group.add_argument(
        "--output-format",
        choices=DRY_RUN_OUTPUT_FORMAT_CHOICES,
        default=None,
        help="Alternative single-flag output selector for --dry-run output. Use text, table, or json.",
    )

    service_account_diff_parser = _add_parser_with_examples(
        service_account_subparsers,
        "diff",
        "Diff Grafana service accounts against a previously exported snapshot.",
        (
            "Compare exported service accounts with Grafana",
            [
                "grafana-util access service-account diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-service-accounts"
            ],
        ),
    )
    add_common_cli_args(service_account_diff_parser)
    add_access_diff_cli_args(
        service_account_diff_parser,
        resource="service-account",
    )

    service_account_delete_parser = _add_parser_with_examples(
        service_account_subparsers,
        "delete",
        "Delete a Grafana service account.",
        (
            "Delete one service account by name",
            [
                "grafana-util access service-account delete --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --yes"
            ],
        ),
    )
    add_common_cli_args(service_account_delete_parser)
    add_service_account_delete_cli_args(service_account_delete_parser)

    service_account_token_parser = _add_parser_with_examples(
        service_account_subparsers,
        "token",
        "Manage Grafana service-account tokens.",
        (
            "Create one token for a service account",
            [
                "grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name ci-token --seconds-to-live 86400"
            ],
        ),
        (
            "Delete one service-account token",
            [
                "grafana-util access service-account token delete --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name ci-token --yes"
            ],
        ),
    )
    service_account_token_subparsers = service_account_token_parser.add_subparsers(
        dest="token_command"
    )
    service_account_token_subparsers.required = True

    service_account_token_add_parser = _add_parser_with_examples(
        service_account_token_subparsers,
        "add",
        "Create a Grafana service-account token.",
        (
            "Create one service-account token",
            [
                "grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name ci-token --seconds-to-live 86400"
            ],
        ),
    )
    add_common_cli_args(service_account_token_add_parser)
    add_service_account_token_add_cli_args(service_account_token_add_parser)

    service_account_token_delete_parser = _add_parser_with_examples(
        service_account_token_subparsers,
        "delete",
        "Delete a Grafana service-account token.",
        (
            "Delete one service-account token",
            [
                "grafana-util access service-account token delete --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name ci-token --yes"
            ],
        ),
    )
    add_common_cli_args(service_account_token_delete_parser)
    add_service_account_token_delete_cli_args(service_account_token_delete_parser)
    return parser


def add_common_cli_args(
    parser,
    allow_token_auth=True,
    username_dest="username",
    password_dest="password",
    include_org_id=True,
    group_name="Connection And Auth",
):
    target = parser.add_argument_group(group_name) if group_name else parser
    target.add_argument(
        "--url",
        default=DEFAULT_URL,
        help="Grafana base URL (default: %s)" % DEFAULT_URL,
    )
    if allow_token_auth:
        target.add_argument(
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
        target.add_argument(
            "--prompt-token",
            action="store_true",
            help=(
                "Prompt for the Grafana API token without echo instead of "
                "passing --token on the command line."
            ),
        )
    target.add_argument(
        "--basic-user",
        dest=username_dest,
        default=None,
        metavar="USERNAME",
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    target.add_argument(
        "--basic-password",
        dest=password_dest,
        default=None,
        metavar="PASSWORD",
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
        ),
    )
    target.add_argument(
        "--prompt-password",
        action="store_true",
        help=(
            "Prompt for the Grafana Basic auth password without echo instead of "
            "passing --basic-password on the command line."
        ),
    )
    if include_org_id:
        target.add_argument(
            "--org-id",
            default=None,
            help="Grafana organization id to send through X-Grafana-Org-Id.",
        )
    target.add_argument(
        "--timeout",
        type=positive_int,
        default=DEFAULT_TIMEOUT,
        help="HTTP timeout in seconds (default: %s)." % DEFAULT_TIMEOUT,
    )
    target.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    target.add_argument(
        "--http-transport",
        choices=HTTP_TRANSPORT_CHOICES,
        default=DEFAULT_HTTP_TRANSPORT,
        help=(
            "Select the HTTP transport implementation. " "Use auto, requests, or httpx."
        ),
    )


def add_user_list_cli_args(parser):
    filter_group = parser.add_argument_group("Filters")
    filter_group.add_argument(
        "--scope",
        choices=SCOPE_CHOICES,
        default=DEFAULT_SCOPE,
        help="Choose org-scoped or global user listing (default: %s)." % DEFAULT_SCOPE,
    )
    filter_group.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match across login, email, and name.",
    )
    filter_group.add_argument(
        "--login",
        default=None,
        help="Filter to one exact login.",
    )
    filter_group.add_argument(
        "--email",
        default=None,
        help="Filter to one exact email.",
    )
    filter_group.add_argument(
        "--org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Filter by Grafana organization role.",
    )
    filter_group.add_argument(
        "--grafana-admin",
        default=None,
        type=bool_choice,
        help="Filter by Grafana server-admin state: true or false.",
    )
    filter_group.add_argument(
        "--with-teams",
        action="store_true",
        help=(
            "Include team memberships. API token auth is not supported here; use "
            "Grafana username/password login."
        ),
    )
    filter_group.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Page number after filtering (default: 1).",
    )
    filter_group.add_argument(
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
    identity_group = parser.add_argument_group("User Identity")
    identity_group.add_argument(
        "--login",
        required=True,
        help="Login name for the new Grafana user.",
    )
    identity_group.add_argument(
        "--email",
        required=True,
        help="Email address for the new Grafana user.",
    )
    identity_group.add_argument(
        "--name",
        required=True,
        help="Display name for the new Grafana user.",
    )
    password_group = parser.add_mutually_exclusive_group(required=True)
    password_group.add_argument(
        "--password",
        dest="new_user_password",
        default=None,
        help="Password for the new local Grafana user.",
    )
    password_group.add_argument(
        "--password-file",
        dest="new_user_password_file",
        default=None,
        help="Read the new local Grafana user password from this file.",
    )
    password_group.add_argument(
        "--prompt-user-password",
        action="store_true",
        help="Prompt for the new local Grafana user password without echo.",
    )
    privileges_group = parser.add_argument_group("Privileges")
    privileges_group.add_argument(
        "--org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Optional Grafana organization role to set after user creation.",
    )
    privileges_group.add_argument(
        "--grafana-admin",
        default=None,
        type=bool_choice,
        help="Optional Grafana server-admin state to set after user creation: true or false.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the created user as JSON.",
    )


def add_user_modify_cli_args(parser):
    identity_group = parser.add_argument_group("Target Identity")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--user-id",
        default=None,
        help="Modify the user identified by this Grafana user id.",
    )
    identity_mutually.add_argument(
        "--login",
        default=None,
        help="Resolve the user by exact login before modifying it.",
    )
    identity_mutually.add_argument(
        "--email",
        default=None,
        help="Resolve the user by exact email before modifying it.",
    )
    mutate_group = parser.add_argument_group("Profile Changes")
    mutate_group.add_argument(
        "--set-login",
        default=None,
        help="Set a new login for the target user.",
    )
    mutate_group.add_argument(
        "--set-email",
        default=None,
        help="Set a new email address for the target user.",
    )
    mutate_group.add_argument(
        "--set-name",
        default=None,
        help="Set a new display name for the target user.",
    )
    password_group = parser.add_mutually_exclusive_group()
    password_group.add_argument(
        "--set-password",
        default=None,
        help="Set a new local password for the target user.",
    )
    password_group.add_argument(
        "--set-password-file",
        default=None,
        help="Read the new local password for the target user from this file.",
    )
    password_group.add_argument(
        "--prompt-set-password",
        action="store_true",
        help="Prompt for the target user's new local password without echo.",
    )
    privileges_group = parser.add_argument_group("Privileges")
    privileges_group.add_argument(
        "--set-org-role",
        default=None,
        choices=["Viewer", "Editor", "Admin", "None"],
        help="Optional Grafana organization role to set after profile changes.",
    )
    privileges_group.add_argument(
        "--set-grafana-admin",
        default=None,
        type=bool_choice,
        help="Optional Grafana server-admin state to set after profile changes: true or false.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the modified user as JSON.",
    )


def add_user_delete_cli_args(parser):
    identity_group = parser.add_argument_group("Target")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--user-id",
        default=None,
        help="Delete the user identified by this Grafana user id.",
    )
    identity_mutually.add_argument(
        "--login",
        default=None,
        help="Resolve the user by exact login before deleting it.",
    )
    identity_mutually.add_argument(
        "--email",
        default=None,
        help="Resolve the user by exact email before deleting it.",
    )
    action_group = parser.add_argument_group("Delete Action")
    action_group.add_argument(
        "--scope",
        choices=SCOPE_CHOICES,
        default="global",
        help="Choose org-scoped removal or global deletion (default: global).",
    )
    action_group.add_argument(
        "--yes",
        action="store_true",
        help="Confirm that the target user should be deleted or removed.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the deleted user summary as JSON.",
    )


def add_service_account_list_cli_args(parser):
    filter_group = parser.add_argument_group("Filters")
    filter_group.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against service-account name or login.",
    )
    pagination_group = parser.add_argument_group("Pagination")
    pagination_group.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Grafana search page number (default: 1).",
    )
    pagination_group.add_argument(
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


def add_org_list_cli_args(parser):
    filter_group = parser.add_argument_group("Selection")
    filter_group.add_argument(
        "--org-id",
        default=None,
        help="Filter to one exact organization id.",
    )
    filter_group.add_argument(
        "--name",
        default=None,
        help="Filter to one exact organization name.",
    )
    filter_group.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against organization name.",
    )
    membership_group = parser.add_argument_group("Membership")
    membership_group.add_argument(
        "--with-users",
        action="store_true",
        help="Include organization users and roles in the output.",
    )
    output_group = parser.add_mutually_exclusive_group()
    output_group.add_argument(
        "--table",
        action="store_true",
        help="Render organizations as a table.",
    )
    output_group.add_argument(
        "--csv",
        action="store_true",
        help="Render organizations as CSV.",
    )
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render organizations as JSON.",
    )
    add_list_output_format_arg(parser)


def add_org_add_cli_args(parser):
    identity_group = parser.add_argument_group("Org Identity")
    identity_group.add_argument(
        "--name",
        required=True,
        help="Organization name to create.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the created organization as JSON.",
    )


def add_org_modify_cli_args(parser):
    identity_group = parser.add_argument_group("Target Selection")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--org-id",
        dest="target_org_id",
        default=None,
        help="Rename the organization identified by this Grafana organization id.",
    )
    identity_mutually.add_argument(
        "--name",
        default=None,
        help="Resolve the organization by exact name before renaming it.",
    )
    updates_group = parser.add_argument_group("Org Updates")
    updates_group.add_argument(
        "--set-name",
        required=True,
        help="Set a new organization name for the target org.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the modified organization as JSON.",
    )


def add_org_delete_cli_args(parser):
    identity_group = parser.add_argument_group("Target Selection")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--org-id",
        dest="target_org_id",
        default=None,
        help="Delete the organization identified by this Grafana organization id.",
    )
    identity_mutually.add_argument(
        "--name",
        default=None,
        help="Resolve the organization by exact name before deleting it.",
    )
    safety_group = parser.add_argument_group("Safety")
    safety_group.add_argument(
        "--yes",
        action="store_true",
        help="Confirm that the target organization should be deleted.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the deleted organization summary as JSON.",
    )


def add_org_export_cli_args(parser):
    scope_group = parser.add_argument_group("Export Scope")
    scope_group.add_argument(
        "--org-id",
        default=None,
        help="Filter export to one exact organization id.",
    )
    scope_group.add_argument(
        "--name",
        default=None,
        help="Filter export to one exact organization name.",
    )
    controls_group = parser.add_argument_group("Export Controls")
    controls_group.add_argument(
        "--with-users",
        action="store_true",
        help="Include organization users and org roles in the export bundle.",
    )


def add_team_list_cli_args(parser):
    filter_group = parser.add_argument_group("Filters")
    filter_group.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against team name or email.",
    )
    filter_group.add_argument(
        "--name",
        default=None,
        help="Filter to one exact team name.",
    )
    membership_group = parser.add_argument_group("Membership")
    membership_group.add_argument(
        "--with-members",
        action="store_true",
        help="Include team member login names when the API returns them.",
    )
    pagination_group = parser.add_argument_group("Pagination")
    pagination_group.add_argument(
        "--page",
        type=positive_int,
        default=1,
        help="Page number after filtering (default: 1).",
    )
    pagination_group.add_argument(
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
    identity_group = parser.add_argument_group("Target Selection")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--team-id",
        default=None,
        help="Modify the team identified by this Grafana team id.",
    )
    identity_mutually.add_argument(
        "--name",
        default=None,
        help="Resolve the team by exact name before modifying memberships.",
    )
    membership_group = parser.add_argument_group("Membership")
    membership_group.add_argument(
        "--add-member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one team member by exact login or exact email. Repeat as needed.",
    )
    membership_group.add_argument(
        "--remove-member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Remove one team member by exact login or exact email. Repeat as needed.",
    )
    membership_group.add_argument(
        "--add-admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Promote one user to team admin by exact login or exact email. Repeat as needed.",
    )
    membership_group.add_argument(
        "--remove-admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Demote one team admin to regular team member by exact login or exact email. Repeat as needed.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the team modification result as JSON.",
    )


def add_team_add_cli_args(parser):
    definition_group = parser.add_argument_group("Team Definition")
    definition_group.add_argument(
        "--name",
        required=True,
        help="Team name to create.",
    )
    definition_group.add_argument(
        "--email",
        default=None,
        help="Optional team email address to store in Grafana.",
    )
    membership_group = parser.add_argument_group("Team Membership")
    membership_group.add_argument(
        "--member",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one initial team member by exact login or exact email. Repeat as needed.",
    )
    membership_group.add_argument(
        "--admin",
        action="append",
        default=[],
        metavar="LOGIN_OR_EMAIL",
        help="Add one initial team admin by exact login or exact email. Repeat as needed.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the created team as JSON.",
    )


def add_service_account_add_cli_args(parser):
    identity_group = parser.add_argument_group("Service Account Identity")
    identity_group.add_argument(
        "--name",
        required=True,
        help="Service-account name to create.",
    )
    identity_group.add_argument(
        "--role",
        default=DEFAULT_SERVICE_ACCOUNT_ROLE,
        choices=["Viewer", "Editor", "Admin", "None"],
        help=("Service-account org role (default: %s)." % DEFAULT_SERVICE_ACCOUNT_ROLE),
    )
    identity_group.add_argument(
        "--disabled",
        default="false",
        type=bool_choice,
        help="Create the service account in disabled state: true or false.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the created service account as JSON.",
    )


def add_service_account_token_add_cli_args(parser):
    identity_group = parser.add_argument_group("Target Selection")
    identity_mutually = identity_group.add_mutually_exclusive_group(required=True)
    identity_mutually.add_argument(
        "--service-account-id",
        default=None,
        help="Service-account id that should own the new token.",
    )
    identity_mutually.add_argument(
        "--name",
        default=None,
        help="Resolve the service account by exact name before creating the token.",
    )
    token_group = parser.add_argument_group("Token Settings")
    token_group.add_argument(
        "--token-name",
        required=True,
        help="Token name to create under the target service account.",
    )
    token_group.add_argument(
        "--seconds-to-live",
        type=positive_int,
        default=None,
        help="Optional token lifetime in seconds.",
    )
    output_group = parser.add_argument_group("Output Options")
    output_group.add_argument(
        "--json",
        action="store_true",
        help="Render the created token payload as JSON.",
    )


def access_export_filename(resource):
    if resource == "user":
        return ACCESS_USER_EXPORT_FILENAME
    if resource == "team":
        return ACCESS_TEAM_EXPORT_FILENAME
    if resource == "org":
        return ACCESS_ORG_EXPORT_FILENAME
    if resource == "service-account":
        return ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME
    raise ValueError("Unsupported access export resource: %s" % resource)


def parse_args(argv=None):
    parser = build_parser()
    argv = normalize_group_alias_argv(list(sys.argv[1:] if argv is None else argv))

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

    if argv == ["org"]:
        parser._subparsers._group_actions[0].choices["org"].print_help()
        raise SystemExit(0)

    if argv == ["service-account"]:
        parser._subparsers._group_actions[0].choices["service-account"].print_help()
        raise SystemExit(0)

    if argv == ["service-account", "token"]:
        parser._subparsers._group_actions[0].choices[
            "service-account"
        ]._subparsers._group_actions[0].choices["token"].print_help()
        raise SystemExit(0)

    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    return args


def _normalize_output_format_args(args, parser):
    output_format = getattr(args, "output_format", None)
    if output_format is None:
        return
    if (
        bool(getattr(args, "table", False))
        or bool(getattr(args, "csv", False))
        or bool(getattr(args, "json", False))
    ):
        parser.error(
            "--output-format cannot be combined with --table, --csv, or --json for access list commands."
        )
    args.table = output_format == "table"
    args.csv = output_format == "csv"
    args.json = output_format == "json"
