"""Workflow and helper logic for the Python access-management CLI."""

import json
from typing import Any, Optional

from .common import (
    DEFAULT_PAGE_SIZE,
    GrafanaError,
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
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User list with --scope global requires Basic auth "
            "(--basic-user / --basic-password)."
        )
    if args.with_teams and auth_mode != "basic":
        raise GrafanaError("--with-teams requires Basic auth.")


def validate_user_add_auth(auth_mode):
    if auth_mode != "basic":
        raise GrafanaError(
            "User add requires Basic auth (--basic-user / --basic-password)."
        )


def validate_user_modify_args(args):
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


def validate_user_modify_auth(auth_mode):
    if auth_mode != "basic":
        raise GrafanaError(
            "User modify requires Basic auth (--basic-user / --basic-password)."
        )


def validate_user_delete_args(args):
    if not args.yes:
        raise GrafanaError("User delete requires --yes.")


def validate_user_delete_auth(args, auth_mode):
    if args.scope == "global" and auth_mode != "basic":
        raise GrafanaError(
            "User delete with --scope global requires Basic auth "
            "(--basic-user / --basic-password)."
        )


def validate_team_modify_args(args):
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
    return None


def validate_service_account_delete_auth(_auth_mode):
    return None


def validate_service_account_token_delete_auth(_auth_mode):
    return None


def service_account_role_to_api(value):
    normalized = normalize_org_role(value)
    if normalized == "None":
        return "NoBasicRole"
    return normalized


def normalize_created_user(user_id, args):
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


def lookup_service_account_id_by_name(client, service_account_name):
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
    overlap = set(add_values) & set(remove_values)
    if overlap:
        raise GrafanaError(
            "Cannot target the same identity in both %s and %s: %s"
            % (add_label, remove_label, ", ".join(sorted(overlap)))
        )


def team_member_admin_state(member):
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
    login = str(member.get("login") or "").strip()
    email = str(member.get("email") or "").strip()
    return email or login


def format_user_summary_line(user):
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


def add_service_account_with_client(args, client):
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


def add_user_with_client(args, client):
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
    if args.resource == "team" and args.command == "delete":
        validate_team_delete_auth(auth_mode)
        return delete_team_with_client(args, client)
    if args.resource == "service-account" and args.command == "list":
        return list_service_accounts_with_client(args, client)
    if args.resource == "service-account" and args.command == "add":
        return add_service_account_with_client(args, client)
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
