#!/usr/bin/env python3
"""Stable facade for the Python access-management CLI."""

import getpass
import sys

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
    DEFAULT_TIMEOUT,
    DEFAULT_URL,
    LIST_OUTPUT_FORMAT_CHOICES,
    SCOPE_CHOICES,
    add_common_cli_args,
    add_list_output_format_arg,
    add_service_account_add_cli_args,
    add_service_account_list_cli_args,
    add_service_account_token_add_cli_args,
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
    add_team_with_client,
    add_user_with_client,
    apply_team_membership_changes,
    delete_service_account_token_with_client,
    delete_service_account_with_client,
    delete_team_with_client,
    delete_user_with_client,
    dispatch_access_command,
    format_deleted_service_account_summary_line,
    format_deleted_service_account_token_summary_line,
    format_deleted_team_summary_line,
    format_user_summary_line,
    list_service_accounts_with_client,
    list_teams_with_client,
    list_users_with_client,
    lookup_global_user_by_identity,
    lookup_org_user_by_identity,
    lookup_org_user_by_user_id,
    lookup_service_account_id_by_name,
    lookup_team_by_name,
    modify_team_with_client,
    modify_user_with_client,
    normalize_created_user,
    normalize_deleted_user,
    normalize_identity_list,
    normalize_modified_user,
    service_account_role_to_api,
    team_member_admin_state,
    validate_conflicting_identity_sets,
    validate_service_account_delete_auth,
    validate_service_account_token_delete_auth,
    validate_team_delete_auth,
    validate_team_modify_args,
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
    return resolve_auth(args)


def run(args):
    headers, auth_mode = build_request_headers(args)
    client = GrafanaAccessClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )
    return dispatch_access_command(args, client, auth_mode)


def main(argv=None):
    try:
        args = parse_args(argv)
        return run(args)
    except GrafanaError as exc:
        print("Error: %s" % exc, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
