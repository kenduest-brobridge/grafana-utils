#!/usr/bin/env python3
"""List Grafana users and manage service accounts through Grafana APIs.

Initial scope:
- `grafana-access-utils user list`
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
        description="List Grafana users and manage Grafana service accounts."
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


def add_common_cli_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--url",
        default=DEFAULT_URL,
        help="Grafana base URL (default: %s)" % DEFAULT_URL,
    )
    parser.add_argument(
        "--token",
        "--api-token",
        dest="api_token",
        default=None,
        help=(
            "Grafana API token. Preferred flag: --token. "
            "Falls back to GRAFANA_API_TOKEN."
        ),
    )
    parser.add_argument(
        "--basic-user",
        "--username",
        dest="username",
        default=None,
        help=(
            "Grafana Basic auth username. Preferred flag: --basic-user. "
            "Falls back to GRAFANA_USERNAME."
        ),
    )
    parser.add_argument(
        "--basic-password",
        "--password",
        dest="password",
        default=None,
        help=(
            "Grafana Basic auth password. Preferred flag: --basic-password. "
            "Falls back to GRAFANA_PASSWORD."
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

    if cli_token and (cli_username or cli_password):
        raise GrafanaError(
            "Choose either token auth (--token / --api-token) or Basic auth "
            "(--basic-user / --username with --basic-password / --password), not both."
        )
    if cli_username and not cli_password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )
    if cli_password and not cli_username:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )

    token = cli_token or env_value("GRAFANA_API_TOKEN")
    if token:
        headers = {"Authorization": "Bearer %s" % token}
        return headers, "token"

    username = cli_username or env_value("GRAFANA_USERNAME")
    password = cli_password or env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(
            ("%s:%s" % (username, password)).encode("utf-8")
        ).decode("ascii")
        headers = {"Authorization": "Basic %s" % encoded}
        return headers, "basic"
    if username or password:
        raise GrafanaError(
            "Basic auth requires both --basic-user / --username and "
            "--basic-password / --password."
        )

    raise GrafanaError(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN "
        "or --basic-user and --basic-password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
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
