import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "alert_sync_workbench.py"
alert_sync_workbench = importlib.import_module("grafana_utils.alert_sync_workbench")
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class AlertSyncWorkbenchTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_assess_alert_sync_specs_reports_candidate_plan_only_and_blocked(self):
        document = alert_sync_workbench.assess_alert_sync_specs(
            [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "managedFields": ["condition"],
                    "body": {"condition": "A > 90"},
                },
                {
                    "kind": "alert",
                    "uid": "logs-high",
                    "title": "Logs High",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "B > 10",
                        "contactPoints": ["pagerduty-primary"],
                    },
                },
                {
                    "kind": "alert",
                    "uid": "bad-alert",
                    "title": "Bad Alert",
                    "managedFields": ["labels"],
                    "body": {"labels": {"severity": "warning"}},
                },
            ]
        )
        self.assertEqual(document["kind"], alert_sync_workbench.ALERT_SYNC_KIND)
        self.assertEqual(document["summary"]["candidateCount"], 1)
        self.assertEqual(document["summary"]["planOnlyCount"], 1)
        self.assertEqual(document["summary"]["blockedCount"], 1)
        rows = {item["identity"]: item for item in document["alerts"]}
        self.assertEqual(rows["cpu-high"]["status"], "candidate")
        self.assertTrue(rows["cpu-high"]["liveApplyAllowed"])
        self.assertEqual(rows["logs-high"]["status"], "plan-only")
        self.assertEqual(rows["bad-alert"]["status"], "blocked")

    def test_assess_alert_sync_specs_rejects_unknown_managed_field(self):
        with self.assertRaises(GrafanaError):
            alert_sync_workbench.assess_alert_sync_specs(
                [
                    {
                        "kind": "alert",
                        "uid": "cpu-high",
                        "managedFields": ["dashboardUid"],
                        "body": {"condition": "A > 90"},
                    }
                ]
            )

    def test_render_alert_sync_assessment_text_renders_summary(self):
        document = alert_sync_workbench.assess_alert_sync_specs(
            [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "managedFields": ["condition"],
                    "body": {"condition": "A > 90"},
                }
            ]
        )
        output = "\n".join(
            alert_sync_workbench.render_alert_sync_assessment_text(document)
        )
        self.assertIn("Alert sync assessment", output)
        self.assertIn("Alerts: 1 total, 1 candidate, 0 plan-only, 0 blocked", output)
        self.assertIn("- cpu-high status=candidate liveApplyAllowed=true", output)


if __name__ == "__main__":
    unittest.main()
