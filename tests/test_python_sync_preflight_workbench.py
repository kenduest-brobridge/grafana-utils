import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "sync_preflight_workbench.py"
sync_preflight_workbench = importlib.import_module(
    "grafana_utils.sync_preflight_workbench"
)
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class SyncPreflightWorkbenchTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_build_sync_preflight_document_reports_missing_plugin_and_alert_blocks(self):
        document = sync_preflight_workbench.build_sync_preflight_document(
            [
                {
                    "kind": "datasource",
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "body": {"type": "prometheus"},
                },
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "managedFields": ["condition"],
                    "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]},
                },
            ],
            availability={
                "pluginIds": [],
                "datasourceUids": [],
                "contactPoints": [],
            },
        )
        self.assertEqual(
            document["kind"],
            sync_preflight_workbench.SYNC_PREFLIGHT_KIND,
        )
        self.assertEqual(document["summary"]["checkCount"], 4)
        self.assertEqual(document["summary"]["blockingCount"], 3)
        checks = {(item["kind"], item["identity"]): item for item in document["checks"]}
        self.assertEqual(checks[("datasource", "prom-main")]["status"], "create-planned")
        self.assertEqual(checks[("plugin", "prometheus")]["status"], "missing")
        self.assertEqual(checks[("alert-live-apply", "cpu-high")]["status"], "blocked")
        self.assertEqual(
            checks[("alert-contact-point", "cpu-high->pagerduty-primary")]["status"],
            "missing",
        )

    def test_render_sync_preflight_text_renders_summary_and_rows(self):
        document = sync_preflight_workbench.build_sync_preflight_document(
            [
                {
                    "kind": "dashboard",
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "body": {"datasourceUids": ["prom-main"]},
                }
            ],
            availability={"datasourceUids": ["prom-main"]},
        )
        output = "\n".join(sync_preflight_workbench.render_sync_preflight_text(document))
        self.assertIn("Sync preflight summary", output)
        self.assertIn("Checks: 1 total, 1 ok, 0 blocking", output)
        self.assertIn(
            "- dashboard-datasource identity=cpu-main->prom-main status=ok",
            output,
        )

    def test_render_sync_preflight_rejects_wrong_kind(self):
        with self.assertRaises(GrafanaError):
            sync_preflight_workbench.render_sync_preflight_text({"kind": "wrong"})


if __name__ == "__main__":
    unittest.main()
