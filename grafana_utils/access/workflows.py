"""Workflow and helper logic for the Python access-management CLI."""

import json
from pathlib import Path
from typing import Any, Optional

from .common import (
    DEFAULT_PAGE_SIZE,
    GrafanaError,
)
from .parser import (
    ACCESS_EXPORT_KIND_ORGS,
    ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS,
    ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_ORG_EXPORT_FILENAME,
    ACCESS_EXPORT_VERSION,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME,
    ACCESS_USER_EXPORT_FILENAME,
)
from .models import (
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
from .pending_cli_staging import (
    resolve_service_account_id,
    resolve_service_account_token_record,
    resolve_team_id,
    validate_destructive_confirmed,
)


def validate_user_list_auth(args, auth_mode):
    """Validate auth mode before listing users.

    Global scope and team-augmented list flows require Basic auth in Grafana paths.
    """
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User list with --scope global does not support API token auth. Use "
            "Grafana username/password login (--basic-user / --basic-password)."
        )
    if args.with_teams and auth_mode != "basic":
        raise GrafanaError(
            "--with-teams does not support API token auth. Use Grafana "
            "username/password login."
        )


def validate_org_auth(auth_mode):
    """Validate org auth for operations where API-token auth is not guaranteed."""
    if auth_mode != "basic":
        raise GrafanaError(
            "Organization commands do not support API token auth. Use Grafana "
            "username/password login (--basic-user / --basic-password)."
        )


def validate_user_add_auth(auth_mode):
    """Require Basic auth for user creation, where session-style checks may apply."""
    if auth_mode != "basic":
        raise GrafanaError(
            "User add does not support API token auth. Use Grafana "
            "username/password login (--basic-user / --basic-password)."
        )


def validate_user_modify_args(args):
    """Validate user modify args implementation."""
    password_inputs = [
        bool(args.set_password),
        bool(getattr(args, "set_password_file", None)),
        bool(getattr(args, "prompt_set_password", False)),
    ]
    if sum(1 for enabled in password_inputs if enabled) > 1:
        raise GrafanaError(
            "Choose only one of --set-password, --set-password-file, or "
            "--prompt-set-password."
        )
    if not (
        args.set_login
        or args.set_email
        or args.set_name
        or args.set_password
        or getattr(args, "set_password_file", None)
        or getattr(args, "prompt_set_password", False)
        or args.set_org_role
        or args.set_grafana_admin is not None
    ):
        raise GrafanaError(
            "User modify requires at least one of --set-login, --set-email, "
            "--set-name, --set-password, --set-password-file, "
            "--prompt-set-password, --set-org-role, or --set-grafana-admin."
        )


def validate_user_modify_auth(auth_mode):
    """Require Basic auth for user mutation flows (metadata and credentials)."""
    if auth_mode != "basic":
        raise GrafanaError(
            "User modify does not support API token auth. Use Grafana "
            "username/password login (--basic-user / --basic-password)."
        )


def validate_user_delete_args(args):
    """Guard destructive user-delete by requiring explicit `--yes` confirmation."""
    if not args.yes:
        raise GrafanaError("User delete requires --yes.")


def validate_user_delete_auth(args, auth_mode):
    """Validate user-delete auth, with explicit handling for global scope."""
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User delete with --scope global does not support API token auth. Use "
            "Grafana username/password login (--basic-user / --basic-password)."
        )


def validate_team_modify_args(args):
    """Require at least one team membership/admin mutation flag."""
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


def validate_team_delete_auth(_auth_mode):
    """Validate team delete auth constraints for current supported versions."""
    return None


def validate_service_account_delete_auth(_auth_mode):
    """Validate service-account delete auth constraints."""
    return None


def validate_service_account_token_delete_auth(_auth_mode):
    """Validate service-account token delete auth constraints."""
    return None


def service_account_role_to_api(value):
    """Service account role to api implementation."""
    normalized = normalize_org_role(value)
    if normalized == "None":
        return "NoBasicRole"
    return normalized


def _normalize_org_user_record(record):
    """Internal helper for normalize org user record."""
    return {
        "userId": str(
            record.get("userId")
            or record.get("id")
            or record.get("user")
            or ""
        ),
        "login": str(record.get("login") or ""),
        "email": str(record.get("email") or ""),
        "name": str(record.get("name") or ""),
        "orgRole": normalize_org_role(
            record.get("orgRole") or record.get("role") or ""
        ),
    }


def _normalize_org_record(record):
    """Internal helper for normalize org record."""
    users = []
    for item in record.get("users") or []:
        if not isinstance(item, dict):
            continue
        users.append(_normalize_org_user_record(item))
    users.sort(
        key=lambda item: (
            item.get("login") or "",
            item.get("email") or "",
            item.get("userId") or "",
        )
    )
    return {
        "id": str(record.get("id") or record.get("orgId") or ""),
        "name": str(record.get("name") or ""),
        "users": users,
        "userCount": str(record.get("userCount") or len(users)),
    }


def normalize_created_user(user_id, args):
    """Normalize created user implementation."""
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


def _build_access_export_metadata(source_url, kind, source_count, source_dir):
    """Internal helper for build access export metadata."""
    return {
        "kind": kind,
        "version": ACCESS_EXPORT_VERSION,
        "sourceUrl": source_url,
        "recordCount": source_count,
        "sourceDir": source_dir,
    }


def _build_user_export_records(client, args):
    """Internal helper for build user export records."""
    users = []
    if args.scope == "global":
        raw_users = client.iter_global_users(DEFAULT_PAGE_SIZE)
        users = [normalize_global_user(item) for item in raw_users]
    else:
        raw_users = client.list_org_users()
        users = [normalize_org_user(item) for item in raw_users]
    if bool(getattr(args, "with_teams", False)):
        for user in users:
            user_id = user.get("id")
            if not user_id:
                continue
            team_names = []
            for team in client.list_user_teams(user_id):
                team_name = str(team.get("name") or "").strip()
                if team_name:
                    team_names.append(team_name)
            user["teams"] = sorted(team_names)
    return users


def _build_team_export_records(client, args):
    """Internal helper for build team export records."""
    raw_teams = client.iter_teams(query=None, page_size=DEFAULT_PAGE_SIZE)
    teams = []
    for raw_team in raw_teams:
        team = {
            "id": str(raw_team.get("id") or ""),
            "name": str(raw_team.get("name") or ""),
            "email": str(raw_team.get("email") or ""),
            "memberCount": str(raw_team.get("memberCount") or 0),
            "members": [],
            "admins": [],
        }
        if bool(getattr(args, "with_members", False)):
            raw_members = client.list_team_members(team["id"])
            for member in raw_members:
                identity = extract_member_identity(member)
                if not identity:
                    continue
                if identity in team["members"]:
                    if team_member_admin_state(member) is True:
                        if identity not in team["admins"]:
                            team["admins"].append(identity)
                    continue
                team["members"].append(identity)
                if team_member_admin_state(member) is True:
                    team["admins"].append(identity)
        teams.append(team)
    teams.sort(key=lambda item: (item.get("name") or "", item.get("id") or ""))
    return teams


def _build_org_export_records(client, args):
    """Internal helper for build org export records."""
    records = []
    for item in client.list_organizations():
        org = _normalize_org_record(item)
        target_org_id = str(
            getattr(args, "org_id", getattr(args, "target_org_id", "")) or ""
        ).strip()
        target_name = str(getattr(args, "name", "") or "").strip()
        if target_org_id and org.get("id") != target_org_id:
            continue
        if target_name and org.get("name") != target_name:
            continue
        if bool(getattr(args, "with_users", False)):
            org["users"] = [
                _normalize_org_user_record(member)
                for member in client.list_organization_users(org.get("id") or "")
            ]
            org["userCount"] = str(len(org["users"]))
        records.append(org)
    records.sort(key=lambda item: (item.get("name") or "", item.get("id") or ""))
    return records


def _normalize_user_record(record):
    """Internal helper for normalize user record."""
    return {
        "id": str(record.get("id") or ""),
        "login": str(record.get("login") or ""),
        "email": str(record.get("email") or ""),
        "name": str(record.get("name") or ""),
        "orgRole": normalize_org_role(record.get("orgRole") or ""),
        "grafanaAdmin": normalize_bool(record.get("grafanaAdmin")),
        "teams": _normalize_access_identity_list(record.get("teams") or []),
    }


def _normalize_team_record(record):
    """Internal helper for normalize team record."""
    return {
        "id": str(record.get("id") or ""),
        "name": str(record.get("name") or ""),
        "email": str(record.get("email") or ""),
        "memberCount": str(record.get("memberCount") or 0),
        "members": _normalize_access_identity_list(record.get("members") or []),
        "admins": _normalize_access_identity_list(record.get("admins") or []),
    }


def _normalize_user_for_diff(record):
    """Internal helper for normalize user for diff."""
    return {
        "login": str(record.get("login") or ""),
        "email": str(record.get("email") or ""),
        "name": str(record.get("name") or ""),
        "orgRole": normalize_org_role(
            record.get("orgRole") or record.get("role") or ""
        ),
        "grafanaAdmin": normalize_bool(
            record.get("grafanaAdmin")
            if record.get("grafanaAdmin") is not None
            else record.get("isGrafanaAdmin")
            if record.get("isGrafanaAdmin") is not None
            else record.get("isAdmin")
        ),
        "teams": sorted(_normalize_access_identity_list(record.get("teams") or [])),
    }


def _normalize_team_for_diff(record, include_members=False):
    """Internal helper for normalize team for diff."""
    payload = {
        "name": str(record.get("name") or ""),
        "email": str(record.get("email") or ""),
    }
    if include_members:
        payload["members"] = sorted(
            _normalize_access_identity_list(record.get("members") or [])
        )
        payload["admins"] = sorted(
            _normalize_access_identity_list(record.get("admins") or [])
        )
    else:
        payload["members"] = []
        payload["admins"] = []
    return payload


def _build_user_diff_map(records, source):
    """Internal helper for build user diff map."""
    # Call graph: see callers/callees.
    #   Upstream callers: 493
    #   Downstream callees: 353, 818, 939

    indexed = {}
    for record in records:
        key = _normalize_access_import_identity(_resolve_access_user_key(record))
        if not key:
            raise GrafanaError(
                "User diff record in %s does not include login or email." % source
            )
        if key in indexed:
            raise GrafanaError("Duplicate user identity in %s: %s" % (source, key))
        indexed[key] = {
            "identity": _resolve_access_user_key(record),
            "payload": _normalize_user_for_diff(record),
        }
    return indexed


def _build_team_diff_map(records, source, include_members=False):
    """Internal helper for build team diff map."""
    indexed = {}
    for record in records:
        team_name = str(record.get("name") or "").strip()
        if not team_name:
            raise GrafanaError(
                "Team diff record in %s does not include name." % source
            )
        key = _normalize_access_import_identity(team_name)
        if key in indexed:
            raise GrafanaError("Duplicate team name in %s: %s" % (source, team_name))
        indexed[key] = {
            "identity": team_name,
            "payload": _normalize_team_for_diff(record, include_members),
        }
    return indexed


def _record_diff_fields(left, right):
    """Internal helper for record diff fields."""
    keys = set(left.keys()) | set(right.keys())
    changed = []
    for key in sorted(keys):
        if left.get(key) != right.get(key):
            changed.append(key)
    return changed


def _build_user_export_for_diff_records(client, scope, include_teams):
    """Internal helper for build user export for diff records."""
    raw = (
        client.iter_global_users(DEFAULT_PAGE_SIZE)
        if scope == "global"
        else client.list_org_users()
    )
    records = []
    for item in raw:
        record = (
            normalize_global_user(item)
            if scope == "global"
            else normalize_org_user(item)
        )
        if include_teams:
            teams = []
            user_id = record.get("id")
            if user_id:
                for team in client.list_user_teams(user_id):
                    team_name = str(team.get("name") or "").strip()
                    if team_name:
                        teams.append(team_name)
            record["teams"] = sorted(teams)
        records.append(record)
    return records


def _build_team_export_for_diff_records(client, include_members):
    """Internal helper for build team export for diff records."""
    # Call graph: see callers/callees.
    #   Upstream callers: 555
    #   Downstream callees: 2189, 2217, 373

    records = client.iter_teams(query=None, page_size=DEFAULT_PAGE_SIZE)
    if not include_members:
        return [_normalize_team_for_diff(raw_team, False) for raw_team in records]
    normalized = []
    for raw_team in records:
        team_record = {
            "name": str(raw_team.get("name") or ""),
            "email": str(raw_team.get("email") or ""),
            "members": [],
            "admins": [],
        }
        team_id = str(raw_team.get("id") or "")
        if team_id:
            for member in client.list_team_members(team_id):
                identity = extract_member_identity(member)
                if not identity:
                    continue
                if identity not in team_record["members"]:
                    team_record["members"].append(identity)
                if team_member_admin_state(member) is True and identity not in team_record["admins"]:
                    team_record["admins"].append(identity)
        normalized.append(team_record)
    return [ _normalize_team_for_diff(item, True) for item in normalized ]


def diff_users_with_client(args, client):
    """Diff users with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 328, 392, 429, 439, 896

    local_records = [
        _normalize_user_record(item)
        for item in _load_access_import_bundle(
            args.diff_dir,
            ACCESS_USER_EXPORT_FILENAME,
            ACCESS_EXPORT_KIND_USERS,
        )["records"]
    ]
    include_teams = any(
        bool(record.get("teams")) for record in local_records
    )
    local_map = _build_user_diff_map(local_records, args.diff_dir)

    live_map = _build_user_diff_map(
        _build_user_export_for_diff_records(
            client,
            args.scope,
            include_teams,
        ),
        "Grafana live users",
    )
    differences = 0
    checked = 0

    for key in sorted(local_map.keys()):
        checked += 1
        local_identity = local_map[key]["identity"]
        local_payload = local_map[key]["payload"]
        if key not in live_map:
            print("Diff missing-live user %s" % local_identity)
            differences += 1
            continue
        live_payload = live_map[key]["payload"]
        changed = _record_diff_fields(local_payload, live_payload)
        if changed:
            differences += 1
            print(
                "Diff different user %s fields=%s"
                % (local_identity, ",".join(changed))
            )
        else:
            print("Diff same user %s" % local_identity)

    for key in sorted(live_map.keys()):
        if key in local_map:
            continue
        differences += 1
        print("Diff extra-live user %s" % live_map[key]["identity"])
        checked += 1

    if differences:
        print(
            "Diff checked %s user(s); %s difference(s) found."
            % (checked, differences)
        )
    else:
        print("No user differences across %s user(s)." % checked)
    return differences


def diff_teams_with_client(args, client):
    """Diff teams with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 341, 373, 410, 429, 466, 818, 896

    local_records = [
        _normalize_team_record(item)
        for item in _load_access_import_bundle(
            args.diff_dir,
            ACCESS_TEAM_EXPORT_FILENAME,
            ACCESS_EXPORT_KIND_TEAMS,
        )["records"]
    ]
    include_members = any(
        bool(item.get("members") or item.get("admins")) for item in local_records
    )
    if include_members:
        local_records = [_normalize_team_for_diff(item, True) for item in local_records]
    local_map = _build_team_diff_map(
        local_records,
        args.diff_dir,
        include_members=include_members,
    )
    live_records = _build_team_export_for_diff_records(client, include_members)
    live_map = {}
    for item in live_records:
        team_name = str(item.get("name") or "").strip()
        if not team_name:
            continue
        key = _normalize_access_import_identity(team_name)
        live_map[key] = {
            "identity": team_name,
            "payload": _normalize_team_for_diff(item, include_members),
        }
    differences = 0
    checked = 0

    for key in sorted(local_map.keys()):
        checked += 1
        local_identity = local_map[key]["identity"]
        local_payload = local_map[key]["payload"]
        if key not in live_map:
            print("Diff missing-live team %s" % local_identity)
            differences += 1
            continue
        changed = _record_diff_fields(local_payload, live_map[key]["payload"])
        if changed:
            differences += 1
            print(
                "Diff different team %s fields=%s"
                % (local_identity, ",".join(changed))
            )
        else:
            print("Diff same team %s" % local_identity)

    for key in sorted(live_map.keys()):
        if key in local_map:
            continue
        differences += 1
        print("Diff extra-live team %s" % live_map[key]["identity"])
        checked += 1

    if differences:
        print(
            "Diff checked %s team(s); %s difference(s) found."
            % (checked, differences)
        )
    else:
        print("No team differences across %s team(s)." % checked)
    return differences


def _iter_service_accounts(client, page_size=DEFAULT_PAGE_SIZE):
    """Internal helper for iter service accounts."""
    page = 1
    records = []
    while True:
        batch = client.list_service_accounts(
            query=None,
            page=page,
            per_page=page_size,
        )
        if not batch:
            break
        records.extend(batch)
        if len(batch) < page_size:
            break
        page += 1
    return records


def _normalize_service_account_record(record):
    """Internal helper for normalize service account record."""
    return {
        "id": str(record.get("id") or ""),
        "name": str(record.get("name") or ""),
        "login": str(record.get("login") or ""),
        "role": normalize_org_role(record.get("role") or ""),
        "disabled": normalize_bool(
            record.get("disabled")
            if record.get("disabled") is not None
            else record.get("isDisabled")
        ),
        "tokens": str(record.get("tokens") or 0),
        "orgId": str(record.get("orgId") or ""),
    }


def _normalize_service_account_for_diff(record):
    """Internal helper for normalize service account for diff."""
    return {
        "name": str(record.get("name") or ""),
        "role": normalize_org_role(record.get("role") or ""),
        "disabled": normalize_bool(
            record.get("disabled")
            if record.get("disabled") is not None
            else record.get("isDisabled")
        ),
    }


def _build_service_account_diff_map(records, source):
    """Internal helper for build service account diff map."""
    indexed = {}
    for record in records:
        service_account_name = str(record.get("name") or "").strip()
        if not service_account_name:
            raise GrafanaError(
                "Service-account diff record in %s does not include name." % source
            )
        key = _normalize_access_import_identity(service_account_name)
        if key in indexed:
            raise GrafanaError(
                "Duplicate service-account name in %s: %s"
                % (source, service_account_name)
            )
        indexed[key] = {
            "identity": service_account_name,
            "payload": _normalize_service_account_for_diff(record),
        }
    return indexed


def _lookup_service_account_by_name(client, service_account_name):
    """Internal helper for lookup service account by name."""
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
            "Multiple service accounts matched name %s; refine the lookup."
            % service_account_name
        )
    return exact_matches[0]


def diff_service_accounts_with_client(args, client):
    """Diff service accounts with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 429, 624, 643, 673, 896

    bundle = _load_access_import_bundle(
        args.diff_dir,
        ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    )
    local_records = []
    for item in bundle.get("records") or []:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Access import entry in %s must be an object." % bundle["bundle_path"]
            )
        local_records.append(_normalize_service_account_record(item))
    local_map = _build_service_account_diff_map(local_records, bundle["bundle_path"])
    live_records = [
        _normalize_service_account_record(item)
        for item in _iter_service_accounts(client)
    ]
    live_map = _build_service_account_diff_map(
        live_records,
        "Grafana live service accounts",
    )

    differences = 0
    checked = 0
    for key in sorted(local_map.keys()):
        checked += 1
        local_identity = local_map[key]["identity"]
        local_payload = local_map[key]["payload"]
        if key not in live_map:
            print("Diff missing-live service-account %s" % local_identity)
            differences += 1
            continue
        changed = _record_diff_fields(local_payload, live_map[key]["payload"])
        if changed:
            differences += 1
            print(
                "Diff different service-account %s fields=%s"
                % (local_identity, ",".join(changed))
            )
        else:
            print("Diff same service-account %s" % local_identity)

    for key in sorted(live_map.keys()):
        if key in local_map:
            continue
        checked += 1
        differences += 1
        print("Diff extra-live service-account %s" % live_map[key]["identity"])

    if differences:
        print(
            "Diff checked %s service-account(s); %s difference(s) found."
            % (checked, differences)
        )
    else:
        print("No service-account differences across %s service-account(s)." % checked)
    return differences



def _load_json_document(path):
    """Internal helper for load json document."""
    if not path.exists():
        raise GrafanaError("Access export file not found: %s" % path)
    try:
        content = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise GrafanaError("Failed to read access export file %s: %s" % (path, exc))
    try:
        return json.loads(content)
    except ValueError as exc:
        raise GrafanaError("Invalid JSON in access export file %s: %s" % (path, exc))


def _write_json_document(path, payload):
    """Internal helper for write json document."""
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        path.write_text(
            json.dumps(payload, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
    except OSError as exc:
        raise GrafanaError("Failed to write access export file %s: %s" % (path, exc))


def _assert_not_overwriting(export_dir, filenames, dry_run, overwrite):
    """Internal helper for assert not overwriting."""
    if dry_run:
        return
    for filename in filenames:
        if (export_dir / filename).exists() and not overwrite:
            raise GrafanaError(
                "Refusing to overwrite existing file: %s. Use --overwrite."
                % (export_dir / filename)
            )


def _normalize_access_import_identity(value):
    """Internal helper for normalize access import identity."""
    return str(value or "").strip().lower()


def _normalize_access_identity_list(values):
    """Internal helper for normalize access identity list."""
    normalized = []
    seen = set()
    for value in values:
        text = str(value or "").strip()
        if not text:
            continue
        lowered = _normalize_access_import_identity(text)
        if lowered in seen:
            continue
        seen.add(lowered)
        normalized.append(text)
    return normalized


def _build_access_import_preview_row(index, identity, action, detail):
    """Internal helper for build access import preview row."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return {
        "index": str(index),
        "identity": str(identity or ""),
        "action": str(action or ""),
        "detail": str(detail or ""),
    }


def _render_access_import_preview_table(rows):
    """Internal helper for render access import preview table."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    columns = [
        ("INDEX", "index"),
        ("IDENTITY", "identity"),
        ("ACTION", "action"),
        ("DETAIL", "detail"),
    ]
    widths = []
    for header, key in columns:
        widths.append(
            max(
                len(header),
                max((len(str(row.get(key) or "")) for row in rows), default=0),
            )
        )
    header = "  ".join(
        header.ljust(widths[index]) for index, (header, _key) in enumerate(columns)
    )
    separator = "  ".join("-" * width for width in widths)
    lines = [header, separator]
    for row in rows:
        lines.append(
            "  ".join(
                str(row.get(key) or "").ljust(widths[index])
                for index, (_header, key) in enumerate(columns)
            )
        )
    return lines


def _validate_access_import_preview_output(args, resource_label):
    """Internal helper for validate access import preview output."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if (bool(getattr(args, "table", False)) or bool(getattr(args, "json", False))) and not bool(
        getattr(args, "dry_run", False)
    ):
        raise GrafanaError(
            "--table/--json for %s import are only supported with --dry-run."
            % resource_label
        )
    if bool(getattr(args, "table", False)) and bool(getattr(args, "json", False)):
        raise GrafanaError(
            "--table and --json cannot be used together for %s import."
            % resource_label
        )


def _load_access_import_bundle(import_dir, expected_filename, expected_kind):
    """Internal helper for load access import bundle."""
    bundle_path = Path(import_dir) / expected_filename
    metadata_path = Path(import_dir) / ACCESS_EXPORT_METADATA_FILENAME
    raw = _load_json_document(bundle_path)
    if isinstance(raw, list):
        records = raw
        kind = None
        version = None
    elif isinstance(raw, dict):
        records = raw.get("records")
        if records is None:
            raise GrafanaError(
                "Access import bundle is missing a records list: %s" % bundle_path
            )
        kind = raw.get("kind")
        version = raw.get("version")
    else:
        raise GrafanaError("Unsupported access import payload in %s." % bundle_path)
    if not isinstance(records, list):
        raise GrafanaError("Access import records must be a list in %s." % bundle_path)
    if version is not None and version > ACCESS_EXPORT_VERSION:
        raise GrafanaError(
            "Unsupported %s version %s in %s. Supported <= %s."
            % (expected_filename, version, bundle_path, ACCESS_EXPORT_VERSION)
        )
    if expected_kind is not None and kind not in (None, expected_kind):
        raise GrafanaError(
            "Access import kind mismatch for %s: expected %s, got %s."
            % (bundle_path, expected_kind, kind)
        )
    if not metadata_path.exists():
        metadata = None
    else:
        metadata = _load_json_document(metadata_path)
    return {
        "records": records,
        "metadata": metadata,
        "bundle_path": str(bundle_path),
        "kind": kind,
    }


def _resolve_access_user_key(record):
    """Internal helper for resolve access user key."""
    login = str(record.get("login") or "").strip()
    email = str(record.get("email") or "").strip()
    if login:
        return login
    if email:
        return email
    raise GrafanaError("User import record does not include login or email: %s" % record)


def _build_access_user_payload(record):
    """Internal helper for build access user payload."""
    login = str(record.get("login") or "")
    email = str(record.get("email") or "")
    if not login or not email:
        raise GrafanaError(
            "User import record is missing required login/email fields: %s"
            % record
        )
    return {
        "name": str(record.get("name") or ""),
        "email": email,
        "login": login,
    }


def _lookup_team_memberships_by_identity(client, team_id, include_empty=False):
    """Internal helper for lookup team memberships by identity."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1915
    #   Downstream callees: 2189, 2217, 818

    members = {}
    for item in client.list_team_members(team_id):
        identity = extract_member_identity(item)
        if not identity:
            if not include_empty:
                continue
            identity = str(item.get("userId") or item.get("id") or "").strip()
            if not identity:
                continue
        user_id = str(item.get("userId") or item.get("id") or "")
        members[_normalize_access_import_identity(identity)] = {
            "identity": identity,
            "user_id": user_id,
            "admin": team_member_admin_state(item),
        }
    return members


def _merge_team_membership_target(members, admins):
    """Internal helper for merge team membership target."""
    desired_members = _normalize_access_identity_list(members)
    desired_admins = _normalize_access_identity_list(admins)
    desired_all_identities = []
    seen = set()
    for identity in desired_members + desired_admins:
        key = _normalize_access_import_identity(identity)
        if key in seen:
            continue
        seen.add(key)
        desired_all_identities.append(identity)
    return desired_all_identities, desired_members, desired_admins


def _sync_team_members_for_import(
    client,
    team_id,
    team_name,
    existing_members,
    desired_members,
    desired_admins,
    include_missing=False,
    dry_run=False,
):
    """Internal helper for sync team members for import."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1915
    #   Downstream callees: 2057, 818, 823, 986

    target_members = _normalize_access_identity_list(desired_members)
    target_admins = _normalize_access_identity_list(desired_admins)
    target_all, target_members, target_admins = _merge_team_membership_target(
        target_members,
        target_admins,
    )
    target_all_keys = set(
        _normalize_access_import_identity(item) for item in target_all
    )
    target_admin_keys = set(
        _normalize_access_import_identity(item) for item in target_admins
    )
    existing_keys = list(existing_members.keys())
    existing_key_set = set(existing_keys)
    existing_identity_map = {
        key: payload.get("identity") or key
        for key, payload in existing_members.items()
    }

    summary = {
        "addedMembers": [],
        "removedMembers": [],
        "addedAdmins": [],
        "removedAdmins": [],
        "unchangedAdmins": [],
    }

    # Ensure target members exist.
    for identity in target_all:
        lowered = _normalize_access_import_identity(identity)
        if lowered in existing_key_set:
            if lowered in target_admin_keys and lowered in existing_members:
                if existing_members[lowered].get("admin") is True:
                    summary["unchangedAdmins"].append(
                        existing_identity_map.get(lowered, identity)
                    )
            continue
        summary["addedMembers"].append(identity)
        if lowered in target_admin_keys:
            summary["addedAdmins"].append(identity)
        if dry_run:
            continue
        payload = lookup_org_user_by_identity(client, identity)
        member_user_id = str(
            payload.get("userId")
            or payload.get("id")
            or payload.get("user")
            or ""
        )
        if not member_user_id:
            raise GrafanaError(
                "Team member lookup did not return an id: %s" % identity
            )
        client.add_team_member(team_id, member_user_id)

    # Remove memberships that are missing from the import target only in full sync mode.
    remove_members = []
    if include_missing:
        remove_members = [
            existing_key
            for existing_key in existing_members
            if existing_key not in target_all_keys
        ]
    for identity_key in remove_members:
        payload = existing_members.get(identity_key)
        if not payload:
            continue
        user_id = payload.get("user_id")
        if not user_id:
            continue
        if dry_run:
            summary["removedMembers"].append(payload.get("identity") or identity_key)
            continue
        client.remove_team_member(team_id, user_id)
        summary["removedMembers"].append(payload.get("identity") or identity_key)

    # Keep admin-state synchronized using update endpoint when there is meaningful
    # state change. This mirrors existing add/remove membership behavior and keeps
    # API state deterministic.
    existing_admin_keys = set(
        key
        for key, info in existing_members.items()
        if info.get("admin") is True
    )
    for key in target_admin_keys:
        if key in existing_admin_keys:
            continue
        if key in existing_members:
            summary["addedAdmins"].append(existing_identity_map[key])
    for key in existing_admin_keys:
        if key in target_admin_keys:
            continue
        summary["removedAdmins"].append(existing_identity_map.get(key, key))

    regular_payload = [
        raw_identity
        for raw_identity in target_members
        if _normalize_access_import_identity(raw_identity) not in target_admin_keys
    ]
    admin_payload = [raw_identity for raw_identity in target_admins]

    if not dry_run and (
        include_missing
        or summary["addedAdmins"]
        or summary["removedAdmins"]
        or summary["addedMembers"]
        or summary["removedMembers"]
    ):
        client.update_team_members(
            team_id,
            {
                "members": regular_payload,
                "admins": admin_payload,
            },
        )

    return summary


def _build_user_import_records(import_dir):
    """Internal helper for build user import records."""
    return _load_access_import_bundle(
        Path(import_dir),
        ACCESS_USER_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_USERS,
    )


def _build_team_import_records(import_dir):
    """Internal helper for build team import records."""
    return _load_access_import_bundle(
        Path(import_dir),
        ACCESS_TEAM_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_TEAMS,
    )


def _build_org_import_records(import_dir):
    """Internal helper for build org import records."""
    return _load_access_import_bundle(
        Path(import_dir),
        ACCESS_ORG_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_ORGS,
    )


def _build_service_account_import_records(import_dir):
    """Internal helper for build service account import records."""
    return _load_access_import_bundle(
        Path(import_dir),
        ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
        ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    )


def _build_service_account_import_row(index, identity, action, detail):
    """Internal helper for build service account import row."""
    return {
        "index": str(index),
        "identity": str(identity or ""),
        "action": str(action or ""),
        "detail": str(detail or ""),
    }


def _render_service_account_import_table(rows):
    """Internal helper for render service account import table."""
    headers = ["INDEX", "IDENTITY", "ACTION", "DETAIL"]
    widths = [len(header) for header in headers]
    values = []
    for row in rows:
        current = [
            str(row.get("index") or ""),
            str(row.get("identity") or ""),
            str(row.get("action") or ""),
            str(row.get("detail") or ""),
        ]
        values.append(current)
        for idx, value in enumerate(current):
            widths[idx] = max(widths[idx], len(value))

    def _format(items):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(item.ljust(widths[idx]) for idx, item in enumerate(items))

    lines = [_format(headers), _format(["-" * width for width in widths])]
    for row in values:
        lines.append(_format(row))
    return lines


def _emit_service_account_import_dry_run_output(args, rows, summary):
    """Internal helper for emit service account import dry run output."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1730
    #   Downstream callees: 1177

    if args.json:
        print(
            json.dumps(
                {
                    "rows": rows,
                    "summary": summary,
                },
                indent=2,
                ensure_ascii=False,
            )
        )
        return
    if args.table:
        for line in _render_service_account_import_table(rows):
            print(line)
        print("")
        print(
            "Import summary: processed=%s created=%s updated=%s skipped=%s source=%s"
            % (
                summary["processed"],
                summary["created"],
                summary["updated"],
                summary["skipped"],
                summary["source"],
            )
        )


def validate_service_account_import_dry_run_output(args):
    """Validate service account import dry run output implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1730
    #   Downstream callees: 無

    if (args.table or args.json) and not args.dry_run:
        raise GrafanaError(
            "--table/--json for service-account import are only supported with --dry-run."
        )
    if args.table and args.json:
        raise GrafanaError(
            "--table and --json cannot be used together for service-account import."
        )


def export_users_with_client(args, client):
    """Export users with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 238, 249, 794, 806

    export_dir = Path(args.export_dir)
    records = _build_user_export_records(client, args)
    users_path = export_dir / ACCESS_USER_EXPORT_FILENAME
    metadata_path = export_dir / ACCESS_EXPORT_METADATA_FILENAME
    data = {
        "kind": ACCESS_EXPORT_KIND_USERS,
        "version": ACCESS_EXPORT_VERSION,
        "records": records,
    }
    metadata = _build_access_export_metadata(
        source_url=args.url,
        kind=ACCESS_EXPORT_KIND_USERS,
        source_count=len(records),
        source_dir=str(export_dir),
    )
    _assert_not_overwriting(
        export_dir,
        [ACCESS_USER_EXPORT_FILENAME, ACCESS_EXPORT_METADATA_FILENAME],
        dry_run=args.dry_run,
        overwrite=bool(getattr(args, "overwrite", False)),
    )
    if not args.dry_run:
        _write_json_document(users_path, data)
        _write_json_document(metadata_path, metadata)
    action = "Would export" if args.dry_run else "Exported"
    print(
        "%s %s user(s) from %s -> %s and %s"
        % (action, len(records), args.url, users_path, metadata_path)
    )
    return 0


def export_orgs_with_client(args, client):
    """Export orgs with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 238, 304, 794, 806

    export_dir = Path(args.export_dir)
    records = _build_org_export_records(client, args)
    payload_path = export_dir / ACCESS_ORG_EXPORT_FILENAME
    metadata_path = export_dir / ACCESS_EXPORT_METADATA_FILENAME
    data = {
        "kind": ACCESS_EXPORT_KIND_ORGS,
        "version": ACCESS_EXPORT_VERSION,
        "records": records,
    }
    metadata = _build_access_export_metadata(
        source_url=args.url,
        kind=ACCESS_EXPORT_KIND_ORGS,
        source_count=len(records),
        source_dir=str(export_dir),
    )
    _assert_not_overwriting(
        export_dir,
        [ACCESS_ORG_EXPORT_FILENAME, ACCESS_EXPORT_METADATA_FILENAME],
        dry_run=args.dry_run,
        overwrite=bool(getattr(args, "overwrite", False)),
    )
    if not args.dry_run:
        _write_json_document(payload_path, data)
        _write_json_document(metadata_path, metadata)
    action = "Would export" if args.dry_run else "Exported"
    print(
        "%s %s org(s) from %s -> %s and %s"
        % (action, len(records), args.url, payload_path, metadata_path)
    )
    return 0


def _lookup_org_user_record(users, identity):
    """Internal helper for lookup org user record."""
    target = _normalize_access_import_identity(identity)
    if not target:
        return None
    for user in users:
        login = _normalize_access_import_identity(user.get("login"))
        email = _normalize_access_import_identity(user.get("email"))
        if target in (login, email):
            return user
    return None


def _apply_org_user_import(client, org_id, org_name, desired_users, dry_run):
    """Internal helper for apply org user import."""
    existing_users = [
        _normalize_org_user_record(item)
        for item in client.list_organization_users(org_id)
    ]
    changed = False
    for user in desired_users:
        identity = str(user.get("login") or user.get("email") or "").strip()
        if not identity:
            raise GrafanaError(
                "Organization import record for %s has a user without login/email."
                % org_name
            )
        desired_role = normalize_org_role(user.get("orgRole") or "")
        if not desired_role:
            desired_role = "Viewer"
        existing = _lookup_org_user_record(existing_users, identity)
        if existing is None:
            changed = True
            if dry_run:
                print(
                    "Would add user %s to org %s with role %s"
                    % (identity, org_name, desired_role)
                )
            else:
                client.add_user_to_organization(
                    org_id,
                    {
                        "loginOrEmail": identity,
                        "role": desired_role,
                    },
                )
            continue
        existing_role = normalize_org_role(existing.get("orgRole") or "")
        existing_user_id = str(existing.get("userId") or "")
        if desired_role and desired_role != existing_role:
            changed = True
            if dry_run:
                print(
                    "Would update org user %s role in %s -> %s"
                    % (identity, org_name, desired_role)
                )
            else:
                if not existing_user_id:
                    raise GrafanaError(
                        "Organization user lookup did not return an id for %s in %s."
                        % (identity, org_name)
                    )
                client.update_organization_user_role(
                    org_id,
                    existing_user_id,
                    desired_role,
                )
    return changed


def import_orgs_with_client(args, client):
    """Import orgs with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 1149, 1329, 202, 818

    bundle = _build_org_import_records(args.import_dir)
    raw_records = bundle.get("records") or []
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Access import entry in %s must be an object." % bundle["bundle_path"]
            )
        records.append(_normalize_org_record(item))

    live_orgs = [_normalize_org_record(item) for item in client.list_organizations()]
    live_by_name = {}
    live_by_id = {}
    for org in live_orgs:
        if org.get("name"):
            live_by_name[_normalize_access_import_identity(org.get("name"))] = org
        if org.get("id"):
            live_by_id[str(org.get("id"))] = org

    created = 0
    updated = 0
    skipped = 0
    processed = 0
    for index, record in enumerate(records, 1):
        processed += 1
        org_name = str(record.get("name") or "").strip()
        org_id = str(record.get("id") or "").strip()
        record_changed = False
        if not org_name:
            raise GrafanaError(
                "Access org import record %s in %s lacks name."
                % (index, bundle["bundle_path"])
            )
        existing = live_by_name.get(_normalize_access_import_identity(org_name))
        if existing is None and org_id:
            existing = live_by_id.get(org_id)
        count_as_updated = existing is not None

        target_org_id = ""
        if existing is None:
            if not args.replace_existing:
                skipped += 1
                print(
                    "Skipped org %s (%s): missing and --replace-existing was not set."
                    % (org_name, index)
                )
                continue
            if args.dry_run:
                created += 1
                target_org_id = ""
                record_changed = True
                print("Would create org %s" % org_name)
            else:
                created_payload = client.create_organization({"name": org_name})
                target_org_id = str(
                    created_payload.get("orgId") or created_payload.get("id") or ""
                )
                created += 1
                record_changed = True
                print("Created org %s" % org_name)
        else:
            target_org_id = str(existing.get("id") or "")
            if not args.replace_existing:
                skipped += 1
                print("Skipped existing org %s (%s)" % (org_name, index))
                continue
            existing_name = str(existing.get("name") or "")
            if org_id and target_org_id == org_id and existing_name != org_name:
                if args.dry_run:
                    print("Would rename org %s -> %s" % (existing_name, org_name))
                else:
                    client.update_organization(target_org_id, {"name": org_name})
                record_changed = True

        desired_users = record.get("users") or []
        if desired_users:
            if args.dry_run and not target_org_id:
                membership_changed = bool(desired_users)
                for user in desired_users:
                    identity = str(user.get("login") or user.get("email") or "").strip()
                    if not identity:
                        raise GrafanaError(
                            "Organization import record for %s has a user without login/email."
                            % org_name
                        )
                    role = normalize_org_role(user.get("orgRole") or "") or "Viewer"
                    print(
                        "Would add user %s to org %s with role %s"
                        % (identity, org_name, role)
                    )
            else:
                membership_changed = _apply_org_user_import(
                    client,
                    target_org_id,
                    org_name,
                    desired_users,
                    args.dry_run,
                )
            if membership_changed and existing is not None:
                count_as_updated = True
            record_changed = record_changed or membership_changed
        elif existing is not None and target_org_id and not record_changed:
            skipped += 1
            print("Skipped org %s (%s): already matched live state." % (org_name, index))
            continue

        if existing is not None and record_changed:
            if count_as_updated:
                updated += 1
            print("Updated org %s" % org_name)

    print(
        "Import summary: processed=%s created=%s updated=%s skipped=%s source=%s"
        % (processed, created, updated, skipped, args.import_dir)
    )
    return 0


def import_users_with_client(args, client):
    """Import users with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 1131, 2040, 2057, 2079, 328, 818, 823, 939, 950

    bundle = _build_user_import_records(args.import_dir)
    raw_records = bundle.get("records") or []
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError("Access import entry in %s must be an object." % bundle["bundle_path"])
        records.append(_normalize_user_record(item))

    created = []
    updated = []
    skipped = []
    processed = 0
    for index, record in enumerate(records, 1):
        processed += 1
        login = str(record.get("login") or "").strip()
        email = str(record.get("email") or "").strip()
        if not login and not email:
            raise GrafanaError(
                "Access user import record %s in %s lacks login/email."
                % (index, bundle["bundle_path"])
            )

        if args.scope == "global":
            existing = None
            try:
                existing = lookup_global_user_by_identity(
                    client,
                    login=login or None,
                    email=email or None,
                )
            except GrafanaError:
                existing = None
        else:
            try:
                existing = lookup_org_user_by_identity(client, login or email)
            except GrafanaError:
                existing = None

        if existing is None:
            if not args.replace_existing:
                skipped.append(_resolve_access_user_key(record))
                print("Skipped user %s (%s): missing and --replace-existing was not set." % (login or email, index))
                continue
            if args.scope == "org":
                raise GrafanaError(
                    "User import cannot create missing org users by login/email: %s"
                    % (login or email)
                )
            payload = _build_access_user_payload(record)
            payload.setdefault("password", str(record.get("password") or "").strip())
            if not payload.get("password"):
                raise GrafanaError(
                    "Missing password for new user import entry %s: %s"
                    % (index, login or email)
                )
            if args.dry_run:
                created.append(_resolve_access_user_key(record))
                print("Would create user %s" % (login or email))
                continue
            created_payload = client.create_user(payload)
            created.append(
                str(
                    created_payload.get("id")
                    if isinstance(created_payload, dict)
                    else ""
                )
            )
            print("Created user %s" % (login or email))
            continue

        user_id = str(
            existing.get("id")
            or existing.get("userId")
            or record.get("id")
            or ""
        )
        if not user_id:
            raise GrafanaError("User import record %s resolved without id: %s" % (index, record))

        if not args.replace_existing:
            skipped.append(_resolve_access_user_key(record))
            print("Skipped existing user %s (%s)" % (record.get("login") or record.get("email") or "", index))
            continue

        desired = record
        profile_payload = {}
        if desired.get("login") and desired.get("login") != existing.get("login"):
            profile_payload["login"] = desired.get("login")
        if desired.get("email") and desired.get("email") != existing.get("email"):
            profile_payload["email"] = desired.get("email")
        if desired.get("name") and desired.get("name") != existing.get("name"):
            profile_payload["name"] = desired.get("name")
        if profile_payload:
            if args.dry_run:
                print("Would update user %s profile" % (record.get("login") or record.get("email") or ""))
            else:
                client.update_user(user_id, profile_payload)

        desired_org_role = normalize_org_role(desired.get("orgRole") or "")
        if args.scope == "org":
            existing_org_role = normalize_org_role(existing.get("role") or "")
        else:
            existing_org_role = normalize_org_role(existing.get("orgRole") or existing.get("role") or "")
        if desired_org_role and desired_org_role != existing_org_role:
            if args.dry_run:
                print(
                    "Would update orgRole for user %s -> %s"
                    % (record.get("login") or record.get("email") or "", desired_org_role)
                )
            else:
                client.update_user_org_role(user_id, desired_org_role)

        desired_admin = normalize_bool(desired.get("grafanaAdmin"))
        existing_admin = normalize_bool(
            existing.get("isGrafanaAdmin")
            or existing.get("isAdmin")
        )
        if desired_admin is not None and desired_admin != existing_admin:
            if args.dry_run:
                print(
                    "Would update grafanaAdmin for user %s -> %s"
                    % (
                        record.get("login") or record.get("email") or "",
                        bool_label(desired_admin),
                    )
                )
            else:
                client.update_user_permissions(user_id, desired_admin)

        updated.append(record)

        if args.scope != "global":
            target_teams = _normalize_access_identity_list(record.get("teams") or [])
            if target_teams:
                if not args.replace_existing:
                    continue
                current_members = {}
                for item in client.list_user_teams(user_id):
                    team_name = str(item.get("name") or "").strip()
                    if team_name:
                        current_members[_normalize_access_import_identity(team_name)] = {
                            "id": str(item.get("id") or ""),
                            "name": team_name,
                        }
                desired_team_keys = set(
                    _normalize_access_import_identity(team_name)
                    for team_name in target_teams
                )
                if bool(set(current_members.keys()) - desired_team_keys) and not args.yes:
                    raise GrafanaError(
                        "User import would remove team memberships for %s. Add --yes to confirm."
                        % (record.get("login") or record.get("email") or "")
                    )
                for team_name in target_teams:
                    team_key = _normalize_access_import_identity(team_name)
                    if team_key not in current_members:
                        team_payload = lookup_team_by_name(client, team_name)
                        team_id = str(team_payload.get("id") or "")
                        if not team_id:
                            raise GrafanaError(
                                "Could not resolve target team for user import: %s" % team_name
                            )
                        if args.dry_run:
                            print("Would add user %s to team %s" % (user_id, team_name))
                        else:
                            client.add_team_member(team_id, user_id)
                if args.replace_existing:
                    for team_key in set(current_members.keys()) - desired_team_keys:
                        if args.dry_run:
                            print("Would remove user %s from team %s" % (user_id, current_members[team_key]["name"]))
                        else:
                            team_id = current_members[team_key]["id"]
                            if not team_id:
                                continue
                            client.remove_team_member(team_id, user_id)

        print("Updated user %s" % (record.get("login") or record.get("email") or ""))

    print(
        "Import summary: processed=%s created=%s updated=%s skipped=%s source=%s"
        % (processed, len(created), len(updated), len(skipped), args.import_dir)
    )
    return 0


def export_service_accounts_with_client(args, client):
    """Export service accounts with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 238, 624, 643, 794, 806

    export_dir = Path(args.export_dir)
    records = [
        _normalize_service_account_record(item)
        for item in _iter_service_accounts(client)
    ]
    payload_path = export_dir / ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME
    metadata_path = export_dir / ACCESS_EXPORT_METADATA_FILENAME
    data = {
        "kind": ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
        "version": ACCESS_EXPORT_VERSION,
        "records": records,
    }
    metadata = _build_access_export_metadata(
        source_url=args.url,
        kind=ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
        source_count=len(records),
        source_dir=str(export_dir),
    )
    _assert_not_overwriting(
        export_dir,
        [ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, ACCESS_EXPORT_METADATA_FILENAME],
        dry_run=args.dry_run,
        overwrite=bool(getattr(args, "overwrite", False)),
    )
    if not args.dry_run:
        _write_json_document(payload_path, data)
        _write_json_document(metadata_path, metadata)
    action = "Would export" if args.dry_run else "Exported"
    print(
        "%s %s service-account(s) from %s -> %s and %s"
        % (action, len(records), args.url, payload_path, metadata_path)
    )
    return 0


def import_service_accounts_with_client(args, client):
    """Import service accounts with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 1158, 1167, 1206, 1236, 176, 643, 695

    validate_service_account_import_dry_run_output(args)
    bundle = _build_service_account_import_records(args.import_dir)
    raw_records = bundle.get("records") or []
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Access import entry in %s must be an object." % bundle["bundle_path"]
            )
        records.append(_normalize_service_account_record(item))

    created = 0
    updated = 0
    skipped = 0
    processed = 0
    dry_run_rows = []
    dry_run_structured = bool(args.dry_run and (args.table or args.json))

    for index, record in enumerate(records, 1):
        processed += 1
        service_account_name = str(record.get("name") or "").strip()
        if not service_account_name:
            raise GrafanaError(
                "Access service-account import record %s in %s lacks name."
                % (index, bundle["bundle_path"])
            )
        try:
            existing = _lookup_service_account_by_name(client, service_account_name)
        except GrafanaError:
            existing = None

        if existing is None:
            if not args.replace_existing:
                skipped += 1
                detail = "missing and --replace-existing was not set."
                if dry_run_structured:
                    dry_run_rows.append(
                        _build_service_account_import_row(index, service_account_name, "skip", detail)
                    )
                else:
                    print(
                        "Skipped service-account %s (%s): %s"
                        % (service_account_name, index, detail)
                    )
                continue
            if args.dry_run:
                created += 1
                if dry_run_structured:
                    dry_run_rows.append(
                        _build_service_account_import_row(
                            index,
                            service_account_name,
                            "create",
                            "would create service account",
                        )
                    )
                else:
                    print("Would create service-account %s" % service_account_name)
                continue
            client.create_service_account(
                {
                    "name": service_account_name,
                    "role": service_account_role_to_api(
                        record.get("role") or "Viewer"
                    ),
                    "isDisabled": bool(normalize_bool(record.get("disabled"))),
                }
            )
            created += 1
            print("Created service-account %s" % service_account_name)
            continue

        if not args.replace_existing:
            skipped += 1
            detail = "existing and --replace-existing was not set."
            if dry_run_structured:
                dry_run_rows.append(
                    _build_service_account_import_row(index, service_account_name, "skip", detail)
                )
            else:
                print("Skipped existing service-account %s (%s)" % (service_account_name, index))
            continue

        desired_role = normalize_org_role(record.get("role") or "")
        existing_role = normalize_org_role(existing.get("role") or "")
        desired_disabled = normalize_bool(record.get("disabled"))
        existing_disabled = normalize_bool(
            existing.get("disabled")
            if existing.get("disabled") is not None
            else existing.get("isDisabled")
        )
        update_payload = {"name": service_account_name}
        changed_fields = []
        if desired_role and desired_role != existing_role:
            update_payload["role"] = service_account_role_to_api(desired_role)
            changed_fields.append("role")
        if desired_disabled is not None and desired_disabled != existing_disabled:
            update_payload["isDisabled"] = desired_disabled
            changed_fields.append("disabled")

        if not changed_fields:
            skipped += 1
            detail = "already matched live state."
            if dry_run_structured:
                dry_run_rows.append(
                    _build_service_account_import_row(index, service_account_name, "skip", detail)
                )
            else:
                print("Skipped service-account %s (%s): %s" % (service_account_name, index, detail))
            continue

        if args.dry_run:
            updated += 1
            detail = "would update fields=%s" % ",".join(changed_fields)
            if dry_run_structured:
                dry_run_rows.append(
                    _build_service_account_import_row(index, service_account_name, "update", detail)
                )
            else:
                print("Would update service-account %s %s" % (service_account_name, detail))
            continue

        service_account_id = str(existing.get("id") or "")
        if not service_account_id:
            raise GrafanaError(
                "Resolved service-account did not include an id: %s" % service_account_name
            )
        client.update_service_account(service_account_id, update_payload)
        updated += 1
        print("Updated service-account %s" % service_account_name)

    summary = {
        "processed": processed,
        "created": created,
        "updated": updated,
        "skipped": skipped,
        "source": args.import_dir,
    }
    if dry_run_structured:
        _emit_service_account_import_dry_run_output(args, dry_run_rows, summary)
        if args.json or args.table:
            return 0
    print(
        "Import summary: processed=%s created=%s updated=%s skipped=%s source=%s"
        % (processed, created, updated, skipped, args.import_dir)
    )
    return 0


def export_teams_with_client(args, client):
    """Export teams with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 238, 272, 794, 806

    export_dir = Path(args.export_dir)
    records = _build_team_export_records(client, args)
    teams_path = export_dir / ACCESS_TEAM_EXPORT_FILENAME
    metadata_path = export_dir / ACCESS_EXPORT_METADATA_FILENAME
    data = {
        "kind": ACCESS_EXPORT_KIND_TEAMS,
        "version": ACCESS_EXPORT_VERSION,
        "records": records,
    }
    metadata = _build_access_export_metadata(
        source_url=args.url,
        kind=ACCESS_EXPORT_KIND_TEAMS,
        source_count=len(records),
        source_dir=str(export_dir),
    )
    _assert_not_overwriting(
        export_dir,
        [ACCESS_TEAM_EXPORT_FILENAME, ACCESS_EXPORT_METADATA_FILENAME],
        dry_run=args.dry_run,
        overwrite=bool(getattr(args, "overwrite", False)),
    )
    if not args.dry_run:
        _write_json_document(teams_path, data)
        _write_json_document(metadata_path, metadata)
    action = "Would export" if args.dry_run else "Exported"
    print(
        "%s %s team(s) from %s -> %s and %s"
        % (action, len(records), args.url, teams_path, metadata_path)
    )
    return 0


def import_teams_with_client(args, client):
    """Import teams with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 1001, 1140, 2040, 341, 818, 823, 966

    bundle = _build_team_import_records(args.import_dir)
    raw_records = bundle.get("records") or []
    records = []
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError("Access import entry in %s must be an object." % bundle["bundle_path"])
        records.append(_normalize_team_record(item))

    created = 0
    updated = 0
    skipped = 0
    for index, record in enumerate(records, 1):
        team_name = str(record.get("name") or "").strip()
        if not team_name:
            raise GrafanaError(
                "Access team import record %s in %s is missing name." % (index, bundle["bundle_path"])
            )
        existing = None
        try:
            existing = lookup_team_by_name(client, team_name)
        except GrafanaError:
            existing = None

        if existing is None:
            created += 1
            if args.dry_run:
                print("Would create team %s" % team_name)
            else:
                created_payload = client.create_team({"name": team_name, "email": str(record.get("email") or "")})
                team_id = str(
                    created_payload.get("teamId")
                    or created_payload.get("id")
                    or ""
                )
                if not team_id:
                    raise GrafanaError("Team import did not return team id for %s" % team_name)
                target_members = _normalize_access_identity_list(record.get("members") or [])
                target_admins = _normalize_access_identity_list(record.get("admins") or [])
                if target_members or target_admins:
                    existing_members = {}
                    _sync_team_members_for_import(
                        client,
                        team_id,
                        team_name,
                        existing_members,
                        target_members,
                        target_admins,
                        include_missing=True,
                        dry_run=args.dry_run,
                    )
                print("Created team %s" % team_name)
            continue

        team_id = str(existing.get("id") or existing.get("teamId") or "")
        if not team_id:
            raise GrafanaError("Team %s resolved without id." % team_name)

        if not args.replace_existing:
            skipped += 1
            print("Skipped team %s (%s)" % (team_name, index))
            continue
        target_members = _normalize_access_identity_list(record.get("members") or [])
        target_admins = _normalize_access_identity_list(record.get("admins") or [])
        existing_members = _lookup_team_memberships_by_identity(client, team_id)
        target_keys = set(
            _normalize_access_import_identity(item)
            for item in target_members + target_admins
        )
        if set(existing_members.keys()) - target_keys and not args.yes:
            raise GrafanaError("Team import would remove team memberships for %s. Add --yes to confirm." % team_name)

        if args.dry_run:
            print("Would update team %s" % team_name)
        else:
            _sync_team_members_for_import(
                client,
                team_id,
                team_name,
                existing_members,
                target_members,
                target_admins,
                include_missing=True,
                dry_run=args.dry_run,
            )
            print("Updated team %s" % team_name)
        updated += 1

    print(
        "Import summary: processed=%s created=%s updated=%s skipped=%s source=%s"
        % (created + updated + skipped, created, updated, skipped, args.import_dir)
    )
    return 0


def lookup_service_account_id_by_name(client, service_account_name):
    """Lookup service account id by name implementation."""
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


def lookup_team_by_name(client, team_name):
    """Lookup team by name implementation."""
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


def lookup_org_user_by_identity(client, identity):
    """Lookup org user by identity implementation."""
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


def lookup_global_user_by_identity(client, login=None, email=None):
    """Lookup global user by identity implementation."""
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


def lookup_org_user_by_user_id(client, user_id):
    """Lookup org user by user id implementation."""
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


def normalize_modified_user(base_user, args):
    """Normalize modified user implementation."""
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


def normalize_deleted_user(base_user, scope):
    """Normalize deleted user implementation."""
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


def normalize_identity_list(values):
    """Normalize identity list implementation."""
    normalized = []
    seen = set()
    for value in values:
        item = str(value or "").strip()
        if not item or item in seen:
            continue
        normalized.append(item)
        seen.add(item)
    return normalized


def validate_conflicting_identity_sets(add_values, remove_values, add_label, remove_label):
    """Validate conflicting identity sets implementation."""
    overlap = set(add_values) & set(remove_values)
    if overlap:
        raise GrafanaError(
            "Cannot target the same identity in both %s and %s: %s"
            % (add_label, remove_label, ", ".join(sorted(overlap)))
        )


def team_member_admin_state(member):
    """Team member admin state implementation."""
    explicit = normalize_bool(
        member.get("isAdmin", member.get("admin"))
    )
    if explicit is not None:
        return explicit
    for key in ("role", "teamRole", "permissionName"):
        value = str(member.get(key) or "").strip().lower()
        if not value:
            continue
        if value in {"admin", "teamadmin", "team-admin", "administrator"}:
            return True
        if value in {"member", "viewer", "editor"}:
            return False
    permission = member.get("permission")
    if permission is not None:
        try:
            parsed = int(permission)
        except (TypeError, ValueError):
            parsed = None
        if parsed == 4:
            return True
        if parsed == 0:
            return False
    return None


def extract_member_identity(member):
    """Extract member identity implementation."""
    login = str(member.get("login") or "").strip()
    email = str(member.get("email") or "").strip()
    return email or login


def format_user_summary_line(user):
    """Format user summary line implementation."""
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
    parts.append("grafanaAdmin=%s" % grafana_admin)
    teams = user.get("teams") or []
    if teams:
        parts.append("teams=%s" % ",".join(teams))
    parts.append("scope=%s" % (user.get("scope") or ""))
    return " ".join(parts)


def format_deleted_team_summary_line(team):
    """Format deleted team summary line implementation."""
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


def format_deleted_service_account_summary_line(service_account):
    """Format deleted service account summary line implementation."""
    parts = [
        "serviceAccountId=%s" % (service_account.get("id") or ""),
        "name=%s" % (service_account.get("name") or ""),
    ]
    login = service_account.get("login") or ""
    if login:
        parts.append("login=%s" % login)
    message = service_account.get("message") or ""
    if message:
        parts.append("message=%s" % message)
    return " ".join(parts)


def format_deleted_service_account_token_summary_line(token):
    """Format deleted service account token summary line implementation."""
    parts = [
        "serviceAccountId=%s" % (token.get("serviceAccountId") or ""),
        "tokenId=%s" % (token.get("id") or ""),
        "name=%s" % (token.get("name") or ""),
    ]
    message = token.get("message") or ""
    if message:
        parts.append("message=%s" % message)
    return " ".join(parts)


def list_users_with_client(args, client):
    """List users with client implementation."""
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


def list_service_accounts_with_client(args, client):
    """List service accounts with client implementation."""
    items = client.list_service_accounts(
        query=args.query,
        page=args.page,
        per_page=args.per_page,
    )
    rows = []
    for item in items:
        normalized = normalize_service_account(item)
        if args.query and not service_account_matches_query(normalized, args.query):
            continue
        rows.append(normalized)
    if args.csv:
        render_service_account_csv(rows)
        return 0
    if args.json:
        print(render_service_account_json(rows))
        return 0
    if args.table:
        for line in render_service_account_table(rows):
            print(line)
    else:
        for row in rows:
            print(format_service_account_summary_line(row))
    print("")
    print("Listed %s service account(s) at %s" % (len(rows), args.url))
    return 0


def list_teams_with_client(args, client):
    """List teams with client implementation."""
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


def _build_org_rows(client, args):
    """Internal helper for build org rows."""
    rows = []
    target_org_id = str(
        getattr(args, "org_id", getattr(args, "target_org_id", "")) or ""
    ).strip()
    exact_name = str(getattr(args, "name", "") or "").strip()
    query = str(getattr(args, "query", "") or "").strip().lower()
    include_users = bool(getattr(args, "with_users", False))
    for item in client.list_organizations():
        org = _normalize_org_record(item)
        if target_org_id and org.get("id") != target_org_id:
            continue
        if exact_name and org.get("name") != exact_name:
            continue
        if query and query not in str(org.get("name") or "").lower():
            continue
        if include_users:
            org["users"] = [
                _normalize_org_user_record(member)
                for member in client.list_organization_users(org.get("id") or "")
            ]
            org["userCount"] = str(len(org["users"]))
        rows.append(org)
    rows.sort(key=lambda item: (item.get("name") or "", item.get("id") or ""))
    return rows


def _render_org_table(rows):
    """Internal helper for render org table."""
    headers = ["ID", "NAME", "USER_COUNT"]
    widths = [len(header) for header in headers]
    values = []
    for row in rows:
        current = [
            str(row.get("id") or ""),
            str(row.get("name") or ""),
            str(row.get("userCount") or ""),
        ]
        values.append(current)
        for index, value in enumerate(current):
            widths[index] = max(widths[index], len(value))

    def _format(items):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            item.ljust(widths[index]) for index, item in enumerate(items)
        )

    lines = [_format(headers), _format(["-" * width for width in widths])]
    for row in values:
        lines.append(_format(row))
    return lines


def _render_org_csv(rows):
    """Internal helper for render org csv."""
    print("id,name,userCount")
    for row in rows:
        print(
            "%s,%s,%s"
            % (
                _csv_escape(str(row.get("id") or "")),
                _csv_escape(str(row.get("name") or "")),
                _csv_escape(str(row.get("userCount") or "")),
            )
        )


def _csv_escape(value):
    """Internal helper for csv escape."""
    text = str(value or "")
    if any(char in text for char in [",", "\"", "\n"]):
        return "\"%s\"" % text.replace("\"", "\"\"")
    return text


def _render_org_json(rows):
    """Internal helper for render org json."""
    return json.dumps(rows, indent=2, ensure_ascii=False)


def _format_org_summary_line(row):
    """Internal helper for format org summary line."""
    parts = [
        "id=%s" % (row.get("id") or ""),
        "name=%s" % (row.get("name") or ""),
        "userCount=%s" % (row.get("userCount") or "0"),
    ]
    return " ".join(parts)


def list_orgs_with_client(args, client):
    """List orgs with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 2364, 2392, 2422, 2444, 2449

    rows = _build_org_rows(client, args)
    if args.csv:
        _render_org_csv(rows)
        return 0
    if args.json:
        print(_render_org_json(rows))
        return 0
    if args.table:
        for line in _render_org_table(rows):
            print(line)
    else:
        for row in rows:
            print(_format_org_summary_line(row))
    print("")
    print("Listed %s org(s) at %s" % (len(rows), args.url))
    return 0


def add_service_account_with_client(args, client):
    """Add service account with client implementation."""
    payload = {
        "name": args.name,
        "role": service_account_role_to_api(args.role),
        "isDisabled": args.disabled == "true",
    }
    created = normalize_service_account(client.create_service_account(payload))
    if args.json:
        print(
            json.dumps(
                serialize_service_account_row(created),
                indent=2,
                ensure_ascii=False,
            )
        )
    else:
        print(
            "Created service-account %s -> id=%s role=%s disabled=%s"
            % (
                created.get("name") or "",
                created.get("id") or "",
                created.get("role") or "",
                bool_label(normalize_bool(created.get("disabled"))),
            )
        )
    return 0


def lookup_organization(client, org_id=None, name=None):
    """Lookup organization implementation."""
    target_org_id = str(org_id or "").strip()
    target_name = str(name or "").strip()
    if not target_org_id and not target_name:
        raise GrafanaError("Organization lookup requires --org-id or --name.")
    matches = []
    for item in client.list_organizations():
        item_id = str(item.get("id") or item.get("orgId") or "").strip()
        item_name = str(item.get("name") or "").strip()
        if target_org_id and item_id == target_org_id:
            matches.append(item)
            continue
        if target_name and item_name == target_name:
            matches.append(item)
    if not matches:
        raise GrafanaError(
            "Organization not found by id or name: %s" % (target_org_id or target_name)
        )
    if len(matches) > 1:
        raise GrafanaError(
            "Organization identity matched multiple items: %s"
            % (target_org_id or target_name)
        )
    return _normalize_org_record(matches[0])


def add_org_with_client(args, client):
    """Add org with client implementation."""
    created_payload = client.create_organization({"name": args.name})
    org_id = str(created_payload.get("orgId") or created_payload.get("id") or "")
    created = {
        "id": org_id,
        "name": str(args.name or ""),
        "userCount": "0",
        "users": [],
    }
    if args.json:
        print(json.dumps(created, indent=2, ensure_ascii=False))
    else:
        print("Created org %s -> id=%s" % (created.get("name") or "", org_id))
    return 0


def modify_org_with_client(args, client):
    """Modify org with client implementation."""
    org = lookup_organization(
        client,
        org_id=getattr(args, "org_id", getattr(args, "target_org_id", None)),
        name=args.name,
    )
    org_id = str(org.get("id") or "")
    if not org_id:
        raise GrafanaError("Organization lookup did not return an id.")
    client.update_organization(org_id, {"name": args.set_name})
    modified = {
        "id": org_id,
        "name": str(args.set_name or ""),
        "previousName": str(org.get("name") or ""),
        "userCount": str(org.get("userCount") or "0"),
        "users": org.get("users") or [],
    }
    if args.json:
        print(json.dumps(modified, indent=2, ensure_ascii=False))
    else:
        print(
            "Modified org %s -> id=%s name=%s"
            % (
                org.get("name") or "",
                org_id,
                modified.get("name") or "",
            )
        )
    return 0


def delete_org_with_client(args, client):
    """Delete org with client implementation."""
    validate_destructive_confirmed(args, "Org delete requires --yes.")
    org = lookup_organization(
        client,
        org_id=getattr(args, "org_id", getattr(args, "target_org_id", None)),
        name=args.name,
    )
    org_id = str(org.get("id") or "")
    if not org_id:
        raise GrafanaError("Organization lookup did not return an id.")
    delete_payload = client.delete_organization(org_id)
    result = {
        "id": org_id,
        "name": str(org.get("name") or ""),
        "message": str(delete_payload.get("message") or ""),
    }
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print("Deleted org %s -> id=%s" % (result["name"], result["id"]))
    return 0


def add_user_with_client(args, client):
    """Add user with client implementation."""
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
    if args.org_role is not None:
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


def modify_user_with_client(args, client):
    """Modify user with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 2079, 2125, 94

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


def delete_user_with_client(args, client):
    """Delete user with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 132, 2057, 2079, 2106, 2145

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


def modify_team_with_client(args, client):
    """Modify team with client implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 3059
    #   Downstream callees: 147, 2040, 2775

    validate_team_modify_args(args)
    if args.team_id:
        team_payload = client.get_team(args.team_id)
    else:
        team_payload = lookup_team_by_name(client, args.name)
    team_id = str(team_payload.get("id") or args.team_id or "")
    if not team_id:
        raise GrafanaError("Resolved team did not include an id.")
    team_name = str(team_payload.get("name") or args.name or "")
    payload = apply_team_membership_changes(
        client,
        team_id,
        team_name,
        add_member=args.add_member,
        remove_member=args.remove_member,
        add_admin=args.add_admin,
        remove_admin=args.remove_admin,
    )
    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        print(format_team_modify_summary_line(payload))
    return 0


def apply_team_membership_changes(
    client,
    team_id,
    team_name,
    add_member=None,
    remove_member=None,
    add_admin=None,
    remove_admin=None,
    fetch_existing_members=True,
):
    """Apply team membership changes implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 2748, 2913
    #   Downstream callees: 2057, 2166, 2179, 2189, 2217

    add_member_targets = normalize_identity_list(add_member or [])
    remove_member_targets = normalize_identity_list(remove_member or [])
    add_admin_targets = normalize_identity_list(add_admin or [])
    remove_admin_targets = normalize_identity_list(remove_admin or [])

    validate_conflicting_identity_sets(
        add_member_targets, remove_member_targets, "--add-member", "--remove-member"
    )
    validate_conflicting_identity_sets(
        add_admin_targets, remove_admin_targets, "--add-admin", "--remove-admin"
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
            if not identity:
                raise GrafanaError(
                    "Resolved user did not include a login or email for %s." % target
                )
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
            {
                "members": regular_members,
                "admins": admin_members,
            },
        )

    return {
        "teamId": team_id,
        "name": team_name,
        "addedMembers": added_members,
        "removedMembers": removed_members,
        "addedAdmins": added_admins,
        "removedAdmins": removed_admins,
    }


def add_team_with_client(args, client):
    """Add team with client implementation."""
    payload = {
        "name": args.name,
    }
    if args.email is not None:
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


def add_service_account_token_with_client(args, client):
    """Add service account token with client implementation."""
    if args.service_account_id:
        service_account_id = str(args.service_account_id)
    else:
        service_account_id = lookup_service_account_id_by_name(client, args.name)
    payload = {
        "name": args.token_name,
    }
    if args.seconds_to_live is not None:
        payload["secondsToLive"] = args.seconds_to_live
    token_payload = client.create_service_account_token(service_account_id, payload)
    token_payload["serviceAccountId"] = str(service_account_id)
    if args.json:
        print(render_service_account_token_json(token_payload))
    else:
        print(
            "Created service-account token %s -> serviceAccountId=%s"
            % (args.token_name, service_account_id)
        )
    return 0


def delete_service_account_with_client(args, client):
    """Delete service account with client implementation."""
    validate_destructive_confirmed(
        args,
        "Service-account delete",
    )
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


def delete_service_account_token_with_client(args, client):
    """Delete service account token with client implementation."""
    validate_destructive_confirmed(
        args,
        "Service-account token delete",
    )
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


def delete_team_with_client(args, client):
    """Delete team with client implementation."""
    validate_destructive_confirmed(args, "Team delete requires --yes.")
    team_id = resolve_team_id(client, args.team_id, args.name)
    team_payload = client.get_team(team_id)
    delete_payload = client.delete_team(team_id)
    result = {
        "teamId": str(team_payload.get("id") or team_id),
        "name": str(team_payload.get("name") or args.name or ""),
        "email": str(team_payload.get("email") or ""),
        "message": str(delete_payload.get("message") or ""),
    }
    if args.json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(format_deleted_team_summary_line(result))
    return 0




def dispatch_access_command(args, client, auth_mode):
    """Dispatch access command implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 123, 1248, 1282, 138, 1386, 1506, 161, 166, 1693, 171, 1730, 1881, 1915, 2291, 2314, 2344, 2459, 2479, 2535, 2552, 2584, 2608, 2648, 2699, 2748, 2913, 2945, 2968, 2997, 3038, 493, 555, 59, 718, 76, 85

    if args.resource == "org" and args.command == "list":
        validate_org_auth(auth_mode)
        return list_orgs_with_client(args, client)
    if args.resource == "org" and args.command == "add":
        validate_org_auth(auth_mode)
        return add_org_with_client(args, client)
    if args.resource == "org" and args.command == "modify":
        validate_org_auth(auth_mode)
        return modify_org_with_client(args, client)
    if args.resource == "org" and args.command == "delete":
        validate_org_auth(auth_mode)
        return delete_org_with_client(args, client)
    if args.resource == "org" and args.command == "export":
        validate_org_auth(auth_mode)
        return export_orgs_with_client(args, client)
    if args.resource == "org" and args.command == "import":
        validate_org_auth(auth_mode)
        return import_orgs_with_client(args, client)
    if args.resource == "user" and args.command == "list":
        validate_user_list_auth(args, auth_mode)
        return list_users_with_client(args, client)
    if args.resource == "user" and args.command == "export":
        return export_users_with_client(args, client)
    if args.resource == "user" and args.command == "import":
        return import_users_with_client(args, client)
    if args.resource == "user" and args.command == "add":
        validate_user_add_auth(auth_mode)
        return add_user_with_client(args, client)
    if args.resource == "user" and args.command == "modify":
        validate_user_modify_auth(auth_mode)
        return modify_user_with_client(args, client)
    if args.resource == "user" and args.command == "delete":
        validate_user_delete_auth(args, auth_mode)
        return delete_user_with_client(args, client)
    if args.resource == "user" and args.command == "diff":
        validate_user_list_auth(args, auth_mode)
        return diff_users_with_client(args, client)
    if args.resource == "team" and args.command == "list":
        return list_teams_with_client(args, client)
    if args.resource == "team" and args.command == "add":
        return add_team_with_client(args, client)
    if args.resource == "team" and args.command == "modify":
        return modify_team_with_client(args, client)
    if args.resource == "team" and args.command == "delete":
        validate_team_delete_auth(auth_mode)
        return delete_team_with_client(args, client)
    if args.resource == "team" and args.command == "diff":
        return diff_teams_with_client(args, client)
    if args.resource == "team" and args.command == "export":
        return export_teams_with_client(args, client)
    if args.resource == "team" and args.command == "import":
        return import_teams_with_client(args, client)
    if args.resource == "service-account" and args.command == "list":
        return list_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "add":
        return add_service_account_with_client(args, client)
    if args.resource == "service-account" and args.command == "export":
        return export_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "import":
        return import_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "diff":
        return diff_service_accounts_with_client(args, client)
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
