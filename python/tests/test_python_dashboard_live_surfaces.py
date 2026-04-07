import importlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

dashboard_authoring = importlib.import_module("grafana_utils.dashboard_authoring")
dashboard_cli = importlib.import_module("grafana_utils.dashboard_cli")


class FakeDashboardClient:
    def __init__(self, payload):
        self.payload = payload
        self.imported_payloads = []

    def fetch_dashboard(self, uid):
        self.fetched_uid = uid
        return self.payload

    def import_dashboard(self, payload):
        self.imported_payloads.append(payload)
        return {"status": "success", "uid": payload["dashboard"].get("uid")}


class DashboardLiveSurfaceTests(unittest.TestCase):
    def test_dashboard_parse_args_supports_edit_live(self):
        args = dashboard_cli.parse_args(
            [
                "edit-live",
                "--url",
                "http://localhost:3000",
                "--basic-user",
                "admin",
                "--basic-password",
                "admin",
                "--dashboard-uid",
                "cpu-main",
            ]
        )

        self.assertEqual(args.command, "edit-live")
        self.assertEqual(args.dashboard_uid, "cpu-main")
        self.assertEqual(args.message, "Imported by grafana-utils")

    def test_dashboard_parse_args_supports_serve(self):
        args = dashboard_cli.parse_args(
            [
                "serve",
                "--input",
                "./dashboards/raw",
                "--open-browser",
            ]
        )

        self.assertEqual(args.command, "serve")
        self.assertEqual(args.input, "./dashboards/raw")
        self.assertTrue(args.open_browser)

    def test_build_live_dashboard_authoring_document_preserves_wrapper_metadata(self):
        document = dashboard_authoring.build_live_dashboard_authoring_document(
            {
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["ops"],
                },
                "meta": {"folderUid": "infra", "message": "live"},
            }
        )

        self.assertEqual(document["dashboard"]["id"], None)
        self.assertEqual(document["dashboard"]["uid"], "cpu-main")
        self.assertEqual(document["dashboard"]["title"], "CPU Main")
        self.assertEqual(document["meta"]["folderUid"], "infra")
        self.assertEqual(document["meta"]["message"], "live")

    def test_build_dashboard_authoring_review_detects_uid_drift_and_id_state(self):
        review = dashboard_authoring.build_dashboard_authoring_review(
            {
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main-clone",
                    "title": "CPU Main",
                    "tags": ["ops"],
                },
                "meta": {"folderUid": "infra"},
            },
            input_file="edited draft for cpu-main",
            source_uid="cpu-main",
        )

        self.assertEqual(review["documentKind"], "wrapped")
        self.assertFalse(review["dashboardIdIsNull"])
        self.assertIn("dashboard.id must stay null", " ".join(review["blockingIssues"]))
        self.assertIn("edited draft changed dashboard.uid", " ".join(review["blockingIssues"]))
        self.assertEqual(review["suggestedNextAction"], "fix blocking issues, then publish --dry-run")

    def test_load_dashboard_serve_items_from_directory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            dashboard_authoring.write_json_document(
                {
                    "dashboard": {
                        "id": None,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                    },
                    "meta": {"folderUid": "infra"},
                },
                root / "cpu-main.json",
            )
            dashboard_authoring.write_json_document(
                {
                    "id": None,
                    "uid": "cpu-secondary",
                    "title": "CPU Secondary",
                },
                root / "subdir" / "cpu-secondary.yaml",
            )
            args = SimpleNamespace(input=str(root), script=None, script_format="json", watch=None)

            items = dashboard_authoring.load_dashboard_serve_items(args)

        self.assertEqual(len(items), 2)
        self.assertEqual(items[0]["uid"], "cpu-main")
        self.assertEqual(items[0]["documentKind"], "wrapped")
        self.assertEqual(items[1]["uid"], "cpu-secondary")
        self.assertEqual(items[1]["documentKind"], "bare")

    def test_load_dashboard_serve_items_from_script_json(self):
        args = SimpleNamespace(
            input=None,
            script="printf '[{\"dashboard\":{\"id\":null,\"uid\":\"cpu-main\",\"title\":\"CPU Main\"}}]'",
            script_format="json",
            watch=None,
        )
        completed = SimpleNamespace(returncode=0, stdout='[{"dashboard":{"id":null,"uid":"cpu-main","title":"CPU Main"}}]')

        with mock.patch.object(dashboard_authoring.subprocess, "run", return_value=completed):
            items = dashboard_authoring.load_dashboard_serve_items(args)

        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["uid"], "cpu-main")
        self.assertEqual(items[0]["source"], "script:0")

    def test_run_dashboard_edit_live_applies_edited_dashboard(self):
        fake_client = FakeDashboardClient(
            {
                "dashboard": {
                    "id": 12,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["ops"],
                },
                "meta": {"folderUid": "infra"},
            }
        )
        edited = {
            "dashboard": {
                "id": None,
                "uid": "cpu-main",
                "title": "CPU Main Updated",
                "tags": ["ops", "sre"],
            },
            "meta": {"folderUid": "infra", "message": "Edited"},
        }
        args = SimpleNamespace(
            dashboard_uid="cpu-main",
            output=None,
            apply_live=True,
            message="Keep current",
            yes=True,
        )

        with mock.patch.object(
            dashboard_authoring, "edit_payload_in_external_editor", return_value=edited
        ):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = dashboard_authoring.run_dashboard_edit_live(fake_client, args)

        self.assertEqual(result, 0)
        self.assertEqual(len(fake_client.imported_payloads), 1)
        payload = fake_client.imported_payloads[0]
        self.assertEqual(payload["dashboard"]["uid"], "cpu-main")
        self.assertEqual(payload["dashboard"]["id"], None)
        self.assertEqual(payload["folderUid"], "infra")
        self.assertEqual(payload["message"], "Keep current")
        self.assertIn("Applied edited dashboard cpu-main back to Grafana.", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
