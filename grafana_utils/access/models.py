"""Access-management row normalization and rendering helpers."""

import argparse
import csv
import json
import sys
from typing import Any, Optional

from .common import (
    DEFAULT_PAGE_SIZE,
    OUTPUT_FIELDS,
    SERVICE_ACCOUNT_OUTPUT_FIELDS,
    SERVICE_ACCOUNT_TOKEN_OUTPUT_FIELDS,
    TEAM_OUTPUT_FIELDS,
)


def normalize_org_role(value: Any) -> str:
    normalized = str(value or "").strip()
    if not normalized:
        return ""
    lowered = normalized.lower()
    if lowered in {"none", "nobasicrole"}:
        return "None"
    if lowered == "editor":
        return "Editor"
    if lowered == "viewer":
        return "Viewer"
    if lowered == "admin":
        return "Admin"
    return normalized


def normalize_bool(value: Any) -> Optional[bool]:
    if value is None:
        return None
    if isinstance(value, bool):
        return value
    text = str(value).strip().lower()
    if text in {"true", "1", "yes"}:
        return True
    if text in {"false", "0", "no"}:
        return False
    return None


def bool_label(value: Optional[bool]) -> str:
    if value is True:
        return "true"
    if value is False:
        return "false"
    return ""


def normalize_team(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": str(item.get("id") or ""),
        "name": str(item.get("name") or ""),
        "email": str(item.get("email") or ""),
        "memberCount": str(item.get("memberCount") or 0),
        "members": [],
    }


def normalize_org_user(item: dict[str, Any]) -> dict[str, Any]:
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


def normalize_global_user(item: dict[str, Any]) -> dict[str, Any]:
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


def normalize_service_account(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": str(item.get("id") or ""),
        "name": str(item.get("name") or ""),
        "login": str(item.get("login") or ""),
        "role": normalize_org_role(item.get("role")),
        "disabled": normalize_bool(item.get("isDisabled")),
        "tokens": str(item.get("tokens") or 0),
        "orgId": str(item.get("orgId") or ""),
    }


def user_matches_filters(user: dict[str, Any], args: argparse.Namespace) -> bool:
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
    users: list[dict[str, Any]],
    page: int,
    per_page: int,
) -> list[dict[str, Any]]:
    start = (page - 1) * per_page
    end = start + per_page
    return users[start:end]


def attach_team_memberships(
    users: list[dict[str, Any]],
    client: Any,
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
    client: Any,
    args: argparse.Namespace,
) -> list[dict[str, Any]]:
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


def serialize_user_row(user: dict[str, Any]) -> dict[str, Any]:
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


def render_user_json(users: list[dict[str, Any]]) -> str:
    payload = [serialize_user_row(user) for user in users]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_user_csv(users: list[dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=OUTPUT_FIELDS)
    writer.writeheader()
    for user in users:
        row = serialize_user_row(user)
        row["teams"] = ",".join(row["teams"])
        writer.writerow(row)


def render_user_table(users: list[dict[str, Any]]) -> list[str]:
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

    def build_row(values: dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field]) for field in OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join("-" * widths[field] for field in OUTPUT_FIELDS)
    return [header_row, separator_row] + [build_row(row) for row in rows]


def service_account_matches_query(
    service_account: dict[str, Any],
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


def team_matches_filters(team: dict[str, Any], args: argparse.Namespace) -> bool:
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
    teams: list[dict[str, Any]],
    page: int,
    per_page: int,
) -> list[dict[str, Any]]:
    start = (page - 1) * per_page
    end = start + per_page
    return teams[start:end]


def attach_team_members(
    teams: list[dict[str, Any]],
    client: Any,
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


def build_team_rows(
    client: Any,
    args: argparse.Namespace,
) -> list[dict[str, Any]]:
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


def serialize_team_row(team: dict[str, Any]) -> dict[str, Any]:
    row = {}
    for field in TEAM_OUTPUT_FIELDS:
        value = team.get(field)
        if field == "members":
            row[field] = list(value or [])
        else:
            row[field] = str(value or "")
    return row


def render_team_json(teams: list[dict[str, Any]]) -> str:
    payload = [serialize_team_row(team) for team in teams]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_team_csv(teams: list[dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=TEAM_OUTPUT_FIELDS)
    writer.writeheader()
    for team in teams:
        row = serialize_team_row(team)
        row["members"] = ",".join(row["members"])
        writer.writerow(row)


def render_team_table(teams: list[dict[str, Any]]) -> list[str]:
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

    def build_row(values: dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field])
            for field in TEAM_OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join("-" * widths[field] for field in TEAM_OUTPUT_FIELDS)
    return [header_row, separator_row] + [build_row(row) for row in rows]


def format_team_summary_line(team: dict[str, Any]) -> str:
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


def format_team_modify_summary_line(payload: dict[str, Any]) -> str:
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


def format_team_add_summary_line(payload: dict[str, Any]) -> str:
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


def serialize_service_account_row(
    service_account: dict[str, Any],
) -> dict[str, Any]:
    row = {}
    for field in SERVICE_ACCOUNT_OUTPUT_FIELDS:
        value = service_account.get(field)
        if field == "disabled":
            row[field] = bool_label(normalize_bool(value))
        else:
            row[field] = str(value or "")
    return row


def render_service_account_json(service_accounts: list[dict[str, Any]]) -> str:
    payload = [
        serialize_service_account_row(service_account)
        for service_account in service_accounts
    ]
    return json.dumps(payload, indent=2, ensure_ascii=False)


def render_service_account_csv(service_accounts: list[dict[str, Any]]) -> None:
    writer = csv.DictWriter(sys.stdout, fieldnames=SERVICE_ACCOUNT_OUTPUT_FIELDS)
    writer.writeheader()
    for service_account in service_accounts:
        writer.writerow(serialize_service_account_row(service_account))


def render_service_account_table(
    service_accounts: list[dict[str, Any]],
) -> list[str]:
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

    def build_row(values: dict[str, Any]) -> str:
        return "  ".join(
            str(values.get(field) or "").ljust(widths[field])
            for field in SERVICE_ACCOUNT_OUTPUT_FIELDS
        )

    header_row = build_row(headers)
    separator_row = "  ".join(
        "-" * widths[field] for field in SERVICE_ACCOUNT_OUTPUT_FIELDS
    )
    return [header_row, separator_row] + [build_row(row) for row in rows]


def format_service_account_summary_line(service_account: dict[str, Any]) -> str:
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


def serialize_service_account_token_row(payload: dict[str, Any]) -> dict[str, Any]:
    return {
        "serviceAccountId": str(payload.get("serviceAccountId") or ""),
        "name": str(payload.get("name") or ""),
        "secondsToLive": str(payload.get("secondsToLive") or ""),
        "key": str(payload.get("key") or ""),
    }


def render_service_account_token_json(payload: dict[str, Any]) -> str:
    return json.dumps(
        serialize_service_account_token_row(payload),
        indent=2,
        ensure_ascii=False,
    )
