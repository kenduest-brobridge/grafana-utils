import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource_secret_workbench.py"
secret_workbench = importlib.import_module("grafana_utils.datasource_secret_workbench")
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class DatasourceSecretWorkbenchTests(unittest.TestCase):
    def test_datasource_secret_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_datasource_secret_build_datasource_secret_plan_fails_for_missing_placeholder_value(self):
        with self.assertRaisesRegex(
            GrafanaError,
            "Missing datasource secret placeholder 'metrics-token'",
        ):
            secret_workbench.build_datasource_secret_plan(
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "secureJsonDataPlaceholders": {
                        "httpHeaderValue1": "${secret:metrics-token}",
                    },
                },
                {},
            )

    def test_datasource_secret_collect_secret_placeholders_rejects_opaque_secret_replay(self):
        with self.assertRaisesRegex(
            GrafanaError,
            "opaque replay is not allowed",
        ):
            secret_workbench.collect_secret_placeholders(
                {"basicAuthPassword": "already-a-secret"}
            )

    def test_datasource_secret_build_datasource_secret_plan_shapes_resolved_plan_for_review(self):
        plan = secret_workbench.build_datasource_secret_plan(
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "secureJsonDataPlaceholders": {
                    "basicAuthPassword": "${secret:loki-basic-pass}",
                    "httpHeaderValue1": "${secret:loki-tenant-token}",
                },
            },
            {
                "loki-basic-pass": "pass-123",
                "loki-tenant-token": "Bearer tenant-a",
            },
        )

        self.assertEqual(plan.datasource_uid, "loki-main")
        self.assertEqual(plan.action, "inject-secrets")
        self.assertTrue(plan.review_required)
        self.assertEqual(plan.provider_kind, "inline-placeholder-map")
        self.assertEqual(
            plan.resolved_secure_json_data,
            {
                "basicAuthPassword": "pass-123",
                "httpHeaderValue1": "Bearer tenant-a",
            },
        )
        self.assertEqual(
            secret_workbench.summarize_secret_plan(plan),
            {
                "datasourceUid": "loki-main",
                "datasourceName": "Loki Main",
                "datasourceType": "loki",
                "action": "inject-secrets",
                "reviewRequired": True,
                "providerKind": "inline-placeholder-map",
                "secretFields": ["basicAuthPassword", "httpHeaderValue1"],
                "placeholderNames": ["loki-basic-pass", "loki-tenant-token"],
            },
        )
