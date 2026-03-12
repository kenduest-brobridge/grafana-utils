import argparse
import ast
import importlib
import io
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "access_cli.py"
WRAPPER_PATH = REPO_ROOT / "cmd" / "grafana-access-utils.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
access_utils = importlib.import_module("grafana_utils.access_cli")


class FakeAccessClient:
    def __init__(
        self,
        org_users=None,
        global_users=None,
        teams_by_user_id=None,
        teams=None,
        team_members_by_team_id=None,
        service_accounts=None,
    ):
        self.org_users = [dict(item) for item in (org_users or [])]
        self.global_users = [dict(item) for item in (global_users or [])]
        self.teams_by_user_id = {
            str(key): [dict(item) for item in value]
            for key, value in (teams_by_user_id or {}).items()
        }
        self.teams = [dict(item) for item in (teams or [])]
        self.team_members_by_team_id = {
            str(key): [dict(item) for item in value]
            for key, value in (team_members_by_team_id or {}).items()
        }
        self.service_accounts = [dict(item) for item in (service_accounts or [])]
        self.global_page_sizes = []
        self.team_lookups = []
        self.team_searches = []
        self.team_member_lookups = []
        self.service_account_searches = []
        self.created_service_accounts = []
        self.created_service_account_tokens = []
        self.created_users = []
        self.user_gets = []
        self.updated_users = []
        self.updated_user_passwords = []
        self.deleted_global_users = []
        self.deleted_org_users = []
        self.updated_user_org_roles = []
        self.updated_user_permissions = []
        self.team_gets = []
        self.created_teams = []
        self.added_team_members = []
        self.removed_team_members = []
        self.updated_team_memberships = []

    def list_org_users(self):
        return [dict(item) for item in self.org_users]

    def iter_global_users(self, page_size):
        self.global_page_sizes.append(page_size)
        return [dict(item) for item in self.global_users]

    def list_user_teams(self, user_id):
        self.team_lookups.append(str(user_id))
        return [dict(item) for item in self.teams_by_user_id.get(str(user_id), [])]

    def list_teams(self, query, page, per_page):
        self.team_searches.append((query, page, per_page))
        return [dict(item) for item in self.teams]

    def iter_teams(self, query, page_size):
        self.team_searches.append((query, "iter", page_size))
        return [dict(item) for item in self.teams]

    def list_team_members(self, team_id):
        self.team_member_lookups.append(str(team_id))
        return [
            dict(item)
            for item in self.team_members_by_team_id.get(str(team_id), [])
        ]

    def get_team(self, team_id):
        self.team_gets.append(str(team_id))
        for item in self.teams:
            if str(item.get("id")) == str(team_id):
                return dict(item)
        return {"id": team_id, "name": ""}

    def create_team(self, payload):
        self.created_teams.append(dict(payload))
        team_id = str(len(self.teams) + 40)
        team = {
            "id": team_id,
            "name": payload.get("name"),
            "email": payload.get("email", ""),
            "memberCount": 0,
        }
        self.teams.append(dict(team))
        self.team_members_by_team_id.setdefault(team_id, [])
        return {"teamId": team_id, "message": "Team created"}

    def add_team_member(self, team_id, user_id):
        self.added_team_members.append((str(team_id), str(user_id)))
        return {"message": "Team member added"}

    def remove_team_member(self, team_id, user_id):
        self.removed_team_members.append((str(team_id), str(user_id)))
        return {"message": "Team member removed"}

    def update_team_members(self, team_id, payload):
        self.updated_team_memberships.append((str(team_id), dict(payload)))
        return {"message": "Team members updated"}

    def list_service_accounts(self, query, page, per_page):
        self.service_account_searches.append((query, page, per_page))
        return [dict(item) for item in self.service_accounts]

    def create_service_account(self, payload):
        self.created_service_accounts.append(dict(payload))
        return {
            "id": 21,
            "name": payload.get("name"),
            "login": "sa-1-%s" % payload.get("name"),
            "role": payload.get("role"),
            "isDisabled": payload.get("isDisabled"),
            "tokens": 0,
            "orgId": 1,
        }

    def create_service_account_token(self, service_account_id, payload):
        self.created_service_account_tokens.append(
            (str(service_account_id), dict(payload))
        )
        return {
            "id": 4,
            "name": payload.get("name"),
            "key": "glsa_token",
            "secondsToLive": payload.get("secondsToLive"),
        }

    def create_user(self, payload):
        self.created_users.append(dict(payload))
        return {
            "id": 31,
            "message": "User created",
        }

    def get_user(self, user_id):
        self.user_gets.append(str(user_id))
        for item in self.global_users:
            if str(item.get("id")) == str(user_id):
                return dict(item)
        return {"id": user_id}

    def update_user(self, user_id, payload):
        self.updated_users.append((str(user_id), dict(payload)))
        return {"message": "User updated"}

    def update_user_password(self, user_id, password):
        self.updated_user_passwords.append((str(user_id), password))
        return {"message": "User password updated"}

    def delete_global_user(self, user_id):
        self.deleted_global_users.append(str(user_id))
        return {"message": "User deleted"}

    def delete_org_user(self, user_id):
        self.deleted_org_users.append(str(user_id))
        return {"message": "Org user removed"}

    def update_user_org_role(self, user_id, role):
        self.updated_user_org_roles.append((str(user_id), role))
        return {"message": "Organization user updated"}

    def update_user_permissions(self, user_id, is_grafana_admin):
        self.updated_user_permissions.append((str(user_id), bool(is_grafana_admin)))
        return {"message": "User permissions updated"}


class AccessCliTests(unittest.TestCase):
    def test_access_script_parses_as_python36_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 6))

    def test_access_wrapper_script_parses_as_python36_syntax(self):
        source = WRAPPER_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(WRAPPER_PATH), feature_version=(3, 6))

    def test_parse_args_without_command_prints_top_level_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args([])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("{user,team,service-account}", stdout.getvalue())

    def test_parse_args_user_without_subcommand_prints_user_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args(["user"])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("list", stdout.getvalue())

    def test_parse_args_team_without_subcommand_prints_team_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args(["team"])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("list", stdout.getvalue())

    def test_parse_args_service_account_token_without_subcommand_prints_token_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args(["service-account", "token"])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("add", stdout.getvalue())

    def test_parse_args_supports_user_list_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "list",
                "--scope",
                "global",
                "--query",
                "ops",
                "--page",
                "2",
                "--per-page",
                "5",
                "--table",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "list")
        self.assertEqual(args.scope, "global")
        self.assertEqual(args.query, "ops")
        self.assertEqual(args.page, 2)
        self.assertEqual(args.per_page, 5)
        self.assertTrue(args.table)

    def test_parse_args_supports_user_add_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "add",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--login",
                "alice",
                "--email",
                "alice@example.com",
                "--name",
                "Alice",
                "--password",
                "secret123",
                "--org-id",
                "7",
                "--org-role",
                "Editor",
                "--grafana-admin",
                "true",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "add")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.login, "alice")
        self.assertEqual(args.email, "alice@example.com")
        self.assertEqual(args.name, "Alice")
        self.assertEqual(args.new_user_password, "secret123")
        self.assertEqual(args.org_id, "7")
        self.assertEqual(args.org_role, "Editor")
        self.assertEqual(args.grafana_admin, "true")
        self.assertTrue(args.json)

    def test_parse_args_supports_user_modify_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "modify",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--login",
                "alice",
                "--set-login",
                "alice2",
                "--set-email",
                "alice2@example.com",
                "--set-name",
                "Alice Two",
                "--set-password",
                "new-secret",
                "--set-org-role",
                "Admin",
                "--set-grafana-admin",
                "true",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "modify")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.login, "alice")
        self.assertEqual(args.set_login, "alice2")
        self.assertEqual(args.set_email, "alice2@example.com")
        self.assertEqual(args.set_name, "Alice Two")
        self.assertEqual(args.set_password, "new-secret")
        self.assertEqual(args.set_org_role, "Admin")
        self.assertEqual(args.set_grafana_admin, "true")
        self.assertTrue(args.json)

    def test_parse_args_supports_user_delete_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "delete",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--email",
                "alice@example.com",
                "--scope",
                "org",
                "--yes",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "delete")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.email, "alice@example.com")
        self.assertEqual(args.scope, "org")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_user_add_help_uses_basic_auth_and_local_password_flags(self):
        parser = access_utils.build_parser()
        user_add_parser = parser._subparsers._group_actions[0].choices["user"]._subparsers._group_actions[0].choices["add"]
        help_text = user_add_parser.format_help()

        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--basic-password PASSWORD", help_text)
        self.assertIn("--password NEW_USER_PASSWORD", help_text)
        self.assertNotIn("--token", help_text)

    def test_user_modify_help_uses_basic_auth_only(self):
        parser = access_utils.build_parser()
        user_modify_parser = parser._subparsers._group_actions[0].choices["user"]._subparsers._group_actions[0].choices["modify"]
        help_text = user_modify_parser.format_help()

        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--basic-password PASSWORD", help_text)
        self.assertIn("--set-password SET_PASSWORD", help_text)
        self.assertNotIn("--token", help_text)

    def test_user_delete_help_uses_scope_and_confirmation_flags(self):
        parser = access_utils.build_parser()
        user_delete_parser = parser._subparsers._group_actions[0].choices["user"]._subparsers._group_actions[0].choices["delete"]
        help_text = user_delete_parser.format_help()

        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--scope {org,global}", help_text)
        self.assertIn("--yes", help_text)

    def test_parse_args_supports_team_list_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "list",
                "--query",
                "ops",
                "--name",
                "Ops",
                "--with-members",
                "--page",
                "2",
                "--per-page",
                "5",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "list")
        self.assertEqual(args.query, "ops")
        self.assertEqual(args.name, "Ops")
        self.assertTrue(args.with_members)
        self.assertEqual(args.page, 2)
        self.assertEqual(args.per_page, 5)
        self.assertTrue(args.json)

    def test_team_add_help_describes_initial_members_and_admins(self):
        parser = access_utils.build_parser()
        team_add_parser = parser._subparsers._group_actions[0].choices["team"]._subparsers._group_actions[0].choices["add"]
        help_text = team_add_parser.format_help()

        self.assertIn("--name NAME", help_text)
        self.assertIn("--email EMAIL", help_text)
        self.assertIn("--member LOGIN_OR_EMAIL", help_text)
        self.assertIn("--admin LOGIN_OR_EMAIL", help_text)

    def test_parse_args_supports_team_add_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "add",
                "--name",
                "Ops",
                "--email",
                "ops@example.com",
                "--member",
                "alice",
                "--member",
                "bob@example.com",
                "--admin",
                "carol",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "add")
        self.assertEqual(args.name, "Ops")
        self.assertEqual(args.email, "ops@example.com")
        self.assertEqual(args.member, ["alice", "bob@example.com"])
        self.assertEqual(args.admin, ["carol"])
        self.assertTrue(args.json)

    def test_parse_args_supports_team_modify_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "modify",
                "--team-id",
                "7",
                "--add-member",
                "alice",
                "--remove-member",
                "bob@example.com",
                "--add-admin",
                "carol",
                "--remove-admin",
                "dave@example.com",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "modify")
        self.assertEqual(args.team_id, "7")
        self.assertEqual(args.add_member, ["alice"])
        self.assertEqual(args.remove_member, ["bob@example.com"])
        self.assertEqual(args.add_admin, ["carol"])
        self.assertEqual(args.remove_admin, ["dave@example.com"])
        self.assertTrue(args.json)

    def test_parse_args_supports_preferred_auth_aliases(self):
        args = access_utils.parse_args(
            [
                "user",
                "list",
                "--token",
                "abc123",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
            ]
        )

        self.assertEqual(args.api_token, "abc123")
        self.assertEqual(args.username, "admin")
        self.assertEqual(args.password, "secret")
        self.assertFalse(args.prompt_password)

    def test_parse_args_supports_prompt_password(self):
        args = access_utils.parse_args(
            ["user", "list", "--basic-user", "admin", "--prompt-password"]
        )

        self.assertEqual(args.username, "admin")
        self.assertIsNone(args.password)
        self.assertTrue(args.prompt_password)

    def test_parse_args_supports_service_account_token_add(self):
        args = access_utils.parse_args(
            [
                "service-account",
                "token",
                "add",
                "--service-account-id",
                "7",
                "--token-name",
                "robot-token",
                "--seconds-to-live",
                "3600",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "service-account")
        self.assertEqual(args.command, "token")
        self.assertEqual(args.token_command, "add")
        self.assertEqual(args.service_account_id, "7")
        self.assertEqual(args.token_name, "robot-token")
        self.assertEqual(args.seconds_to_live, 3600)
        self.assertTrue(args.json)

    def test_build_request_headers_adds_org_id(self):
        args = argparse.Namespace(
            api_token="abc123",
            username=None,
            password=None,
            org_id="7",
        )

        headers, auth_mode = access_utils.build_request_headers(args)

        self.assertEqual(auth_mode, "token")
        self.assertEqual(headers["Authorization"], "Bearer abc123")
        self.assertEqual(headers["X-Grafana-Org-Id"], "7")

    def test_resolve_auth_supports_basic_auth(self):
        args = argparse.Namespace(
            api_token=None,
            username="admin",
            password="secret",
            prompt_password=False,
        )

        headers, auth_mode = access_utils.resolve_auth(args)

        self.assertEqual(auth_mode, "basic")
        self.assertTrue(headers["Authorization"].startswith("Basic "))

    def test_resolve_auth_prefers_explicit_basic_auth_over_env_token(self):
        import os

        args = argparse.Namespace(
            api_token=None,
            username=None,
            password=None,
            prompt_password=False,
            auth_username="admin",
            auth_password="secret",
        )
        original_token = os.environ.get("GRAFANA_API_TOKEN")
        os.environ["GRAFANA_API_TOKEN"] = "env-token"
        try:
            headers, auth_mode = access_utils.resolve_auth(args)
        finally:
            if original_token is None:
                os.environ.pop("GRAFANA_API_TOKEN", None)
            else:
                os.environ["GRAFANA_API_TOKEN"] = original_token

        self.assertEqual(auth_mode, "basic")
        self.assertTrue(headers["Authorization"].startswith("Basic "))

    def test_resolve_auth_rejects_mixed_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="admin",
            password="secret",
            prompt_password=False,
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "Choose either token auth"):
            access_utils.resolve_auth(args)

    def test_resolve_auth_supports_prompt_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="admin",
            password=None,
            prompt_password=True,
        )

        with mock.patch("grafana_utils.access_cli.getpass.getpass", return_value="secret") as prompt:
            headers, auth_mode = access_utils.resolve_auth(args)

        self.assertEqual(auth_mode, "basic")
        self.assertTrue(headers["Authorization"].startswith("Basic "))
        prompt.assert_called_once_with("Grafana Basic auth password: ")

    def test_resolve_auth_rejects_prompt_without_username(self):
        args = argparse.Namespace(
            api_token=None,
            username=None,
            password=None,
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            access_utils.GrafanaError,
            "--prompt-password requires --basic-user / --username.",
        ):
            access_utils.resolve_auth(args)

    def test_resolve_auth_rejects_prompt_with_explicit_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="admin",
            password="secret",
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            access_utils.GrafanaError,
            "Choose either --basic-password / --password or --prompt-password, not both.",
        ):
            access_utils.resolve_auth(args)

    def test_validate_user_list_auth_rejects_global_token_auth(self):
        args = argparse.Namespace(scope="global", with_teams=False)

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires Basic auth"):
            access_utils.validate_user_list_auth(args, "token")

    def test_validate_user_list_auth_rejects_with_teams_token_auth(self):
        args = argparse.Namespace(scope="org", with_teams=True)

        with self.assertRaisesRegex(access_utils.GrafanaError, "--with-teams requires Basic auth"):
            access_utils.validate_user_list_auth(args, "token")

    def test_validate_user_add_auth_rejects_token_auth(self):
        with self.assertRaisesRegex(access_utils.GrafanaError, "User add requires Basic auth"):
            access_utils.validate_user_add_auth("token")

    def test_validate_user_modify_auth_rejects_token_auth(self):
        with self.assertRaisesRegex(access_utils.GrafanaError, "User modify requires Basic auth"):
            access_utils.validate_user_modify_auth("token")

    def test_validate_user_delete_auth_rejects_global_token_auth(self):
        args = argparse.Namespace(scope="global")

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires Basic auth"):
            access_utils.validate_user_delete_auth(args, "token")

    def test_validate_user_delete_args_requires_confirmation(self):
        args = argparse.Namespace(yes=False)

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires --yes"):
            access_utils.validate_user_delete_args(args)

    def test_validate_user_modify_args_requires_changes(self):
        args = argparse.Namespace(
            set_login=None,
            set_email=None,
            set_name=None,
            set_password=None,
            set_org_role=None,
            set_grafana_admin=None,
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires at least one"):
            access_utils.validate_user_modify_args(args)

    def test_validate_team_modify_args_requires_changes(self):
        args = argparse.Namespace(
            add_member=[],
            remove_member=[],
            add_admin=[],
            remove_admin=[],
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires at least one"):
            access_utils.validate_team_modify_args(args)

    def test_list_users_with_client_filters_org_users(self):
        client = FakeAccessClient(
            org_users=[
                {
                    "userId": 2,
                    "login": "bob",
                    "email": "bob@example.com",
                    "name": "Bob",
                    "role": "Editor",
                },
                {
                    "userId": 1,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "role": "Admin",
                },
            ]
        )
        args = argparse.Namespace(
            scope="org",
            query="alice",
            login=None,
            email=None,
            org_role="Admin",
            grafana_admin=None,
            with_teams=False,
            page=1,
            per_page=10,
            csv=False,
            json=False,
            table=False,
            url="http://127.0.0.1:3000",
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.list_users_with_client(args, client)

        self.assertEqual(result, 0)
        rendered = output.getvalue()
        self.assertIn("login=alice", rendered)
        self.assertNotIn("login=bob", rendered)

    def test_build_user_rows_supports_global_scope_with_teams(self):
        client = FakeAccessClient(
            global_users=[
                {
                    "id": 9,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "isAdmin": True,
                }
            ],
            teams_by_user_id={
                "9": [
                    {"name": "Ops"},
                    {"name": "SRE"},
                ]
            },
        )
        args = argparse.Namespace(
            scope="global",
            query=None,
            login=None,
            email=None,
            org_role=None,
            grafana_admin="true",
            with_teams=True,
            page=1,
            per_page=20,
        )

        users = access_utils.build_user_rows(client, args)

        self.assertEqual(client.global_page_sizes, [100])
        self.assertEqual(client.team_lookups, ["9"])
        self.assertEqual(len(users), 1)
        self.assertEqual(users[0]["teams"], ["Ops", "SRE"])

    def test_render_user_json_is_machine_readable(self):
        payload = access_utils.render_user_json(
            [
                {
                    "id": 1,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "orgRole": "Admin",
                    "grafanaAdmin": True,
                    "scope": "org",
                    "teams": ["Ops"],
                }
            ]
        )

        self.assertIn('"login": "alice"', payload)
        self.assertIn('"teams": [', payload)

    def test_build_team_rows_filters_and_attaches_members(self):
        client = FakeAccessClient(
            teams=[
                {
                    "id": 3,
                    "name": "Ops",
                    "email": "ops@example.com",
                    "memberCount": 2,
                },
                {
                    "id": 8,
                    "name": "Platform",
                    "email": "platform@example.com",
                    "memberCount": 1,
                },
            ],
            team_members_by_team_id={
                "3": [
                    {"login": "alice"},
                    {"login": "bob"},
                ]
            },
        )
        args = argparse.Namespace(
            query="ops",
            name="Ops",
            with_members=True,
            page=1,
            per_page=10,
        )

        teams = access_utils.build_team_rows(client, args)

        self.assertEqual(client.team_searches, [("ops", "iter", 100)])
        self.assertEqual(client.team_member_lookups, ["3"])
        self.assertEqual(len(teams), 1)
        self.assertEqual(teams[0]["members"], ["alice", "bob"])

    def test_render_team_json_is_machine_readable(self):
        payload = access_utils.render_team_json(
            [
                {
                    "id": "3",
                    "name": "Ops",
                    "email": "ops@example.com",
                    "memberCount": "2",
                    "members": ["alice", "bob"],
                }
            ]
        )

        self.assertIn('"name": "Ops"', payload)
        self.assertIn('"members": [', payload)

    def test_list_teams_with_client_renders_table(self):
        client = FakeAccessClient(
            teams=[
                {
                    "id": 3,
                    "name": "Ops",
                    "email": "ops@example.com",
                    "memberCount": 2,
                }
            ],
            team_members_by_team_id={
                "3": [{"login": "alice"}],
            },
        )
        args = argparse.Namespace(
            query="ops",
            name=None,
            with_members=True,
            page=1,
            per_page=10,
            csv=False,
            json=False,
            table=True,
            url="http://127.0.0.1:3000",
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.list_teams_with_client(args, client)

        self.assertEqual(result, 0)
        rendered = output.getvalue()
        self.assertIn("Member Logins", rendered)
        self.assertIn("alice", rendered)
        self.assertIn("Listed 1 team(s)", rendered)

    def test_modify_team_with_client_adds_and_removes_members(self):
        client = FakeAccessClient(
            org_users=[
                {"userId": 11, "login": "alice", "email": "alice@example.com"},
                {"userId": 12, "login": "bob", "email": "bob@example.com"},
            ],
            teams=[
                {"id": 3, "name": "Ops", "email": "ops@example.com"},
            ],
            team_members_by_team_id={
                "3": [
                    {
                        "userId": 12,
                        "login": "bob",
                        "email": "bob@example.com",
                    }
                ]
            },
        )
        args = argparse.Namespace(
            team_id="3",
            name=None,
            add_member=["alice"],
            remove_member=["bob"],
            add_admin=[],
            remove_admin=[],
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.modify_team_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.team_gets, ["3"])
        self.assertEqual(client.added_team_members, [("3", "11")])
        self.assertEqual(client.removed_team_members, [("3", "12")])
        self.assertEqual(client.updated_team_memberships, [])
        self.assertIn('"addedMembers": [', output.getvalue())
        self.assertIn('"removedMembers": [', output.getvalue())

    def test_add_team_with_client_creates_team_and_initial_memberships(self):
        client = FakeAccessClient(
            org_users=[
                {"userId": 11, "login": "alice", "email": "alice@example.com"},
                {"userId": 12, "login": "owner", "email": "owner@example.com"},
            ]
        )
        args = argparse.Namespace(
            name="Ops",
            email="ops@example.com",
            member=["alice"],
            admin=["owner@example.com"],
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_team_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.created_teams,
            [{"name": "Ops", "email": "ops@example.com"}],
        )
        self.assertEqual(client.team_gets, ["40"])
        self.assertEqual(client.team_member_lookups, [])
        self.assertEqual(client.added_team_members, [("40", "11")])
        self.assertEqual(
            client.updated_team_memberships,
            [
                (
                    "40",
                    {
                        "members": ["alice@example.com"],
                        "admins": ["owner@example.com"],
                    },
                )
            ],
        )
        self.assertIn('"teamId": "40"', output.getvalue())
        self.assertIn('"addedAdmins": [', output.getvalue())

    def test_add_team_with_client_prints_text_summary_by_default(self):
        client = FakeAccessClient(
            org_users=[
                {"userId": 11, "login": "alice", "email": "alice@example.com"},
            ]
        )
        args = argparse.Namespace(
            name="Ops",
            email="ops@example.com",
            member=["alice"],
            admin=[],
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_team_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertIn("teamId=40", output.getvalue())
        self.assertIn("name=Ops", output.getvalue())
        self.assertIn("email=ops@example.com", output.getvalue())
        self.assertIn("addedMembers=alice@example.com", output.getvalue())

    def test_add_team_with_client_requires_resolvable_initial_users(self):
        client = FakeAccessClient()
        args = argparse.Namespace(
            name="Ops",
            email=None,
            member=["missing-user"],
            admin=[],
            json=False,
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "User not found by login or email"):
            access_utils.add_team_with_client(args, client)

    def test_modify_team_with_client_updates_admins_with_bulk_payload(self):
        client = FakeAccessClient(
            org_users=[
                {"userId": 21, "login": "owner", "email": "owner@example.com"},
                {"userId": 22, "login": "member", "email": "member@example.com"},
                {"userId": 23, "login": "carol", "email": "carol@example.com"},
            ],
            teams=[
                {"id": 3, "name": "Ops", "email": "ops@example.com"},
            ],
            team_members_by_team_id={
                "3": [
                    {
                        "userId": 21,
                        "login": "owner",
                        "email": "owner@example.com",
                        "isAdmin": True,
                    },
                    {
                        "userId": 22,
                        "login": "member",
                        "email": "member@example.com",
                        "isAdmin": False,
                    },
                ]
            },
        )
        args = argparse.Namespace(
            team_id=None,
            name="Ops",
            add_member=[],
            remove_member=[],
            add_admin=["carol"],
            remove_admin=["owner@example.com"],
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.modify_team_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.added_team_members, [])
        self.assertEqual(client.removed_team_members, [])
        self.assertEqual(
            client.updated_team_memberships,
            [
                (
                    "3",
                    {
                        "members": [
                            "member@example.com",
                            "owner@example.com",
                        ],
                        "admins": ["carol@example.com"],
                    },
                )
            ],
        )
        rendered = output.getvalue()
        self.assertIn("addedAdmins=carol@example.com", rendered)
        self.assertIn("removedAdmins=owner@example.com", rendered)

    def test_modify_team_with_client_rejects_admin_changes_without_metadata(self):
        client = FakeAccessClient(
            org_users=[
                {"userId": 23, "login": "carol", "email": "carol@example.com"},
            ],
            teams=[
                {"id": 3, "name": "Ops", "email": "ops@example.com"},
            ],
            team_members_by_team_id={
                "3": [
                    {
                        "userId": 21,
                        "login": "owner",
                        "email": "owner@example.com",
                    }
                ]
            },
        )
        args = argparse.Namespace(
            team_id="3",
            name=None,
            add_member=[],
            remove_member=[],
            add_admin=["carol"],
            remove_admin=[],
            json=True,
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "admin state metadata"):
            access_utils.modify_team_with_client(args, client)

    def test_list_service_accounts_with_client_renders_json(self):
        client = FakeAccessClient(
            service_accounts=[
                {
                    "id": 2,
                    "name": "access-cli-test",
                    "login": "sa-1-access-cli-test",
                    "role": "Admin",
                    "isDisabled": False,
                    "tokens": 1,
                    "orgId": 1,
                }
            ]
        )
        args = argparse.Namespace(
            query="access",
            page=1,
            per_page=10,
            csv=False,
            json=True,
            table=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.list_service_accounts_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.service_account_searches, [("access", 1, 10)])
        self.assertIn('"name": "access-cli-test"', output.getvalue())

    def test_add_user_with_client_uses_expected_payload_and_follow_up_calls(self):
        client = FakeAccessClient()
        args = argparse.Namespace(
            login="alice",
            email="alice@example.com",
            name="Alice",
            new_user_password="secret123",
            org_id="7",
            org_role="Editor",
            grafana_admin="true",
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_user_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.created_users,
            [
                {
                    "name": "Alice",
                    "email": "alice@example.com",
                    "login": "alice",
                    "password": "secret123",
                    "OrgId": "7",
                }
            ],
        )
        self.assertEqual(client.updated_user_org_roles, [("31", "Editor")])
        self.assertEqual(client.updated_user_permissions, [("31", True)])
        self.assertIn('"login": "alice"', output.getvalue())
        self.assertIn('"orgRole": "Editor"', output.getvalue())

    def test_modify_user_with_client_updates_all_supported_fields(self):
        client = FakeAccessClient(
            global_users=[
                {
                    "id": 9,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "isAdmin": False,
                }
            ]
        )
        args = argparse.Namespace(
            user_id=None,
            login="alice",
            email=None,
            set_login="alice2",
            set_email="alice2@example.com",
            set_name="Alice Two",
            set_password="new-secret",
            set_org_role="Admin",
            set_grafana_admin="true",
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.modify_user_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.updated_users,
            [
                (
                    "9",
                    {
                        "login": "alice2",
                        "email": "alice2@example.com",
                        "name": "Alice Two",
                    },
                )
            ],
        )
        self.assertEqual(client.updated_user_passwords, [("9", "new-secret")])
        self.assertEqual(client.updated_user_org_roles, [("9", "Admin")])
        self.assertEqual(client.updated_user_permissions, [("9", True)])
        self.assertIn('"login": "alice2"', output.getvalue())
        self.assertIn('"orgRole": "Admin"', output.getvalue())

    def test_modify_user_with_client_can_resolve_by_user_id(self):
        client = FakeAccessClient(
            global_users=[
                {
                    "id": 9,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "isAdmin": True,
                }
            ]
        )
        args = argparse.Namespace(
            user_id="9",
            login=None,
            email=None,
            set_login=None,
            set_email="alice3@example.com",
            set_name=None,
            set_password=None,
            set_org_role=None,
            set_grafana_admin=None,
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.modify_user_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.user_gets, ["9"])
        self.assertEqual(client.updated_users, [("9", {"email": "alice3@example.com"})])
        self.assertIn("Modified user alice", output.getvalue())

    def test_delete_user_with_client_deletes_global_user_by_login(self):
        client = FakeAccessClient(
            global_users=[
                {
                    "id": 9,
                    "login": "alice",
                    "email": "alice@example.com",
                    "name": "Alice",
                    "isAdmin": True,
                }
            ]
        )
        args = argparse.Namespace(
            user_id=None,
            login="alice",
            email=None,
            scope="global",
            yes=True,
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_user_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.deleted_global_users, ["9"])
        self.assertEqual(client.deleted_org_users, [])
        self.assertIn('"login": "alice"', output.getvalue())
        self.assertIn('"scope": "global"', output.getvalue())

    def test_delete_user_with_client_removes_org_user_by_user_id(self):
        client = FakeAccessClient(
            org_users=[
                {
                    "userId": 12,
                    "login": "bob",
                    "email": "bob@example.com",
                    "name": "Bob",
                    "role": "Editor",
                }
            ]
        )
        args = argparse.Namespace(
            user_id="12",
            login=None,
            email=None,
            scope="org",
            yes=True,
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_user_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.deleted_org_users, ["12"])
        self.assertEqual(client.deleted_global_users, [])
        self.assertIn("Deleted user bob", output.getvalue())
        self.assertIn("scope=org", output.getvalue())

    def test_add_service_account_with_client_uses_expected_payload(self):
        client = FakeAccessClient()
        args = argparse.Namespace(
            name="robot",
            role="None",
            disabled="true",
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_service_account_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.created_service_accounts,
            [{"name": "robot", "role": "NoBasicRole", "isDisabled": True}],
        )
        self.assertIn('"role": "None"', output.getvalue())
        self.assertIn('"disabled": "true"', output.getvalue())

    def test_lookup_service_account_id_by_name_finds_exact_match(self):
        client = FakeAccessClient(
            service_accounts=[
                {"id": 4, "name": "robot", "login": "sa-robot"},
                {"id": 5, "name": "robot-2", "login": "sa-robot-2"},
            ]
        )

        service_account_id = access_utils.lookup_service_account_id_by_name(
            client, "robot"
        )

        self.assertEqual(service_account_id, "4")
        self.assertEqual(client.service_account_searches, [("robot", 1, 100)])

    def test_add_service_account_token_with_client_resolves_name(self):
        client = FakeAccessClient(
            service_accounts=[
                {"id": 7, "name": "robot", "login": "sa-robot"},
            ]
        )
        args = argparse.Namespace(
            service_account_id=None,
            name="robot",
            token_name="robot-token",
            seconds_to_live=7200,
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_service_account_token_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.created_service_account_tokens,
            [("7", {"name": "robot-token", "secondsToLive": 7200})],
        )
        self.assertIn('"serviceAccountId": "7"', output.getvalue())
        self.assertIn('"name": "robot-token"', output.getvalue())

    def test_main_returns_one_on_auth_error(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = access_utils.main(
                ["user", "list", "--scope", "global", "--token", "abc123"]
            )
        self.assertEqual(result, 1)


if __name__ == "__main__":
    unittest.main()
