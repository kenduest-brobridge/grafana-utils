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
from .access.parser import (
    parse_args,
)
from .access.workflows import (
    dispatch_access_command,
)
from .auth_staging import AuthConfigError, resolve_cli_auth_from_namespace
from .clients.access_client import GrafanaAccessClient


def resolve_auth(args):
    """Resolve auth and convert parse-layer config errors to CLI error type."""
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
    if (
        getattr(args, "command", None) == "add"
        and getattr(args, "resource", None) == "user"
    ):
        if getattr(args, "new_user_password_file", None):
            args.new_user_password = _read_secret_file(
                args.new_user_password_file,
                "New user password",
            )
        elif bool(getattr(args, "prompt_user_password", False)):
            args.new_user_password = getpass.getpass("New Grafana user password: ")
            if not args.new_user_password:
                raise GrafanaError("Prompted new user password cannot be empty.")
    if (
        getattr(args, "command", None) == "modify"
        and getattr(args, "resource", None) == "user"
    ):
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
        verify_ssl=args.verify_ssl,
        transport_name=args.http_transport,
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
