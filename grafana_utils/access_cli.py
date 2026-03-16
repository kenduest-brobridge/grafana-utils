#!/usr/bin/env python3
"""Stable facade for the Python access-management CLI.

Purpose:
- Provide the command-line face for access operations and route parsed inputs to
  workflow orchestration with an authenticated access client.

Architecture:
- This module is intentionally thin and keeps responsibility boundaries clear:
  parser/auth definitions are imported from `access.parser`, runtime orchestration
  lives in `access.workflows`.
- The only meaningful behavior here is request-auth resolution and command
  delegation, which keeps unified CLI changes isolated from access-specific logic.

Caveats:
- Keep only parser/auth glue and dispatch here; access business logic belongs to
  `access/workflows.py` and model helpers.
- `main()` is the expected error-handling boundary for CLI exit codes.
"""

import getpass
import sys
from pathlib import Path

from .access.common import GrafanaError
from .access.models import (
    build_team_rows,
    build_user_rows,
    render_team_json,
    render_user_json,
)
from .access.parser import (
    DEFAULT_SCOPE,
    DEFAULT_SERVICE_ACCOUNT_ROLE,
    DEFAULT_ACCESS_ORG_EXPORT_DIR,
    DEFAULT_TIMEOUT,
    DEFAULT_URL,
    LIST_OUTPUT_FORMAT_CHOICES,
    SCOPE_CHOICES,
    add_common_cli_args,
    add_list_output_format_arg,
    add_service_account_add_cli_args,
    add_service_account_list_cli_args,
    add_service_account_token_add_cli_args,
    add_org_add_cli_args,
    add_org_delete_cli_args,
    add_org_export_cli_args,
    add_org_list_cli_args,
    add_org_modify_cli_args,
    add_team_add_cli_args,
    add_team_list_cli_args,
    add_team_modify_cli_args,
    add_user_add_cli_args,
    add_user_delete_cli_args,
    add_user_list_cli_args,
    add_user_modify_cli_args,
    bool_choice,
    build_parser,
    parse_args,
    positive_int,
)
from .access.workflows import (
    add_service_account_token_with_client,
    add_service_account_with_client,
    add_org_with_client,
    add_team_with_client,
    add_user_with_client,
    apply_team_membership_changes,
    delete_service_account_token_with_client,
    delete_service_account_with_client,
    delete_org_with_client,
    delete_team_with_client,
    delete_user_with_client,
    dispatch_access_command,
    diff_teams_with_client,
    diff_users_with_client,
    export_orgs_with_client,
    export_teams_with_client,
    export_users_with_client,
    format_deleted_service_account_summary_line,
    format_deleted_service_account_token_summary_line,
    format_deleted_team_summary_line,
    format_user_summary_line,
    import_teams_with_client,
    import_users_with_client,
    import_orgs_with_client,
    list_service_accounts_with_client,
    list_orgs_with_client,
    list_teams_with_client,
    list_users_with_client,
    lookup_organization,
    lookup_global_user_by_identity,
    lookup_org_user_by_identity,
    lookup_org_user_by_user_id,
    lookup_service_account_id_by_name,
    lookup_team_by_name,
    modify_org_with_client,
    modify_team_with_client,
    modify_user_with_client,
    normalize_created_user,
    normalize_deleted_user,
    normalize_identity_list,
    normalize_modified_user,
    service_account_role_to_api,
    _sync_team_members_for_import,
    diff_service_accounts_with_client,
    export_service_accounts_with_client,
    import_service_accounts_with_client,
    team_member_admin_state,
    validate_conflicting_identity_sets,
    validate_service_account_delete_auth,
    validate_service_account_token_delete_auth,
    validate_team_delete_auth,
    validate_team_modify_args,
    validate_org_auth,
    validate_user_add_auth,
    validate_user_delete_args,
    validate_user_delete_auth,
    validate_user_list_auth,
    validate_user_modify_args,
    validate_user_modify_auth,
)
from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
from .clients.access_client import GrafanaAccessClient


def resolve_auth(args):
    """Resolve auth in CLI glue and normalize parse-time auth errors.

    Centralized auth decoding keeps dispatch behavior consistent across all access
    commands.
    """
    try:
        return resolve_cli_auth_from_namespace(
            args,
            prompt_reader=getpass.getpass,
            token_prompt_reader=getpass.getpass,
            password_prompt_reader=getpass.getpass,
        )
    except AuthConfigError as exc:
        raise GrafanaError(str(exc))


def build_request_headers(args):
    """Build final auth headers from parsed credentials and prompts."""
    return resolve_auth(args)


def _read_secret_file(path, label):
    """Read password-like secret from file while trimming terminal newline artifacts.

    CR/LF are stripped to avoid false mismatches when secrets are loaded from
    heredoc- or printf-generated files.
    """
    file_path = Path(path)
    try:
        content = file_path.read_text(encoding="utf-8")
    except OSError as exc:
        raise GrafanaError("Failed to read %s file %s: %s" % (label, file_path, exc))
    secret = content.rstrip("\r\n")
    if not secret:
        raise GrafanaError("%s file was empty: %s" % (label, file_path))
    return secret


def resolve_user_secret_inputs(args):
    if getattr(args, "command", None) == "add" and getattr(args, "resource", None) == "user":
        if getattr(args, "new_user_password_file", None):
            args.new_user_password = _read_secret_file(
                args.new_user_password_file,
                "New user password",
            )
        elif bool(getattr(args, "prompt_user_password", False)):
            args.new_user_password = getpass.getpass("New Grafana user password: ")
            if not args.new_user_password:
                raise GrafanaError("Prompted new user password cannot be empty.")
    if getattr(args, "command", None) == "modify" and getattr(args, "resource", None) == "user":
        if getattr(args, "set_password_file", None):
            args.set_password = _read_secret_file(
                args.set_password_file,
                "Set password",
            )
        elif bool(getattr(args, "prompt_set_password", False)):
            args.set_password = getpass.getpass("Updated Grafana user password: ")
            if not args.set_password:
                raise GrafanaError("Prompted set password cannot be empty.")
    return args


def run(args):
    """Build a CLI-scoped client and dispatch the parsed command to access workflows.

    Flow:
    - Resolve transport auth headers.
    - Create a domain client with parsed URL/timeouts.
    - Delegate to `dispatch_access_command` with the parsed auth mode.
    """
    headers, auth_mode = build_request_headers(args)
    client = GrafanaAccessClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=bool(args.verify_ssl or getattr(args, "ca_cert", None)),
        ca_cert=getattr(args, "ca_cert", None),
    )
    return dispatch_access_command(args, client, auth_mode)


def main(argv=None):
    """Run access CLI through parser -> auth -> workflow dispatch and normalize exits."""
    try:
        args = parse_args(argv)
        args = resolve_user_secret_inputs(args)
        return run(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
