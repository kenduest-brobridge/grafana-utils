import argparse
import ast
import importlib
import io
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path


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
        service_accounts=None,
    ):
        self.org_users = [dict(item) for item in (org_users or [])]
        self.global_users = [dict(item) for item in (global_users or [])]
        self.teams_by_user_id = {
            str(key): [dict(item) for item in value]
            for key, value in (teams_by_user_id or {}).items()
        }
        self.service_accounts = [dict(item) for item in (service_accounts or [])]
        self.global_page_sizes = []
        self.team_lookups = []
        self.service_account_searches = []
        self.created_service_accounts = []
        self.created_service_account_tokens = []

    def list_org_users(self):
        return [dict(item) for item in self.org_users]

    def iter_global_users(self, page_size):
        self.global_page_sizes.append(page_size)
        return [dict(item) for item in self.global_users]

    def list_user_teams(self, user_id):
        self.team_lookups.append(str(user_id))
        return [dict(item) for item in self.teams_by_user_id.get(str(user_id), [])]

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


class AccessCliTests(unittest.TestCase):
    def test_access_script_parses_as_python36_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 6))

    def test_access_wrapper_script_parses_as_python36_syntax(self):
        source = WRAPPER_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(WRAPPER_PATH), feature_version=(3, 6))

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
        )

        headers, auth_mode = access_utils.resolve_auth(args)

        self.assertEqual(auth_mode, "basic")
        self.assertTrue(headers["Authorization"].startswith("Basic "))

    def test_resolve_auth_rejects_mixed_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="admin",
            password="secret",
        )

        with self.assertRaisesRegex(access_utils.GrafanaError, "Choose either token auth"):
            access_utils.resolve_auth(args)

    def test_validate_user_list_auth_rejects_global_token_auth(self):
        args = argparse.Namespace(scope="global", with_teams=False)

        with self.assertRaisesRegex(access_utils.GrafanaError, "requires Basic auth"):
            access_utils.validate_user_list_auth(args, "token")

    def test_validate_user_list_auth_rejects_with_teams_token_auth(self):
        args = argparse.Namespace(scope="org", with_teams=True)

        with self.assertRaisesRegex(access_utils.GrafanaError, "--with-teams requires Basic auth"):
            access_utils.validate_user_list_auth(args, "token")

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
