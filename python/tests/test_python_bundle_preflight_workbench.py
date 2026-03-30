import ast
import importlib
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
MODULE_PATH = PYTHON_ROOT / "grafana_utils" / "bundle_preflight_workbench.py"
bundle_preflight_workbench = importlib.import_module(
    "grafana_utils.bundle_preflight_workbench"
)
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class BundlePreflightWorkbenchTests(unittest.TestCase):
    def test_bundle_preflight_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def build_source_bundle(self):
        return {
            "environment": "staging",
            "dashboards": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "folderPath": "Operations",
                    "datasourceUids": ["prom-main"],
                }
            ],
            "datasources": [
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "secureJsonDataPlaceholders": {
                        "basicAuthPassword": "${secret:prom-basic-auth}",
                    },
                    "secureJsonDataProviders": {
                        "httpHeaderValue1": "${provider:vault:secret/data/prom/token}",
                    },
                }
            ],
            "folders": [
                {
                    "kind": "folder",
                    "uid": "ops",
                    "title": "Operations",
                    "body": {"title": "Operations"},
                }
            ],
            "alerts": [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "A > 90",
                        "datasourceUid": "prom-main",
                        "datasourceName": "Prometheus Main",
                        "contactPoints": ["pagerduty-primary"],
                        "notificationSettings": {"receiver": "slack-primary"},
                    },
                }
            ],
        }

    def build_target_inventory(self):
        return {
            "environment": "prod",
            "dashboards": [
                {"uid": "cpu-main", "title": "CPU Main"},
            ],
            "datasources": [],
        }

    def test_bundle_preflight_build_bundle_preflight_document_aggregates_staged_checks(
        self,
    ):
        document = bundle_preflight_workbench.build_bundle_preflight_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            availability={
                "pluginIds": [],
                "datasourceUids": [],
                "datasourceNames": [],
                "contactPoints": [],
                "providerNames": [],
                "secretPlaceholderNames": [],
                "requiredPluginIds": ["grafana-piechart-panel"],
            },
        )
        self.assertEqual(
            document["kind"],
            bundle_preflight_workbench.BUNDLE_PREFLIGHT_KIND,
        )
        self.assertIn("promotionPlan", document)
        self.assertIn("promotionPreflight", document)
        self.assertIn("syncPreflight", document)
        self.assertIn("alertAssessment", document)
        self.assertIn("providerAssessment", document)
        self.assertIn("secretAssessment", document)
        self.assertEqual(document["summary"]["alertPlanOnlyCount"], 1)
        self.assertGreaterEqual(document["summary"]["syncBlockingCount"], 1)
        self.assertEqual(document["summary"]["providerBlockingCount"], 1)
        self.assertEqual(document["summary"]["secretBlockingCount"], 1)

    def test_bundle_preflight_render_bundle_preflight_text_renders_summary(self):
        document = bundle_preflight_workbench.build_bundle_preflight_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            availability={
                "pluginIds": [],
                "datasourceUids": [],
                "datasourceNames": [],
                "contactPoints": [],
                "providerNames": [],
                "secretPlaceholderNames": [],
            },
        )
        output = "\n".join(
            bundle_preflight_workbench.render_bundle_preflight_text(document)
        )
        self.assertIn("Bundle preflight summary", output)
        self.assertIn("Promotion blocking:", output)
        self.assertIn("Sync blocking:", output)
        self.assertIn("Alert plan-only: 1", output)
        self.assertIn("Provider blocking: 1", output)
        self.assertIn("Secret placeholders blocking: 1", output)
        self.assertEqual(
            document["secretAssessment"]["plans"][0]["providerKind"],
            "inline-placeholder-map",
        )
        self.assertEqual(
            document["secretAssessment"]["plans"][0]["placeholderNames"],
            ["prom-basic-auth"],
        )
        self.assertEqual(
            document["secretAssessment"]["checks"][0]["placeholderName"],
            "prom-basic-auth",
        )

    def test_bundle_preflight_render_bundle_preflight_rejects_wrong_kind(self):
        with self.assertRaises(GrafanaError):
            bundle_preflight_workbench.render_bundle_preflight_text({"kind": "wrong"})


if __name__ == "__main__":
    unittest.main()
