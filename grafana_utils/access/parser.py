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
ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS = "grafana-utils-access-service-account-export-index"
ACCESS_EXPORT_VERSION = 1
SCOPE_CHOICES = ("org", "global")
LIST_OUTPUT_FORMAT_CHOICES = ("text", "table", "csv", "json")
DRY_RUN_OUTPUT_FORMAT_CHOICES = ("text", "table", "json")

ACCESS_ROOT_HELP_EXAMPLES = (
    "Examples:\n\n"
    "  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n"
    "  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --dry-run --table --yes\n"
    "  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly"
)

ACCESS_RESOURCE_HELP_EXAMPLES = {
    "user": (
        "Examples:\n\n"
        "  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n"
        "  grafana-util access user import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-users --dry-run --table"
    ),
    "team": (
        "Examples:\n\n"
        "  grafana-util access team list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n"
        "  grafana-util access team add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name Ops --member alice --admin carol"
    ),
    "org": (
        "Examples:\n\n"
        "  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --json\n"
        "  grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-orgs --dry-run --yes"
    ),
    "service-account": (
        "Examples:\n\n"
        "  grafana-util access service-account list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n"
        "  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly"
    ),
}

ACCESS_COMMAND_HELP_EXAMPLES = {
    ("user", "list"): "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --with-teams --json",
    ("user", "export"): "Examples:\n\n  grafana-util access user export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-users --with-teams --overwrite",
    ("user", "import"): "Examples:\n\n  grafana-util access user import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-users --replace-existing --dry-run --table --yes",
    ("user", "diff"): "Examples:\n\n  grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global",
    ("user", "add"): "Examples:\n\n  grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret --org-role Editor",
    ("user", "modify"): "Examples:\n\n  grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-name 'Alice Ops' --set-org-role Admin",
    ("user", "delete"): "Examples:\n\n  grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --scope global --yes",
    ("team", "list"): "Examples:\n\n  grafana-util access team list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --with-members --json",
    ("team", "add"): "Examples:\n\n  grafana-util access team add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name Ops --member alice --admin carol",
    ("team", "modify"): "Examples:\n\n  grafana-util access team modify --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name Ops --add-member alice --remove-admin bob",
    ("team", "export"): "Examples:\n\n  grafana-util access team export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-teams --overwrite",
    ("team", "import"): "Examples:\n\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run --table --yes",
    ("team", "diff"): "Examples:\n\n  grafana-util access team diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-teams",
    ("team", "delete"): "Examples:\n\n  grafana-util access team delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name Ops --yes",
    ("org", "list"): "Examples:\n\n  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --json",
    ("org", "add"): "Examples:\n\n  grafana-util access org add --url http://localhost:3000 --basic-user admin --basic-password admin --name Platform --json",
    ("org", "modify"): "Examples:\n\n  grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name Platform --set-name 'Platform Core'",
    ("org", "delete"): "Examples:\n\n  grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name Platform --yes",
    ("org", "export"): "Examples:\n\n  grafana-util access org export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-orgs --with-users --overwrite",
    ("org", "import"): "Examples:\n\n  grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-orgs --replace-existing --dry-run --yes",
    ("service-account", "list"): "Examples:\n\n  grafana-util access service-account list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json",
    ("service-account", "add"): "Examples:\n\n  grafana-util access service-account add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --role Editor --json",
    ("service-account", "export"): "Examples:\n\n  grafana-util access service-account export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./access-service-accounts --overwrite",
    ("service-account", "import"): "Examples:\n\n  grafana-util access service-account import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./access-service-accounts --replace-existing --dry-run --table --yes",
    ("service-account", "diff"): "Examples:\n\n  grafana-util access service-account diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./access-service-accounts",
    ("service-account", "delete"): "Examples:\n\n  grafana-util access service-account delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --yes",
    ("service-account", "token"): "Examples:\n\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly",
    ("service-account", "token-add"): "Examples:\n\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly --seconds-to-live 3600",
    ("service-account", "token-delete"): "Examples:\n\n  grafana-util access service-account token delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly --yes",
}

ACCESS_ROOT_HELP_EXAMPLES = """Examples:

  List org users as JSON:
    grafana-util access user list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json

  Create a Grafana user with Basic auth:
    grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password 'secret'

  Import teams with destructive sync acknowledgement:
    grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes

  Create a service-account token:
    grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
"""

USER_LIST_HELP_EXAMPLES = """Examples:

  grafana-util access user list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --scope org --json
  grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --table
"""
USER_EXPORT_HELP_EXAMPLES = """Examples:

  grafana-util access user export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-users --with-teams --overwrite
"""
USER_IMPORT_HELP_EXAMPLES = """Examples:

  grafana-util access user import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-users --scope global --replace-existing --dry-run
  grafana-util access user import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-users --scope org --replace-existing --yes
"""
USER_DIFF_HELP_EXAMPLES = """Examples:

  grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global
"""
USER_ADD_HELP_EXAMPLES = """Examples:

  grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password 'secret'
"""
USER_MODIFY_HELP_EXAMPLES = """Examples:

  grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-name 'Alice Ops' --set-org-role Editor
"""
USER_DELETE_HELP_EXAMPLES = """Examples:

  grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --scope global --yes
"""

TEAM_LIST_HELP_EXAMPLES = """Examples:

  grafana-util access team list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --with-members --json
"""
TEAM_ADD_HELP_EXAMPLES = """Examples:

  grafana-util access team add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name Ops --email ops@example.com --member alice --admin bob --json
"""
TEAM_MODIFY_HELP_EXAMPLES = """Examples:

  grafana-util access team modify --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --team-id 7 --add-member alice --remove-admin bob --json
"""
TEAM_EXPORT_HELP_EXAMPLES = """Examples:

  grafana-util access team export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-teams --with-members --overwrite
"""
TEAM_IMPORT_HELP_EXAMPLES = """Examples:

  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run
  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes
"""
TEAM_DIFF_HELP_EXAMPLES = """Examples:

  grafana-util access team diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-teams
"""
TEAM_DELETE_HELP_EXAMPLES = """Examples:

  grafana-util access team delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name Ops --yes
"""

ORG_LIST_HELP_EXAMPLES = """Examples:

  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --json
"""
ORG_ADD_HELP_EXAMPLES = """Examples:

  grafana-util access org add --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --json
"""
ORG_MODIFY_HELP_EXAMPLES = """Examples:

  grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json
"""
ORG_DELETE_HELP_EXAMPLES = """Examples:

  grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --yes
"""
ORG_EXPORT_HELP_EXAMPLES = """Examples:

  grafana-util access org export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./access-orgs --with-users --overwrite
"""
ORG_IMPORT_HELP_EXAMPLES = """Examples:

  grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-orgs --replace-existing --yes
"""

SERVICE_ACCOUNT_LIST_HELP_EXAMPLES = """Examples:

  grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
"""
SERVICE_ACCOUNT_ADD_HELP_EXAMPLES = """Examples:

  grafana-util access service-account add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --role Editor --json
"""
SERVICE_ACCOUNT_EXPORT_HELP_EXAMPLES = """Examples:

  grafana-util access service-account export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./access-service-accounts --overwrite
"""
SERVICE_ACCOUNT_IMPORT_HELP_EXAMPLES = """Examples:

  grafana-util access service-account import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./access-service-accounts --replace-existing --dry-run
  grafana-util access service-account import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./access-service-accounts --replace-existing --yes
"""
SERVICE_ACCOUNT_DIFF_HELP_EXAMPLES = """Examples:

  grafana-util access service-account diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./access-service-accounts
"""
SERVICE_ACCOUNT_DELETE_HELP_EXAMPLES = """Examples:

  grafana-util access service-account delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --yes
"""
SERVICE_ACCOUNT_TOKEN_ROOT_HELP_EXAMPLES = """Examples:

  grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
  grafana-util access service-account token delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly --yes
"""
SERVICE_ACCOUNT_TOKEN_ADD_HELP_EXAMPLES = """Examples:

  grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly --seconds-to-live 86400 --json
"""
SERVICE_ACCOUNT_TOKEN_DELETE_HELP_EXAMPLES = """Examples:

  grafana-util access service-account token delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly --yes
"""


def subparser_kwargs(epilog=None):
    """Subparser kwargs implementation."""
    kwargs = {"formatter_class": argparse.RawDescriptionHelpFormatter}
    if epilog:
        kwargs["epilog"] = epilog
    return kwargs


def positive_int(value):
    """Positive int implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("value must be >= 1")
    return parsed


def bool_choice(value):
    """Bool choice implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    normalized = str(value).strip().lower()
    if normalized not in {"true", "false"}:
        raise argparse.ArgumentTypeError("value must be true or false")
    return normalized


def parser_help_kwargs(epilog):
    """Parser help kwargs implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return {
        "epilog": epilog,
        "formatter_class": argparse.RawDescriptionHelpFormatter,
    }


def add_list_output_format_arg(parser):
    """Add list output format arg implementation."""
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
    """Add access export cli args implementation."""
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
    """Add access import cli args implementation."""
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
    """Add access diff cli args implementation."""
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
    """Build parser implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1342
    #   Downstream callees: 1015, 1056, 1070, 1096, 1122, 1141, 1189, 1237, 1270, 1298, 240, 286, 312, 353, 652, 745, 819, 873, 941, 977

    parser = argparse.ArgumentParser(
        prog=prog,
        description="List and manage Grafana users, teams, organizations, and service accounts.",
        epilog=ACCESS_ROOT_HELP_EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="resource")
    subparsers.required = True

    user_parser = subparsers.add_parser("user", help="List Grafana users.", **subparser_kwargs())
    user_subparsers = user_parser.add_subparsers(dest="command")
    user_subparsers.required = True

    list_parser = user_subparsers.add_parser("list", help="List Grafana users from org-scoped or global APIs.", **subparser_kwargs(USER_LIST_HELP_EXAMPLES))
    add_common_cli_args(list_parser)
    add_user_list_cli_args(list_parser)

    user_export_parser = user_subparsers.add_parser("export", help="Export Grafana users to JSON files.", **subparser_kwargs(USER_EXPORT_HELP_EXAMPLES))
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

    user_import_parser = user_subparsers.add_parser("import", help="Import Grafana users from a JSON export.", **subparser_kwargs(USER_IMPORT_HELP_EXAMPLES))
    add_common_cli_args(
        user_import_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(user_import_parser, resource="user", default_scope=DEFAULT_SCOPE)

    user_diff_parser = user_subparsers.add_parser("diff", help="Diff Grafana users against a previously exported users.json file.", **subparser_kwargs(USER_DIFF_HELP_EXAMPLES))
    add_common_cli_args(
        user_diff_parser,
        allow_token_auth=True,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(user_diff_parser, resource="user", default_scope=DEFAULT_SCOPE)

    add_parser = user_subparsers.add_parser("add", help="Create a Grafana user through the global admin API.", **subparser_kwargs(USER_ADD_HELP_EXAMPLES))
    add_common_cli_args(
        add_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_add_cli_args(add_parser)

    modify_parser = user_subparsers.add_parser("modify", help="Modify a Grafana user through the global admin APIs.", **subparser_kwargs(USER_MODIFY_HELP_EXAMPLES))
    add_common_cli_args(
        modify_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_modify_cli_args(modify_parser)

    delete_parser = user_subparsers.add_parser("delete", help="Delete a Grafana user from the org or globally.", **subparser_kwargs(USER_DELETE_HELP_EXAMPLES))
    add_common_cli_args(
        delete_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_user_delete_cli_args(delete_parser)

    team_parser = subparsers.add_parser("team", help="List Grafana teams.", **subparser_kwargs())
    team_subparsers = team_parser.add_subparsers(dest="command")
    team_subparsers.required = True

    team_list_parser = team_subparsers.add_parser("list", help="List Grafana teams from the org-scoped API.", **subparser_kwargs(TEAM_LIST_HELP_EXAMPLES))
    add_common_cli_args(team_list_parser)
    add_team_list_cli_args(team_list_parser)

    team_add_parser = team_subparsers.add_parser("add", help="Create a Grafana team and optionally seed members and admins.", **subparser_kwargs(TEAM_ADD_HELP_EXAMPLES))
    add_common_cli_args(team_add_parser)
    add_team_add_cli_args(team_add_parser)

    team_modify_parser = team_subparsers.add_parser("modify", help="Modify Grafana team members and team admins.", **subparser_kwargs(TEAM_MODIFY_HELP_EXAMPLES))
    add_common_cli_args(team_modify_parser)
    add_team_modify_cli_args(team_modify_parser)

    team_export_parser = team_subparsers.add_parser("export", help="Export Grafana teams and membership to JSON files.", **subparser_kwargs(TEAM_EXPORT_HELP_EXAMPLES))
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

    team_import_parser = team_subparsers.add_parser("import", help="Import Grafana teams and membership from a JSON export.", **subparser_kwargs(TEAM_IMPORT_HELP_EXAMPLES))
    add_common_cli_args(
        team_import_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_import_cli_args(team_import_parser, resource="team")

    team_diff_parser = team_subparsers.add_parser("diff", help="Diff Grafana teams against a previously exported teams.json file.", **subparser_kwargs(TEAM_DIFF_HELP_EXAMPLES))
    add_common_cli_args(
        team_diff_parser,
        username_dest="auth_username",
        password_dest="auth_password",
    )
    add_access_diff_cli_args(team_diff_parser, resource="team")

    team_delete_parser = team_subparsers.add_parser("delete", help="Delete a Grafana team.", **subparser_kwargs(TEAM_DELETE_HELP_EXAMPLES))
    add_common_cli_args(team_delete_parser)
    add_team_delete_cli_args(team_delete_parser)

    org_parser = subparsers.add_parser("org", help="List and manage Grafana organizations.", **subparser_kwargs())
    org_subparsers = org_parser.add_subparsers(dest="command")
    org_subparsers.required = True

    org_list_parser = org_subparsers.add_parser("list", help="List Grafana organizations from the admin API.", **subparser_kwargs(ORG_LIST_HELP_EXAMPLES))
    add_common_cli_args(
        org_list_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_list_cli_args(org_list_parser)

    org_add_parser = org_subparsers.add_parser("add", help="Create a Grafana organization.", **subparser_kwargs(ORG_ADD_HELP_EXAMPLES))
    add_common_cli_args(
        org_add_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_add_cli_args(org_add_parser)

    org_modify_parser = org_subparsers.add_parser("modify", help="Rename a Grafana organization.", **subparser_kwargs(ORG_MODIFY_HELP_EXAMPLES))
    add_common_cli_args(
        org_modify_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_modify_cli_args(org_modify_parser)

    org_delete_parser = org_subparsers.add_parser("delete", help="Delete a Grafana organization.", **subparser_kwargs(ORG_DELETE_HELP_EXAMPLES))
    add_common_cli_args(
        org_delete_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_org_delete_cli_args(org_delete_parser)

    org_export_parser = org_subparsers.add_parser("export", help="Export Grafana organizations to JSON files.", **subparser_kwargs(ORG_EXPORT_HELP_EXAMPLES))
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

    org_import_parser = org_subparsers.add_parser("import", help="Import Grafana organizations from a JSON export.", **subparser_kwargs(ORG_IMPORT_HELP_EXAMPLES))
    add_common_cli_args(
        org_import_parser,
        allow_token_auth=False,
        username_dest="auth_username",
        password_dest="auth_password",
        include_org_id=False,
    )
    add_access_import_cli_args(org_import_parser, resource="org")

    service_account_parser = subparsers.add_parser("service-account", help="List, create, export, import, diff, and delete Grafana service accounts.", **subparser_kwargs())
    service_account_subparsers = service_account_parser.add_subparsers(dest="command")
    service_account_subparsers.required = True

    service_account_list_parser = service_account_subparsers.add_parser("list", help="List Grafana service accounts.", **subparser_kwargs(SERVICE_ACCOUNT_LIST_HELP_EXAMPLES))
    add_common_cli_args(service_account_list_parser)
    add_service_account_list_cli_args(service_account_list_parser)

    service_account_add_parser = service_account_subparsers.add_parser("add", help="Create a Grafana service account.", **subparser_kwargs(SERVICE_ACCOUNT_ADD_HELP_EXAMPLES))
    add_common_cli_args(service_account_add_parser)
    add_service_account_add_cli_args(service_account_add_parser)

    service_account_export_parser = service_account_subparsers.add_parser("export", help="Export Grafana service accounts to JSON files.", **subparser_kwargs(SERVICE_ACCOUNT_EXPORT_HELP_EXAMPLES))
    add_common_cli_args(service_account_export_parser)
    add_access_export_cli_args(
        service_account_export_parser,
        DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        resource="service-account",
    )

    service_account_import_parser = service_account_subparsers.add_parser("import", help="Import Grafana service accounts from a JSON export.", **subparser_kwargs(SERVICE_ACCOUNT_IMPORT_HELP_EXAMPLES))
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

    service_account_diff_parser = service_account_subparsers.add_parser("diff", help="Diff Grafana service accounts against a previously exported snapshot.", **subparser_kwargs(SERVICE_ACCOUNT_DIFF_HELP_EXAMPLES))
    add_common_cli_args(service_account_diff_parser)
    add_access_diff_cli_args(
        service_account_diff_parser,
        resource="service-account",
    )

    service_account_delete_parser = service_account_subparsers.add_parser("delete", help="Delete a Grafana service account.", **subparser_kwargs(SERVICE_ACCOUNT_DELETE_HELP_EXAMPLES))
    add_common_cli_args(service_account_delete_parser)
    add_service_account_delete_cli_args(service_account_delete_parser)

    service_account_token_parser = service_account_subparsers.add_parser("token", help="Manage Grafana service-account tokens.", **subparser_kwargs(SERVICE_ACCOUNT_TOKEN_ROOT_HELP_EXAMPLES))
    service_account_token_subparsers = service_account_token_parser.add_subparsers(
        dest="token_command"
    )
    service_account_token_subparsers.required = True

    service_account_token_add_parser = service_account_token_subparsers.add_parser("add", help="Create a Grafana service-account token.", **subparser_kwargs(SERVICE_ACCOUNT_TOKEN_ADD_HELP_EXAMPLES))
    add_common_cli_args(service_account_token_add_parser)
    add_service_account_token_add_cli_args(service_account_token_add_parser)

    service_account_token_delete_parser = service_account_token_subparsers.add_parser("delete", help="Delete a Grafana service-account token.", **subparser_kwargs(SERVICE_ACCOUNT_TOKEN_DELETE_HELP_EXAMPLES))
    add_common_cli_args(service_account_token_delete_parser)
    add_service_account_token_delete_cli_args(service_account_token_delete_parser)
    return parser


def add_common_cli_args(
    parser,
    allow_token_auth=True,
    username_dest="username",
    password_dest="password",
    include_org_id=True,
):
    """Add common cli args implementation."""
    auth_group = parser.add_argument_group("Authentication Options")
    transport_group = parser.add_argument_group("Transport Options")
    auth_group.add_argument(
        "--url",
        default=DEFAULT_URL,
        help="Grafana base URL (default: %s)" % DEFAULT_URL,
    )
    if allow_token_auth:
        auth_group.add_argument(
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
        auth_group.add_argument(
            "--prompt-token",
            action="store_true",
            help=(
                "Prompt for the Grafana API token without echo instead of "
                "passing --token on the command line."
            ),
        )
    auth_group.add_argument(
        "--basic-user",
        dest=username_dest,
        default=None,
        metavar="USERNAME",
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    auth_group.add_argument(
        "--basic-password",
        dest=password_dest,
        default=None,
        metavar="PASSWORD",
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
        ),
    )
    auth_group.add_argument(
        "--prompt-password",
        action="store_true",
        help=(
            "Prompt for the Grafana Basic auth password without echo instead of "
            "passing --basic-password on the command line."
        ),
    )
    if include_org_id:
        auth_group.add_argument(
            "--org-id",
            default=None,
            help="Grafana organization id to send through X-Grafana-Org-Id.",
        )
    transport_group.add_argument(
        "--timeout",
        type=positive_int,
        default=DEFAULT_TIMEOUT,
        help="HTTP timeout in seconds (default: %s)." % DEFAULT_TIMEOUT,
    )
    transport_group.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    transport_group.add_argument(
        "--insecure",
        action="store_true",
        help="Disable TLS certificate verification explicitly.",
    )
    transport_group.add_argument(
        "--ca-cert",
        default=None,
        metavar="PATH",
        help="PEM bundle file to trust for Grafana TLS verification.",
    )


def add_user_list_cli_args(parser):
    """Add user list cli args implementation."""
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
    """Add user add cli args implementation."""
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
    """Add user modify cli args implementation."""
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
    """Add user delete cli args implementation."""
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
    """Add service account list cli args implementation."""
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


def add_org_list_cli_args(parser):
    """Add org list cli args implementation."""
    parser.add_argument(
        "--org-id",
        default=None,
        help="Filter to one exact organization id.",
    )
    parser.add_argument(
        "--name",
        default=None,
        help="Filter to one exact organization name.",
    )
    parser.add_argument(
        "--query",
        default=None,
        help="Case-insensitive substring match against organization name.",
    )
    parser.add_argument(
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
    """Add org add cli args implementation."""
    parser.add_argument(
        "--name",
        required=True,
        help="Organization name to create.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the created organization as JSON.",
    )


def add_org_modify_cli_args(parser):
    """Add org modify cli args implementation."""
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--org-id",
        dest="target_org_id",
        default=None,
        help="Rename the organization identified by this Grafana organization id.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the organization by exact name before renaming it.",
    )
    parser.add_argument(
        "--set-name",
        required=True,
        help="Set a new organization name for the target org.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the modified organization as JSON.",
    )


def add_org_delete_cli_args(parser):
    """Add org delete cli args implementation."""
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--org-id",
        dest="target_org_id",
        default=None,
        help="Delete the organization identified by this Grafana organization id.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the organization by exact name before deleting it.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm that the target organization should be deleted.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the deleted organization summary as JSON.",
    )


def add_org_export_cli_args(parser):
    """Add org export cli args implementation."""
    parser.add_argument(
        "--org-id",
        default=None,
        help="Filter export to one exact organization id.",
    )
    parser.add_argument(
        "--name",
        default=None,
        help="Filter export to one exact organization name.",
    )
    parser.add_argument(
        "--with-users",
        action="store_true",
        help="Include organization users and org roles in the export bundle.",
    )


def add_team_list_cli_args(parser):
    """Add team list cli args implementation."""
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
    """Add team modify cli args implementation."""
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
    """Add team add cli args implementation."""
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
    """Add service account add cli args implementation."""
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
    """Add service account token add cli args implementation."""
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
    """Access export filename implementation."""
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
    """Parse args implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1383, 1399, 376

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

    if argv == ["org"]:
        parser._subparsers._group_actions[0].choices["org"].print_help()
        raise SystemExit(0)

    if argv == ["service-account"]:
        parser._subparsers._group_actions[0].choices["service-account"].print_help()
        raise SystemExit(0)

    if argv == ["service-account", "token"]:
        parser._subparsers._group_actions[0].choices["service-account"]._subparsers._group_actions[0].choices["token"].print_help()
        raise SystemExit(0)

    args = parser.parse_args(argv)
    _normalize_output_format_args(args, parser)
    _validate_tls_args(args, parser)
    return args


def _normalize_output_format_args(args, parser):
    """Internal helper for normalize output format args."""
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


def _validate_tls_args(args, parser):
    """Internal helper for validate tls args."""
    if bool(getattr(args, "verify_ssl", False)) and bool(
        getattr(args, "insecure", False)
    ):
        parser.error("--verify-ssl cannot be combined with --insecure.")
    if getattr(args, "ca_cert", None) and bool(getattr(args, "insecure", False)):
        parser.error("--ca-cert cannot be combined with --insecure.")
