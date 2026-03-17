import argparse
import ast
import importlib
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
MODULE_PATH = PYTHON_ROOT / "grafana_utils" / "access" / "pending_cli_staging.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))
pending_cli = importlib.import_module("grafana_utils.access.pending_cli_staging")


class FakeAccessLookupClient:
    def __init__(self, teams=None, service_accounts=None):
        self.teams = [dict(item) for item in (teams or [])]
        self.service_accounts = [dict(item) for item in (service_accounts or [])]
        self.team_calls = []
        self.service_account_calls = []

    def iter_teams(self, query, page_size):
        self.team_calls.append((query, page_size))
        return [dict(item) for item in self.teams]

    def list_service_accounts(self, query, page, per_page):
        self.service_account_calls.append((query, page, per_page))
        return [dict(item) for item in self.service_accounts]


class PendingCliStagingTests(unittest.TestCase):
    def test_access_pending_cli_staging_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_access_pending_cli_staging_team_delete_parser_requires_identity(self):
        parser = argparse.ArgumentParser(prog="team delete")
        pending_cli.add_team_delete_cli_args(parser)

        args = parser.parse_args(["--team-id", "7", "--yes", "--json"])

        self.assertEqual(args.team_id, "7")
        self.assertTrue(args.yes)
        self.assertTrue(args.json)

    def test_access_pending_cli_staging_service_account_token_delete_parser_supports_name_lookup(
        self,
    ):
        parser = argparse.ArgumentParser(prog="service-account token delete")
        pending_cli.add_service_account_token_delete_cli_args(parser)

        args = parser.parse_args(
            ["--name", "robot", "--token-name", "nightly", "--yes"]
        )

        self.assertEqual(args.name, "robot")
        self.assertEqual(args.token_name, "nightly")
        self.assertTrue(args.yes)

    def test_access_pending_cli_staging_normalize_group_alias_argv_maps_leading_group_only(
        self,
    ):
        self.assertEqual(
            pending_cli.normalize_group_alias_argv(["group", "list"]),
            ["team", "list"],
        )
        self.assertEqual(
            pending_cli.normalize_group_alias_argv(["team", "list"]),
            ["team", "list"],
        )

    def test_access_pending_cli_staging_validate_destructive_confirmed_rejects_missing_yes(
        self,
    ):
        args = argparse.Namespace(yes=False)

        with self.assertRaisesRegex(Exception, "requires --yes"):
            pending_cli.validate_destructive_confirmed(args, "Team delete")

    def test_access_pending_cli_staging_resolve_team_id_supports_explicit_id(self):
        client = FakeAccessLookupClient()

        resolved = pending_cli.resolve_team_id(client, team_id="44")

        self.assertEqual(resolved, "44")
        self.assertEqual(client.team_calls, [])

    def test_access_pending_cli_staging_resolve_team_id_resolves_exact_name(self):
        client = FakeAccessLookupClient(
            teams=[{"id": 5, "name": "ops"}, {"id": 6, "name": "ops-east"}]
        )

        resolved = pending_cli.resolve_team_id(client, name="ops")

        self.assertEqual(resolved, "5")
        self.assertEqual(client.team_calls, [("ops", 100)])

    def test_access_pending_cli_staging_resolve_team_id_rejects_ambiguous_name(self):
        client = FakeAccessLookupClient(
            teams=[{"id": 5, "name": "ops"}, {"id": 6, "name": "ops"}]
        )

        with self.assertRaisesRegex(Exception, "matched multiple items"):
            pending_cli.resolve_team_id(client, name="ops")

    def test_access_pending_cli_staging_resolve_service_account_id_resolves_exact_name(
        self,
    ):
        client = FakeAccessLookupClient(
            service_accounts=[
                {"id": 8, "name": "robot"},
                {"id": 9, "name": "robot-2"},
            ]
        )

        resolved = pending_cli.resolve_service_account_id(client, name="robot")

        self.assertEqual(resolved, "8")
        self.assertEqual(client.service_account_calls, [("robot", 1, 100)])

    def test_access_pending_cli_staging_resolve_service_account_token_record_supports_token_name(
        self,
    ):
        resolved = pending_cli.resolve_service_account_token_record(
            [
                {"id": 2, "name": "nightly"},
                {"id": 3, "name": "adhoc"},
            ],
            token_name="nightly",
        )

        self.assertEqual(resolved["id"], 2)

    def test_access_pending_cli_staging_build_delete_request_paths_quote_ids(self):
        team_request = pending_cli.build_team_delete_request("folder/ops")
        service_account_request = pending_cli.build_service_account_delete_request(
            "svc/acct"
        )
        token_request = pending_cli.build_service_account_token_delete_request(
            "svc/acct",
            "token/1",
        )

        self.assertEqual(team_request["path"], "/api/teams/folder%2Fops")
        self.assertEqual(
            service_account_request["path"],
            "/api/serviceaccounts/svc%2Facct",
        )
        self.assertEqual(
            token_request["path"],
            "/api/serviceaccounts/svc%2Facct/tokens/token%2F1",
        )
