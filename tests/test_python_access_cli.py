import argparse
import ast
import importlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "access_cli.py"
PARSER_MODULE_PATH = REPO_ROOT / "grafana_utils" / "access" / "parser.py"
WORKFLOWS_MODULE_PATH = REPO_ROOT / "grafana_utils" / "access" / "workflows.py"
CLIENT_MODULE_PATH = REPO_ROOT / "grafana_utils" / "clients" / "access_client.py"
MODELS_MODULE_PATH = REPO_ROOT / "grafana_utils" / "access" / "models.py"
MODULE_ENTRYPOINT_PATH = REPO_ROOT / "grafana_utils" / "__main__.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
access_utils = importlib.import_module("grafana_utils.access_cli")
access_client_module = importlib.import_module("grafana_utils.clients.access_client")


class FakeAccessClient:
    def __init__(
        self,
        org_users=None,
        organizations=None,
        organization_users_by_org_id=None,
        global_users=None,
        teams_by_user_id=None,
        teams=None,
        team_members_by_team_id=None,
        service_accounts=None,
        service_account_tokens_by_id=None,
    ):
        self.org_users = [dict(item) for item in (org_users or [])]
        self.organizations = [dict(item) for item in (organizations or [])]
        self.organization_users_by_org_id = {
            str(key): [dict(item) for item in value]
            for key, value in (organization_users_by_org_id or {}).items()
        }
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
        self.service_account_tokens_by_id = {
            str(key): [dict(item) for item in value]
            for key, value in (service_account_tokens_by_id or {}).items()
        }
        self.global_page_sizes = []
        self.team_lookups = []
        self.team_searches = []
        self.team_member_lookups = []
        self.service_account_searches = []
        self.service_account_gets = []
        self.service_account_token_lookups = []
        self.created_service_accounts = []
        self.created_service_account_tokens = []
        self.deleted_teams = []
        self.deleted_service_accounts = []
        self.deleted_service_account_tokens = []
        self.updated_service_accounts = []
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
        self.organization_gets = []
        self.organization_user_lookups = []
        self.created_organizations = []
        self.updated_organizations = []
        self.deleted_organizations = []
        self.added_organization_users = []
        self.updated_organization_user_roles = []
        self.deleted_organization_users = []

    def list_org_users(self):
        return [dict(item) for item in self.org_users]

    def list_organizations(self):
        return [dict(item) for item in self.organizations]

    def get_organization(self, org_id):
        self.organization_gets.append(str(org_id))
        for item in self.organizations:
            if str(item.get("id")) == str(org_id):
                return dict(item)
        return {"id": org_id, "name": ""}

    def list_organization_users(self, org_id):
        self.organization_user_lookups.append(str(org_id))
        return [
            dict(item)
            for item in self.organization_users_by_org_id.get(str(org_id), [])
        ]

    def create_organization(self, payload):
        self.created_organizations.append(dict(payload))
        org_id = str(len(self.organizations) + 1)
        created = {
            "id": org_id,
            "name": payload.get("name", ""),
        }
        self.organizations.append(dict(created))
        self.organization_users_by_org_id.setdefault(org_id, [])
        return {
            "orgId": org_id,
            "message": "Organization created",
        }

    def update_organization(self, org_id, payload):
        self.updated_organizations.append((str(org_id), dict(payload)))
        for item in self.organizations:
            if str(item.get("id")) == str(org_id):
                item["name"] = payload.get("name", item.get("name", ""))
                return {"message": "Organization updated"}
        return {"message": "Organization updated"}

    def delete_organization(self, org_id):
        self.deleted_organizations.append(str(org_id))
        return {"message": "Organization deleted"}

    def add_user_to_organization(self, org_id, payload):
        org_key = str(org_id)
        self.added_organization_users.append((org_key, dict(payload)))
        user = {
            "userId": str(len(self.organization_users_by_org_id.get(org_key, [])) + 100),
            "login": payload.get("loginOrEmail", ""),
            "email": payload.get("loginOrEmail", ""),
            "name": payload.get("loginOrEmail", ""),
            "role": payload.get("role", ""),
        }
        self.organization_users_by_org_id.setdefault(org_key, []).append(user)
        return {"message": "Organization user added"}

    def update_organization_user_role(self, org_id, user_id, role):
        self.updated_organization_user_roles.append((str(org_id), str(user_id), role))
        return {"message": "Organization user updated"}

    def delete_organization_user(self, org_id, user_id):
        self.deleted_organization_users.append((str(org_id), str(user_id)))
        return {"message": "Organization user removed"}

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

    def delete_team(self, team_id):
        self.deleted_teams.append(str(team_id))
        return {"message": "Team deleted"}

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

    def get_service_account(self, service_account_id):
        self.service_account_gets.append(str(service_account_id))
        for item in self.service_accounts:
            if str(item.get("id")) == str(service_account_id):
                return dict(item)
        return {"id": service_account_id, "name": ""}

    def delete_service_account(self, service_account_id):
        self.deleted_service_accounts.append(str(service_account_id))
        return {"message": "Service account deleted"}

    def update_service_account(self, service_account_id, payload):
        self.updated_service_accounts.append((str(service_account_id), dict(payload)))
        return {
            "id": service_account_id,
            "name": payload.get("name", ""),
            "login": "sa-updated",
            "role": payload.get("role", "Viewer"),
            "isDisabled": payload.get("isDisabled", False),
            "tokens": 0,
            "orgId": 1,
        }

    def list_service_account_tokens(self, service_account_id):
        self.service_account_token_lookups.append(str(service_account_id))
        return [
            dict(item)
            for item in self.service_account_tokens_by_id.get(
                str(service_account_id), []
            )
        ]

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

    def delete_service_account_token(self, service_account_id, token_id):
        self.deleted_service_account_tokens.append(
            (str(service_account_id), str(token_id))
        )
        return {"message": "Service account token deleted"}

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
    def test_access_script_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_access_client_module_parses_as_python39_syntax(self):
        source = CLIENT_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(CLIENT_MODULE_PATH), feature_version=(3, 9))

    def test_access_client_update_service_account_uses_patch(self):
        transport = mock.Mock()
        transport.request_json.return_value = {
            "id": 9,
            "name": "deploy-bot",
            "role": "Viewer",
            "isDisabled": False,
        }
        client = access_client_module.GrafanaAccessClient(
            base_url="http://grafana.example",
            headers={},
            timeout=30,
            verify_ssl=False,
            transport=transport,
        )

        payload = {"name": "deploy-bot", "role": "Viewer"}
        result = client.update_service_account(9, payload)

        self.assertEqual(result["id"], 9)
        transport.request_json.assert_called_once_with(
            path="/api/serviceaccounts/9",
            params=None,
            method="PATCH",
            payload=payload,
        )

    def test_access_parser_module_parses_as_python39_syntax(self):
        source = PARSER_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(PARSER_MODULE_PATH), feature_version=(3, 9))

    def test_access_workflows_module_parses_as_python39_syntax(self):
        source = WORKFLOWS_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(WORKFLOWS_MODULE_PATH), feature_version=(3, 9))

    def test_access_models_module_parses_as_python39_syntax(self):
        source = MODELS_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODELS_MODULE_PATH), feature_version=(3, 9))

    def test_access_module_entrypoint_parses_as_python39_syntax(self):
        source = MODULE_ENTRYPOINT_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_ENTRYPOINT_PATH), feature_version=(3, 9))

    def test_access_module_entrypoint_uses_unified_main(self):
        source = MODULE_ENTRYPOINT_PATH.read_text(encoding="utf-8")
        self.assertIn("from .unified_cli import main", source)

    def test_parse_args_without_command_prints_top_level_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args([])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("team", stdout.getvalue())
        self.assertIn("service-account", stdout.getvalue())

    def test_parse_args_supports_http_transport_selection(self):
        args = access_utils.parse_args(
            ["user", "list", "--http-transport", "requests"]
        )

        self.assertEqual(args.http_transport, "requests")

    def test_run_passes_http_transport_selection(self):
        args = argparse.Namespace(
            url="http://127.0.0.1:3000",
            api_token="abc123",
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=False,
            timeout=30,
            verify_ssl=False,
            http_transport="requests",
        )

        with mock.patch.object(access_utils, "GrafanaAccessClient") as client_cls:
            with mock.patch.object(
                access_utils, "dispatch_access_command", return_value=0
            ) as dispatch:
                access_utils.run(args)

        client = client_cls.return_value
        client_cls.assert_called_once_with(
            base_url="http://127.0.0.1:3000",
            headers={"Authorization": "Bearer abc123"},
            timeout=30,
            verify_ssl=False,
            transport_name="requests",
        )
        dispatch.assert_called_once_with(args, client, "token")

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

    def test_parse_args_group_without_subcommand_prints_team_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                access_utils.parse_args(["group"])

        self.assertEqual(exc.exception.code, 0)
        self.assertIn("delete", stdout.getvalue())

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

    def test_parse_args_supports_access_list_output_format(self):
        user_args = access_utils.parse_args(
            ["user", "list", "--output-format", "json"]
        )
        org_args = access_utils.parse_args(
            ["org", "list", "--output-format", "table"]
        )
        team_args = access_utils.parse_args(
            ["team", "list", "--output-format", "csv"]
        )
        service_account_args = access_utils.parse_args(
            ["service-account", "list", "--output-format", "table"]
        )

        self.assertTrue(user_args.json)
        self.assertFalse(user_args.table)
        self.assertTrue(org_args.table)
        self.assertFalse(org_args.json)
        self.assertTrue(team_args.csv)
        self.assertFalse(team_args.json)
        self.assertTrue(service_account_args.table)
        self.assertFalse(service_account_args.csv)

    def test_parse_args_rejects_access_output_format_with_legacy_flags(self):
        with self.assertRaises(SystemExit):
            access_utils.parse_args(["user", "list", "--output-format", "table", "--json"])

    def test_parse_args_supports_org_list_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "list",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--name",
                "Main Org.",
                "--query",
                "main",
                "--with-users",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "list")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.name, "Main Org.")
        self.assertEqual(args.query, "main")
        self.assertTrue(args.with_users)
        self.assertTrue(args.json)

    def test_parse_args_supports_org_add_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "add",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--name",
                "Platform",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "add")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.name, "Platform")
        self.assertTrue(args.json)

    def test_parse_args_supports_org_modify_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "modify",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--org-id",
                "7",
                "--set-name",
                "Platform Two",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "modify")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.target_org_id, "7")
        self.assertEqual(args.set_name, "Platform Two")
        self.assertTrue(args.json)

    def test_parse_args_supports_org_delete_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "delete",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--name",
                "Platform",
                "--yes",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "delete")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "grafana-secret")
        self.assertEqual(args.name, "Platform")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_parse_args_supports_org_export_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "export",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--export-dir",
                "tmp-access-orgs",
                "--name",
                "Platform",
                "--with-users",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "tmp-access-orgs")
        self.assertEqual(args.name, "Platform")
        self.assertTrue(args.with_users)

    def test_parse_args_supports_org_import_mode(self):
        args = access_utils.parse_args(
            [
                "org",
                "import",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--import-dir",
                "tmp-access-orgs",
                "--replace-existing",
                "--dry-run",
                "--yes",
            ]
        )

        self.assertEqual(args.resource, "org")
        self.assertEqual(args.command, "import")
        self.assertEqual(args.import_dir, "tmp-access-orgs")
        self.assertTrue(args.replace_existing)
        self.assertTrue(args.dry_run)
        self.assertTrue(args.yes)

    def test_org_help_uses_basic_auth_and_no_token_flags(self):
        parser = access_utils.build_parser()
        org_list_parser = parser._subparsers._group_actions[0].choices["org"]._subparsers._group_actions[0].choices["list"]
        help_text = org_list_parser.format_help()

        self.assertIn("Connection And Auth:", help_text)
        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--basic-password PASSWORD", help_text)
        self.assertIn("--with-users", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util access org list --url http://localhost:3000", help_text)
        self.assertNotIn("--token", help_text)

    def test_access_root_and_service_account_group_help_include_examples(self):
        parser = access_utils.build_parser()
        root_help = parser.format_help()
        service_account_help = (
            parser._subparsers._group_actions[0].choices["service-account"].format_help()
        )

        self.assertIn("Examples:", root_help)
        self.assertIn("grafana-util access user list", root_help)
        self.assertIn("grafana-util access org export", root_help)
        self.assertIn("Examples:", service_account_help)
        self.assertIn("grafana-util access service-account list", service_account_help)
        self.assertIn("grafana-util access service-account token add", service_account_help)

    def test_user_and_service_account_token_group_help_include_examples(self):
        parser = access_utils.build_parser()
        user_help = parser._subparsers._group_actions[0].choices["user"].format_help()
        token_help = (
            parser._subparsers._group_actions[0]
            .choices["service-account"]
            ._subparsers._group_actions[0]
            .choices["token"]
            .format_help()
        )

        self.assertIn("Examples:", user_help)
        self.assertIn("grafana-util access user list", user_help)
        self.assertIn("grafana-util access user export", user_help)
        self.assertIn("Examples:", token_help)
        self.assertIn("grafana-util access service-account token add", token_help)
        self.assertIn("grafana-util access service-account token delete", token_help)

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

    def test_parse_args_supports_user_add_password_file_mode(self):
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
                "--password-file",
                "/tmp/alice-password.txt",
            ]
        )

        self.assertEqual(args.new_user_password_file, "/tmp/alice-password.txt")
        self.assertFalse(args.prompt_user_password)

    def test_parse_args_supports_user_add_prompt_password_mode(self):
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
                "--prompt-user-password",
            ]
        )

        self.assertTrue(args.prompt_user_password)
        self.assertIsNone(args.new_user_password_file)

    def test_parse_args_rejects_multiple_user_add_password_sources(self):
        with self.assertRaises(SystemExit):
            access_utils.parse_args(
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
                    "--password-file",
                    "/tmp/alice-password.txt",
                ]
            )

    def test_parse_args_supports_user_modify_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "modify",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--org-id",
                "7",
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
        self.assertEqual(args.org_id, "7")
        self.assertEqual(args.login, "alice")
        self.assertEqual(args.set_login, "alice2")
        self.assertEqual(args.set_email, "alice2@example.com")
        self.assertEqual(args.set_name, "Alice Two")
        self.assertEqual(args.set_password, "new-secret")
        self.assertEqual(args.set_org_role, "Admin")
        self.assertEqual(args.set_grafana_admin, "true")
        self.assertTrue(args.json)

    def test_parse_args_supports_user_modify_password_file_mode(self):
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
                "--set-password-file",
                "/tmp/alice-password.txt",
            ]
        )

        self.assertEqual(args.set_password_file, "/tmp/alice-password.txt")
        self.assertFalse(args.prompt_set_password)

    def test_parse_args_supports_user_modify_prompt_password_mode(self):
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
                "--prompt-set-password",
            ]
        )

        self.assertTrue(args.prompt_set_password)
        self.assertIsNone(args.set_password_file)

    def test_parse_args_rejects_multiple_user_modify_password_sources(self):
        with self.assertRaises(SystemExit):
            access_utils.parse_args(
                [
                    "user",
                    "modify",
                    "--basic-user",
                    "admin",
                    "--basic-password",
                    "grafana-secret",
                    "--login",
                    "alice",
                    "--set-password",
                    "new-secret",
                    "--set-password-file",
                    "/tmp/alice-password.txt",
                ]
            )

    def test_parse_args_supports_user_delete_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "delete",
                "--basic-user",
                "admin",
                "--basic-password",
                "grafana-secret",
                "--org-id",
                "7",
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
        self.assertEqual(args.org_id, "7")
        self.assertEqual(args.email, "alice@example.com")
        self.assertEqual(args.scope, "org")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_parse_args_supports_user_export_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "export",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--export-dir",
                "tmp-access-users",
                "--scope",
                "global",
                "--with-teams",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "tmp-access-users")
        self.assertEqual(args.scope, "global")
        self.assertTrue(args.with_teams)

    def test_parse_args_supports_user_import_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "import",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--import-dir",
                "tmp-access-users",
                "--scope",
                "global",
                "--replace-existing",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "import")
        self.assertEqual(args.import_dir, "tmp-access-users")
        self.assertEqual(args.scope, "global")
        self.assertTrue(args.replace_existing)

    def test_parse_args_supports_user_diff_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "diff",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--diff-dir",
                "/tmp/access-users",
                "--scope",
                "global",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "diff")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "secret")
        self.assertEqual(args.diff_dir, "/tmp/access-users")
        self.assertEqual(args.scope, "global")

    def test_user_add_help_uses_basic_auth_and_local_password_flags(self):
        parser = access_utils.build_parser()
        user_add_parser = parser._subparsers._group_actions[0].choices["user"]._subparsers._group_actions[0].choices["add"]
        help_text = user_add_parser.format_help()

        self.assertIn("Connection And Auth:", help_text)
        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--basic-password PASSWORD", help_text)
        self.assertIn("--password NEW_USER_PASSWORD", help_text)
        self.assertIn("--password-file NEW_USER_PASSWORD_FILE", help_text)
        self.assertIn("--prompt-user-password", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util access user add --url http://localhost:3000", help_text)
        self.assertNotIn("--token", help_text)

    def test_user_modify_help_uses_basic_auth_only(self):
        parser = access_utils.build_parser()
        user_modify_parser = parser._subparsers._group_actions[0].choices["user"]._subparsers._group_actions[0].choices["modify"]
        help_text = user_modify_parser.format_help()

        self.assertIn("--basic-user USERNAME", help_text)
        self.assertIn("--basic-password PASSWORD", help_text)
        self.assertIn("--set-password SET_PASSWORD", help_text)
        self.assertIn("--set-password-file SET_PASSWORD_FILE", help_text)
        self.assertIn("--prompt-set-password", help_text)
        self.assertNotIn("--token", help_text)

    def test_org_modify_help_groups_target_updates_and_output(self):
        parser = access_utils.build_parser()
        org_modify_parser = (
            parser._subparsers._group_actions[0]
            .choices["org"]
            ._subparsers._group_actions[0]
            .choices["modify"]
        )
        help_text = org_modify_parser.format_help()

        self.assertIn("Target Selection:", help_text)
        self.assertIn("Org Updates:", help_text)
        self.assertIn("Output Options:", help_text)

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

    def test_parse_args_supports_team_export_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "export",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--export-dir",
                "tmp-access-teams",
                "--with-members",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "tmp-access-teams")
        self.assertTrue(args.with_members)

    def test_parse_args_supports_team_import_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "import",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--import-dir",
                "tmp-access-teams",
                "--replace-existing",
                "--yes",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "import")
        self.assertEqual(args.import_dir, "tmp-access-teams")
        self.assertTrue(args.replace_existing)
        self.assertTrue(args.yes)

    def test_parse_args_supports_team_diff_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "diff",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--diff-dir",
                "/tmp/access-teams",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "diff")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "secret")
        self.assertEqual(args.diff_dir, "/tmp/access-teams")

    def test_team_add_help_describes_initial_members_and_admins(self):
        parser = access_utils.build_parser()
        team_add_parser = parser._subparsers._group_actions[0].choices["team"]._subparsers._group_actions[0].choices["add"]
        help_text = team_add_parser.format_help()

        self.assertIn("Team Definition:", help_text)
        self.assertIn("Team Membership:", help_text)
        self.assertIn("Output Options:", help_text)
        self.assertIn("--name NAME", help_text)
        self.assertIn("--email EMAIL", help_text)
        self.assertIn("--member LOGIN_OR_EMAIL", help_text)
        self.assertIn("--admin LOGIN_OR_EMAIL", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util access team add --url http://localhost:3000", help_text)

    def test_service_account_token_add_help_includes_examples(self):
        parser = access_utils.build_parser()
        token_add_parser = (
            parser._subparsers._group_actions[0]
            .choices["service-account"]
            ._subparsers._group_actions[0]
            .choices["token"]
            ._subparsers._group_actions[0]
            .choices["add"]
        )
        help_text = token_add_parser.format_help()

        self.assertIn("Connection And Auth:", help_text)
        self.assertIn("Target Selection:", help_text)
        self.assertIn("Token Settings:", help_text)
        self.assertIn("Output Options:", help_text)
        self.assertIn("--token-name TOKEN_NAME", help_text)
        self.assertIn("--seconds-to-live SECONDS_TO_LIVE", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn(
            "grafana-util access service-account token add --url http://localhost:3000",
            help_text,
        )

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

    def test_parse_args_supports_team_delete_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "delete",
                "--name",
                "Ops",
                "--yes",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "delete")
        self.assertEqual(args.name, "Ops")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_parse_args_supports_user_export_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "export",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--export-dir",
                "tmp-users",
                "--scope",
                "global",
                "--with-teams",
                "--overwrite",
                "--dry-run",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "secret")
        self.assertEqual(args.export_dir, "tmp-users")
        self.assertEqual(args.scope, "global")
        self.assertTrue(args.with_teams)
        self.assertTrue(args.overwrite)
        self.assertTrue(args.dry_run)

    def test_parse_args_supports_user_import_mode(self):
        args = access_utils.parse_args(
            [
                "user",
                "import",
                "--token",
                "abc123",
                "--import-dir",
                "tmp-users",
                "--scope",
                "org",
                "--replace-existing",
                "--yes",
            ]
        )

        self.assertEqual(args.resource, "user")
        self.assertEqual(args.command, "import")
        self.assertEqual(args.api_token, "abc123")
        self.assertEqual(args.import_dir, "tmp-users")
        self.assertEqual(args.scope, "org")
        self.assertTrue(args.replace_existing)
        self.assertTrue(args.yes)

    def test_parse_args_supports_team_export_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "export",
                "--token",
                "abc123",
                "--export-dir",
                "tmp-teams",
                "--with-members",
                "--overwrite",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.api_token, "abc123")
        self.assertEqual(args.export_dir, "tmp-teams")
        self.assertTrue(args.with_members)
        self.assertTrue(args.overwrite)

    def test_parse_args_supports_team_import_mode(self):
        args = access_utils.parse_args(
            [
                "team",
                "import",
                "--basic-user",
                "admin",
                "--basic-password",
                "secret",
                "--import-dir",
                "tmp-teams",
                "--replace-existing",
                "--yes",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "import")
        self.assertEqual(args.auth_username, "admin")
        self.assertEqual(args.auth_password, "secret")
        self.assertEqual(args.import_dir, "tmp-teams")
        self.assertTrue(args.replace_existing)
        self.assertTrue(args.yes)

    def test_parse_args_supports_group_alias_mode(self):
        args = access_utils.parse_args(
            [
                "group",
                "delete",
                "--team-id",
                "7",
                "--yes",
            ]
        )

        self.assertEqual(args.resource, "team")
        self.assertEqual(args.command, "delete")
        self.assertEqual(args.team_id, "7")
        self.assertTrue(args.yes)

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

    def test_parse_args_rejects_legacy_basic_auth_aliases(self):
        with self.assertRaises(SystemExit):
            access_utils.parse_args(["user", "list", "--username", "admin", "--basic-password", "secret"])
        with self.assertRaises(SystemExit):
            access_utils.parse_args(["user", "list", "--basic-user", "admin", "--password", "secret"])

    def test_parse_args_supports_prompt_password(self):
        args = access_utils.parse_args(
            ["user", "list", "--basic-user", "admin", "--prompt-password"]
        )

        self.assertEqual(args.username, "admin")
        self.assertIsNone(args.password)
        self.assertTrue(args.prompt_password)

    def test_parse_args_supports_prompt_token(self):
        args = access_utils.parse_args(["user", "list", "--prompt-token"])

        self.assertTrue(args.prompt_token)
        self.assertIsNone(args.api_token)

    def test_parse_args_supports_service_account_export(self):
        args = access_utils.parse_args(
            [
                "service-account",
                "export",
                "--export-dir",
                "tmp-service-accounts",
                "--overwrite",
                "--dry-run",
            ]
        )

        self.assertEqual(args.resource, "service-account")
        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "tmp-service-accounts")
        self.assertTrue(args.overwrite)
        self.assertTrue(args.dry_run)

    def test_parse_args_supports_service_account_import_and_diff(self):
        import_args = access_utils.parse_args(
            [
                "service-account",
                "import",
                "--import-dir",
                "tmp-service-accounts",
                "--replace-existing",
                "--dry-run",
                "--output-format",
                "table",
                "--error-policy",
                "continue",
                "--yes",
            ]
        )

        self.assertEqual(import_args.resource, "service-account")
        self.assertEqual(import_args.command, "import")
        self.assertEqual(import_args.import_dir, "tmp-service-accounts")
        self.assertTrue(import_args.replace_existing)
        self.assertTrue(import_args.dry_run)
        self.assertTrue(import_args.table)
        self.assertEqual(import_args.error_policy, "continue")
        self.assertTrue(import_args.yes)

        diff_args = access_utils.parse_args(
            [
                "service-account",
                "diff",
                "--diff-dir",
                "/tmp/service-accounts",
            ]
        )

        self.assertEqual(diff_args.resource, "service-account")
        self.assertEqual(diff_args.command, "diff")
        self.assertEqual(diff_args.diff_dir, "/tmp/service-accounts")
        self.assertEqual(diff_args.error_policy, "abort")

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

    def test_parse_args_supports_service_account_delete(self):
        args = access_utils.parse_args(
            [
                "service-account",
                "delete",
                "--service-account-id",
                "7",
                "--yes",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "service-account")
        self.assertEqual(args.command, "delete")
        self.assertEqual(args.service_account_id, "7")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_parse_args_supports_service_account_token_delete(self):
        args = access_utils.parse_args(
            [
                "service-account",
                "token",
                "delete",
                "--name",
                "robot",
                "--token-name",
                "robot-token",
                "--yes",
                "--json",
            ]
        )

        self.assertEqual(args.resource, "service-account")
        self.assertEqual(args.command, "token")
        self.assertEqual(args.token_command, "delete")
        self.assertEqual(args.name, "robot")
        self.assertEqual(args.token_name, "robot-token")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_build_request_headers_adds_org_id(self):
        args = argparse.Namespace(
            api_token="abc123",
            prompt_token=False,
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
            prompt_token=False,
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
            prompt_token=False,
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
            prompt_token=False,
            username="admin",
            password="secret",
            prompt_password=False,
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "Choose either token auth"):
            access_utils.resolve_auth(args)

    def test_resolve_auth_supports_prompt_password(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="admin",
            password=None,
            prompt_password=True,
        )

        with mock.patch("grafana_utils.access_cli.getpass.getpass", return_value="secret") as prompt:
            headers, auth_mode = access_utils.resolve_auth(args)

        self.assertEqual(auth_mode, "basic")
        self.assertTrue(headers["Authorization"].startswith("Basic "))
        prompt.assert_called_once_with("Grafana Basic auth password: ")

    def test_resolve_auth_supports_prompt_token(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=True,
            username=None,
            password=None,
            prompt_password=False,
        )

        with mock.patch("grafana_utils.access_cli.getpass.getpass", return_value="token-secret") as prompt:
            headers, auth_mode = access_utils.resolve_auth(args)

        self.assertEqual(auth_mode, "token")
        self.assertEqual(headers["Authorization"], "Bearer token-secret")
        prompt.assert_called_once_with("Grafana API token: ")

    def test_resolve_auth_rejects_prompt_without_username(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            access_utils.GrafanaError,
            "--prompt-password requires --basic-user.",
        ):
            access_utils.resolve_auth(args)

    def test_resolve_auth_rejects_prompt_with_explicit_password(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="admin",
            password="secret",
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            access_utils.GrafanaError,
            "Choose either --basic-password or --prompt-password, not both.",
        ):
            access_utils.resolve_auth(args)

    def test_resolve_auth_rejects_explicit_and_prompt_token(self):
        args = argparse.Namespace(
            api_token="abc123",
            prompt_token=True,
            username=None,
            password=None,
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            access_utils.GrafanaError,
            "Choose either --token / --api-token or --prompt-token, not both.",
        ):
            access_utils.resolve_auth(args)

    def test_resolve_user_secret_inputs_reads_new_user_password_file(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            password_path = Path(temp_dir) / "new-user-password.txt"
            password_path.write_text("secret123\n", encoding="utf-8")
            args = argparse.Namespace(
                resource="user",
                command="add",
                new_user_password=None,
                new_user_password_file=str(password_path),
                prompt_user_password=False,
            )

            resolved = access_utils.resolve_user_secret_inputs(args)

        self.assertEqual(resolved.new_user_password, "secret123")

    def test_resolve_user_secret_inputs_prompts_for_new_user_password(self):
        args = argparse.Namespace(
            resource="user",
            command="add",
            new_user_password=None,
            new_user_password_file=None,
            prompt_user_password=True,
        )

        with mock.patch("grafana_utils.access_cli.getpass.getpass", return_value="secret123") as prompt:
            resolved = access_utils.resolve_user_secret_inputs(args)

        self.assertEqual(resolved.new_user_password, "secret123")
        prompt.assert_called_once_with("New Grafana user password: ")

    def test_resolve_user_secret_inputs_reads_set_password_file(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            password_path = Path(temp_dir) / "set-password.txt"
            password_path.write_text("new-secret\n", encoding="utf-8")
            args = argparse.Namespace(
                resource="user",
                command="modify",
                set_password=None,
                set_password_file=str(password_path),
                prompt_set_password=False,
            )

            resolved = access_utils.resolve_user_secret_inputs(args)

        self.assertEqual(resolved.set_password, "new-secret")

    def test_resolve_user_secret_inputs_prompts_for_set_password(self):
        args = argparse.Namespace(
            resource="user",
            command="modify",
            set_password=None,
            set_password_file=None,
            prompt_set_password=True,
        )

        with mock.patch("grafana_utils.access_cli.getpass.getpass", return_value="new-secret") as prompt:
            resolved = access_utils.resolve_user_secret_inputs(args)

        self.assertEqual(resolved.set_password, "new-secret")
        prompt.assert_called_once_with("Updated Grafana user password: ")

    def test_validate_user_list_auth_rejects_global_token_auth(self):
        args = argparse.Namespace(scope="global", with_teams=False)

        with self.assertRaisesRegex(access_utils.GrafanaError, "does not support API token auth"):
            access_utils.validate_user_list_auth(args, "token")

    def test_validate_user_list_auth_rejects_with_teams_token_auth(self):
        args = argparse.Namespace(scope="org", with_teams=True)

        with self.assertRaisesRegex(access_utils.GrafanaError, "does not support API token auth"):
            access_utils.validate_user_list_auth(args, "token")

    def test_validate_user_add_auth_rejects_token_auth(self):
        with self.assertRaisesRegex(access_utils.GrafanaError, "does not support API token auth"):
            access_utils.validate_user_add_auth("token")

    def test_validate_user_modify_auth_rejects_token_auth(self):
        with self.assertRaisesRegex(access_utils.GrafanaError, "does not support API token auth"):
            access_utils.validate_user_modify_auth("token")

    def test_validate_user_delete_auth_rejects_global_token_auth(self):
        args = argparse.Namespace(scope="global")

        with self.assertRaisesRegex(access_utils.GrafanaError, "does not support API token auth"):
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

    def test_sync_team_members_for_import_updates_admin_and_membership_state(self):
        client = FakeAccessClient(org_users=[])

        existing_members = {
            "alice": {"identity": "alice@example.com", "user_id": "10", "admin": True},
            "bob": {"identity": "bob@example.com", "user_id": "11", "admin": False},
            "david": {"identity": "david@example.com", "user_id": "12", "admin": True},
        }
        with mock.patch(
            "grafana_utils.access.workflows.lookup_org_user_by_identity",
            side_effect=lambda *_args, **_kwargs: {
                "userId": "14",
                "login": "charlie",
                "email": "charlie@example.com",
            },
        ):
            summary = access_utils._sync_team_members_for_import(
                client,
                team_id="3",
                team_name="Ops",
                existing_members=existing_members,
                desired_members=["charlie", "alice"],
                desired_admins=["alice"],
                include_missing=True,
                dry_run=False,
            )

        self.assertEqual(summary["addedMembers"], ["charlie"])
        self.assertEqual(summary["addedAdmins"], [])
        self.assertEqual(summary["removedAdmins"], ["david@example.com"])
        self.assertEqual(summary["removedMembers"], ["bob@example.com", "david@example.com"])
        self.assertEqual(summary["unchangedAdmins"], ["alice@example.com"])
        self.assertEqual(client.added_team_members, [("3", "14")])
        self.assertEqual(
            client.removed_team_members,
            [("3", "11"), ("3", "12")],
        )
        self.assertEqual(
            client.updated_team_memberships,
            [
                (
                    "3",
                    {
                        "members": ["charlie"],
                        "admins": ["alice"],
                    },
                )
            ],
        )

    def test_delete_team_with_client_deletes_by_name(self):
        client = FakeAccessClient(
            teams=[
                {"id": 3, "name": "Ops", "email": "ops@example.com"},
            ]
        )
        args = argparse.Namespace(
            team_id=None,
            name="Ops",
            yes=True,
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_team_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.team_searches, [("Ops", "iter", 100)])
        self.assertEqual(client.team_gets, ["3"])
        self.assertEqual(client.deleted_teams, ["3"])
        self.assertIn('"teamId": "3"', output.getvalue())
        self.assertIn('"name": "Ops"', output.getvalue())

    def test_diff_users_with_client_returns_expected_difference_count(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            diff_dir = Path(temp_dir)
            (diff_dir / "users.json").write_text(
                json.dumps(
                    [
                        {
                            "login": "alice",
                            "email": "alice@example.com",
                            "name": "Alice",
                            "orgRole": "Admin",
                            "grafanaAdmin": True,
                        },
                        {
                            "login": "bob",
                            "email": "bob@example.com",
                            "name": "Bob",
                            "orgRole": "Viewer",
                            "grafanaAdmin": False,
                        },
                        {
                            "login": "carol",
                            "email": "carol@example.com",
                            "name": "Carol",
                            "orgRole": "Viewer",
                            "grafanaAdmin": False,
                        },
                    ]
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                diff_dir=str(diff_dir),
                scope="org",
            )
            client = FakeAccessClient(
                org_users=[
                    {
                        "userId": "11",
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "role": "Editor",
                    },
                    {
                        "userId": "12",
                        "login": "dave",
                        "email": "dave@example.com",
                        "name": "Dave",
                        "role": "Viewer",
                    },
                ],
            )
            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.diff_users_with_client(args, client)

            self.assertEqual(result, 4)
            self.assertIn("Diff checked", output.getvalue())

    def test_diff_teams_with_client_returns_expected_difference_count(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            diff_dir = Path(temp_dir)
            (diff_dir / "teams.json").write_text(
                json.dumps(
                    [
                        {"name": "Ops", "email": "ops@example.com"},
                        {"name": "Dev", "email": "dev@example.com"},
                    ]
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                diff_dir=str(diff_dir),
            )
            client = FakeAccessClient(
                teams=[
                    {"id": 3, "name": "Ops", "email": "ops-two@example.com"},
                    {"id": 5, "name": "SRE", "email": "sre@example.com"},
                ]
            )
            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.diff_teams_with_client(args, client)

            self.assertEqual(result, 3)
            self.assertIn("Diff checked", output.getvalue())

    def test_list_orgs_with_client_renders_json_with_users(self):
        client = FakeAccessClient(
            organizations=[
                {"id": 1, "name": "Main Org."},
            ],
            organization_users_by_org_id={
                "1": [
                    {
                        "userId": 11,
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "role": "Admin",
                    }
                ]
            },
        )
        args = argparse.Namespace(
            target_org_id=None,
            name=None,
            query=None,
            with_users=True,
            csv=False,
            json=True,
            table=False,
            url="http://127.0.0.1:3000",
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.list_orgs_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.organization_user_lookups, ["1"])
        self.assertIn('"name": "Main Org."', output.getvalue())
        self.assertIn('"users": [', output.getvalue())

    def test_add_org_with_client_creates_organization(self):
        client = FakeAccessClient()
        args = argparse.Namespace(
            name="Platform",
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.add_org_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.created_organizations, [{"name": "Platform"}])
        self.assertIn('"name": "Platform"', output.getvalue())

    def test_modify_org_with_client_updates_name_by_org_id(self):
        client = FakeAccessClient(
            organizations=[
                {"id": 7, "name": "Platform"},
            ]
        )
        args = argparse.Namespace(
            target_org_id="7",
            name=None,
            set_name="Platform Two",
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.modify_org_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(
            client.updated_organizations,
            [("7", {"name": "Platform Two"})],
        )
        self.assertIn("id=7", output.getvalue())
        self.assertIn("name=Platform Two", output.getvalue())

    def test_delete_org_with_client_deletes_by_name(self):
        client = FakeAccessClient(
            organizations=[
                {"id": 7, "name": "Platform"},
            ]
        )
        args = argparse.Namespace(
            target_org_id=None,
            name="Platform",
            yes=True,
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_org_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.deleted_organizations, ["7"])
        self.assertIn('"id": "7"', output.getvalue())
        self.assertIn('"name": "Platform"', output.getvalue())

    def test_export_orgs_with_client_writes_org_bundle_with_users(self):
        client = FakeAccessClient(
            organizations=[
                {"id": 1, "name": "Main Org."},
            ],
            organization_users_by_org_id={
                "1": [
                    {
                        "userId": 11,
                        "login": "alice",
                        "email": "alice@example.com",
                        "name": "Alice",
                        "role": "Admin",
                    }
                ]
            },
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            args = argparse.Namespace(
                export_dir=temp_dir,
                url="http://127.0.0.1:3000",
                target_org_id=None,
                name=None,
                with_users=True,
                overwrite=False,
                dry_run=False,
            )

            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.export_orgs_with_client(args, client)

            self.assertEqual(result, 0)
            bundle = json.loads(
                (Path(temp_dir) / "orgs.json").read_text(encoding="utf-8")
            )
            self.assertEqual(bundle["records"][0]["name"], "Main Org.")
            self.assertEqual(bundle["records"][0]["users"][0]["login"], "alice")
            self.assertIn("Exported 1 org(s)", output.getvalue())

    def test_import_orgs_with_client_creates_missing_org_and_adds_users(self):
        client = FakeAccessClient()
        with tempfile.TemporaryDirectory() as temp_dir:
            payload = {
                "kind": "grafana-utils-access-org-export-index",
                "version": 1,
                "records": [
                    {
                        "name": "Platform",
                        "users": [
                            {
                                "login": "alice",
                                "email": "alice@example.com",
                                "orgRole": "Editor",
                            }
                        ],
                    }
                ],
            }
            (Path(temp_dir) / "orgs.json").write_text(
                json.dumps(payload),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                import_dir=temp_dir,
                replace_existing=True,
                dry_run=False,
                yes=False,
            )

            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.import_orgs_with_client(args, client)

            self.assertEqual(result, 0)
            self.assertEqual(client.created_organizations, [{"name": "Platform"}])
            self.assertEqual(
                client.added_organization_users,
                [("1", {"loginOrEmail": "alice", "role": "Editor"})],
            )
            self.assertIn("Import summary:", output.getvalue())

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

    def test_export_service_accounts_with_client_writes_bundle(self):
        client = FakeAccessClient(
            service_accounts=[
                {
                    "id": 7,
                    "name": "robot",
                    "login": "sa-robot",
                    "role": "Editor",
                    "isDisabled": False,
                    "tokens": 1,
                    "orgId": 1,
                }
            ]
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            args = argparse.Namespace(
                export_dir=temp_dir,
                url="http://127.0.0.1:3000",
                dry_run=False,
                overwrite=True,
            )
            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.export_service_accounts_with_client(args, client)

            self.assertEqual(result, 0)
            bundle = json.loads((Path(temp_dir) / "service-accounts.json").read_text(encoding="utf-8"))
            self.assertEqual(bundle["kind"], "grafana-utils-access-service-account-export-index")
            self.assertEqual(bundle["records"][0]["name"], "robot")
            self.assertIn("Exported 1 service-account(s)", output.getvalue())

    def test_import_service_accounts_with_client_updates_existing(self):
        client = FakeAccessClient(
            service_accounts=[
                {
                    "id": 7,
                    "name": "robot",
                    "login": "sa-robot",
                    "role": "Viewer",
                    "isDisabled": False,
                    "tokens": 0,
                    "orgId": 1,
                }
            ]
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            diff_dir = Path(temp_dir)
            (diff_dir / "service-accounts.json").write_text(
                json.dumps(
                    {
                        "kind": "grafana-utils-access-service-account-export-index",
                        "version": 1,
                        "records": [
                            {
                                "name": "robot",
                                "role": "Editor",
                                "disabled": True,
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                import_dir=str(diff_dir),
                replace_existing=True,
                dry_run=False,
                table=False,
                json=False,
                yes=False,
            )
            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.import_service_accounts_with_client(args, client)

            self.assertEqual(result, 0)
            self.assertEqual(
                client.updated_service_accounts,
                [("7", {"name": "robot", "role": "Editor", "isDisabled": True})],
            )
            self.assertIn("Updated service-account robot", output.getvalue())

    def test_import_service_accounts_with_client_continue_policy_keeps_processing_after_item_error(self):
        client = FakeAccessClient(service_accounts=[])
        with tempfile.TemporaryDirectory() as temp_dir:
            diff_dir = Path(temp_dir)
            (diff_dir / "service-accounts.json").write_text(
                json.dumps(
                    {
                        "kind": "grafana-utils-access-service-account-export-index",
                        "version": 1,
                        "records": [
                            {"name": "broken", "role": "Viewer", "disabled": False},
                            {"name": "robot", "role": "Editor", "disabled": True},
                        ],
                    }
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                import_dir=str(diff_dir),
                replace_existing=True,
                dry_run=False,
                table=False,
                json=False,
                yes=False,
                error_policy="continue",
            )
            output = io.StringIO()
            errors = io.StringIO()
            original_create = client.create_service_account

            def flaky_create(payload):
                if payload.get("name") == "broken":
                    raise access_utils.GrafanaError("broken service account")
                return original_create(payload)

            with mock.patch.object(client, "create_service_account", side_effect=flaky_create):
                with redirect_stdout(output), redirect_stderr(errors):
                    result = access_utils.import_service_accounts_with_client(args, client)

            self.assertEqual(result, 1)
            self.assertEqual(client.created_service_accounts[0]["name"], "robot")
            self.assertIn("failed=1", output.getvalue())
            self.assertIn("Created service-account robot", output.getvalue())
            self.assertIn("Continuing after service-account import error", errors.getvalue())

    def test_diff_service_accounts_with_client_returns_expected_difference_count(self):
        client = FakeAccessClient(
            service_accounts=[
                {
                    "id": 7,
                    "name": "robot",
                    "login": "sa-robot",
                    "role": "Viewer",
                    "isDisabled": False,
                    "tokens": 0,
                    "orgId": 1,
                },
                {
                    "id": 8,
                    "name": "extra",
                    "login": "sa-extra",
                    "role": "Viewer",
                    "isDisabled": False,
                    "tokens": 0,
                    "orgId": 1,
                },
            ]
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            diff_dir = Path(temp_dir)
            (diff_dir / "service-accounts.json").write_text(
                json.dumps(
                    [
                        {"name": "robot", "role": "Editor", "disabled": False},
                        {"name": "missing", "role": "Viewer", "disabled": False},
                    ]
                ),
                encoding="utf-8",
            )
            args = argparse.Namespace(diff_dir=str(diff_dir))
            output = io.StringIO()
            with redirect_stdout(output):
                result = access_utils.diff_service_accounts_with_client(args, client)

            self.assertEqual(result, 3)
            self.assertIn("Diff checked", output.getvalue())

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

    def test_delete_service_account_with_client_deletes_by_name(self):
        client = FakeAccessClient(
            service_accounts=[
                {"id": 7, "name": "robot", "login": "sa-robot", "role": "Viewer"},
            ]
        )
        args = argparse.Namespace(
            service_account_id=None,
            name="robot",
            yes=True,
            json=False,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_service_account_with_client(args, client)

        self.assertEqual(result, 0)
        self.assertEqual(client.service_account_searches, [("robot", 1, 100)])
        self.assertEqual(client.service_account_gets, ["7"])
        self.assertEqual(client.deleted_service_accounts, ["7"])
        self.assertIn("serviceAccountId=7", output.getvalue())
        self.assertIn("name=robot", output.getvalue())

    def test_delete_service_account_token_with_client_resolves_token_name(self):
        client = FakeAccessClient(
            service_accounts=[
                {"id": 7, "name": "robot", "login": "sa-robot"},
            ],
            service_account_tokens_by_id={
                "7": [
                    {"id": 4, "name": "nightly"},
                    {"id": 5, "name": "adhoc"},
                ]
            },
        )
        args = argparse.Namespace(
            service_account_id=None,
            name="robot",
            token_id=None,
            token_name="nightly",
            yes=True,
            json=True,
        )

        output = io.StringIO()
        with redirect_stdout(output):
            result = access_utils.delete_service_account_token_with_client(
                args, client
            )

        self.assertEqual(result, 0)
        self.assertEqual(client.service_account_searches, [("robot", 1, 100)])
        self.assertEqual(client.service_account_gets, ["7"])
        self.assertEqual(client.service_account_token_lookups, ["7"])
        self.assertEqual(client.deleted_service_account_tokens, [("7", "4")])
        self.assertIn('"tokenId": "4"', output.getvalue())
        self.assertIn('"tokenName": "nightly"', output.getvalue())

    def test_dispatch_access_command_routes_user_export_import(self):
        client = FakeAccessClient()
        args = argparse.Namespace(resource="user", command="export", export_dir="tmp", url="http://127.0.0.1:3000")
        with mock.patch("grafana_utils.access.workflows.export_users_with_client", return_value=77) as export_users:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 77)
        export_users.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="user", command="import", import_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.import_users_with_client", return_value=88) as import_users:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 88)
        import_users.assert_called_once_with(args, client)

        args = argparse.Namespace(
            resource="user",
            command="diff",
            diff_dir="tmp",
            scope="org",
            with_teams=False,
        )
        with mock.patch("grafana_utils.access.workflows.diff_users_with_client", return_value=99) as diff_users:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 99)
        diff_users.assert_called_once_with(args, client)

    def test_dispatch_access_command_routes_team_export_import(self):
        client = FakeAccessClient()
        args = argparse.Namespace(resource="team", command="export", export_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.export_teams_with_client", return_value=55) as export_teams:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 55)
        export_teams.assert_called_once_with(args, client)

        args = argparse.Namespace(
            resource="team",
            command="import",
            import_dir="tmp",
            yes=True,
            replace_existing=True,
        )
        with mock.patch("grafana_utils.access.workflows.import_teams_with_client", return_value=66) as import_teams:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 66)
        import_teams.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="team", command="diff", diff_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.diff_teams_with_client", return_value=44) as diff_teams:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 44)
        diff_teams.assert_called_once_with(args, client)

    def test_dispatch_access_command_routes_org_commands(self):
        client = FakeAccessClient()

        args = argparse.Namespace(resource="org", command="list", with_users=False)
        with mock.patch("grafana_utils.access.workflows.list_orgs_with_client", return_value=12) as list_orgs:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 12)
        list_orgs.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="org", command="add", name="Platform")
        with mock.patch("grafana_utils.access.workflows.add_org_with_client", return_value=23) as add_org:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 23)
        add_org.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="org", command="modify", target_org_id="7", set_name="Platform Two")
        with mock.patch("grafana_utils.access.workflows.modify_org_with_client", return_value=34) as modify_org:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 34)
        modify_org.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="org", command="delete", target_org_id="7", yes=True)
        with mock.patch("grafana_utils.access.workflows.delete_org_with_client", return_value=45) as delete_org:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 45)
        delete_org.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="org", command="export", export_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.export_orgs_with_client", return_value=56) as export_orgs:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 56)
        export_orgs.assert_called_once_with(args, client)

        args = argparse.Namespace(
            resource="org",
            command="import",
            import_dir="tmp",
            replace_existing=True,
            dry_run=False,
            yes=False,
        )
        with mock.patch("grafana_utils.access.workflows.import_orgs_with_client", return_value=67) as import_orgs:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 67)
        import_orgs.assert_called_once_with(args, client)

    def test_dispatch_access_command_routes_service_account_export_import_diff(self):
        client = FakeAccessClient()
        args = argparse.Namespace(resource="service-account", command="export", export_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.export_service_accounts_with_client", return_value=33) as export_service_accounts:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 33)
        export_service_accounts.assert_called_once_with(args, client)

        args = argparse.Namespace(
            resource="service-account",
            command="import",
            import_dir="tmp",
            replace_existing=True,
            dry_run=True,
            table=False,
            json=False,
            yes=False,
        )
        with mock.patch("grafana_utils.access.workflows.import_service_accounts_with_client", return_value=22) as import_service_accounts:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 22)
        import_service_accounts.assert_called_once_with(args, client)

        args = argparse.Namespace(resource="service-account", command="diff", diff_dir="tmp")
        with mock.patch("grafana_utils.access.workflows.diff_service_accounts_with_client", return_value=11) as diff_service_accounts:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 11)
        diff_service_accounts.assert_called_once_with(args, client)

    def test_dispatch_access_command_routes_service_account_token_commands(self):
        client = FakeAccessClient()
        args = argparse.Namespace(
            resource="service-account",
            command="token",
            token_command="add",
            service_account="robot",
            name="nightly",
        )
        with mock.patch(
            "grafana_utils.access.workflows.add_service_account_token_with_client",
            return_value=21,
        ) as add_token:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 21)
        add_token.assert_called_once_with(args, client)

        args = argparse.Namespace(
            resource="service-account",
            command="token",
            token_command="delete",
            service_account="robot",
            token="nightly",
        )
        with mock.patch(
            "grafana_utils.access.workflows.validate_service_account_token_delete_auth"
        ) as validate_delete, mock.patch(
            "grafana_utils.access.workflows.delete_service_account_token_with_client",
            return_value=22,
        ) as delete_token:
            result = access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(result, 22)
        validate_delete.assert_called_once_with("basic")
        delete_token.assert_called_once_with(args, client)

    def test_dispatch_access_command_rejects_unsupported_command(self):
        client = FakeAccessClient()
        args = argparse.Namespace(resource="team", command="archive")
        with self.assertRaises(access_utils.GrafanaError) as ctx:
            access_utils.dispatch_access_command(args, client, "basic")
        self.assertEqual(str(ctx.exception), "Unsupported command.")

    def test_main_returns_one_on_auth_error(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = access_utils.main(
                ["user", "list", "--scope", "global", "--token", "abc123"]
            )
        self.assertEqual(result, 1)


if __name__ == "__main__":
    unittest.main()
