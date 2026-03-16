import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource_secret_provider_workbench.py"
provider_workbench = importlib.import_module(
    "grafana_utils.datasource_secret_provider_workbench"
)
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class DatasourceSecretProviderWorkbenchTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_collect_provider_references_rejects_opaque_secret_replay(self):
        with self.assertRaisesRegex(GrafanaError, "opaque replay is not allowed"):
            provider_workbench.collect_provider_references(
                {"basicAuthPassword": "already-a-secret"}
            )

    def test_build_provider_plan_shapes_review_summary(self):
        plan = provider_workbench.build_provider_plan(
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "secureJsonDataProviders": {
                    "basicAuthPassword": "${provider:vault:secret/data/loki/basic-auth}",
                    "httpHeaderValue1": "${provider:aws-sm:prod/loki/token}",
                },
            }
        )
        self.assertEqual(plan.provider_kind, "external-provider-reference")
        self.assertTrue(plan.review_required)
        self.assertEqual(
            provider_workbench.summarize_provider_plan(plan),
            {
                "datasourceUid": "loki-main",
                "datasourceName": "Loki Main",
                "datasourceType": "loki",
                "providerKind": "external-provider-reference",
                "action": "resolve-provider-secrets",
                "reviewRequired": True,
                "providers": [
                    {
                        "fieldName": "basicAuthPassword",
                        "providerName": "vault",
                        "secretPath": "secret/data/loki/basic-auth",
                    },
                    {
                        "fieldName": "httpHeaderValue1",
                        "providerName": "aws-sm",
                        "secretPath": "prod/loki/token",
                    },
                ],
            },
        )

    def test_iter_provider_names_deduplicates_names(self):
        plan = provider_workbench.build_provider_plan(
            {
                "name": "Prometheus Main",
                "type": "prometheus",
                "secureJsonDataProviders": {
                    "password": "${provider:vault:secret/a}",
                    "httpHeaderValue1": "${provider:vault:secret/b}",
                    "httpHeaderValue2": "${provider:aws-sm:secret/c}",
                },
            }
        )
        self.assertEqual(
            list(provider_workbench.iter_provider_names(plan.references)),
            ["vault", "aws-sm"],
        )


if __name__ == "__main__":
    unittest.main()
