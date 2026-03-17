import ast
import importlib
import json
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
MODULE_PATH = PYTHON_ROOT / "grafana_utils" / "dashboards" / "variable_inspection.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

variable_inspection = importlib.import_module(
    "grafana_utils.dashboards.variable_inspection"
)
GrafanaError = importlib.import_module("grafana_utils.dashboards.common").GrafanaError


class FakeDashboardClient(object):
    def __init__(self, payloads=None):
        self.payloads = dict(payloads or {})
        self.calls = []

    def fetch_dashboard(self, uid):
        self.calls.append(uid)
        return self.payloads[uid]


class DashboardVariableInspectionTests(unittest.TestCase):
    def sample_dashboard(self):
        return {
            "uid": "cpu-main",
            "title": "CPU Main",
            "templating": {
                "list": [
                    {
                        "name": "env",
                        "type": "query",
                        "label": "Environment",
                        "current": {"text": "prod", "value": "prod"},
                        "query": "label_values(up, env)",
                        "multi": True,
                        "includeAll": False,
                        "options": [
                            {"text": "prod", "value": "prod"},
                            {"text": "stage", "value": "stage"},
                            {"text": "dev", "value": "dev"},
                            {"text": "qa", "value": "qa"},
                        ],
                    },
                    {
                        "name": "datasource",
                        "type": "datasource",
                        "label": "Datasource",
                        "current": {"text": "Prometheus Main", "value": "prom-main"},
                        "datasource": {"uid": "grafana"},
                        "query": "prometheus",
                        "multi": False,
                        "includeAll": False,
                        "options": [
                            {"text": "Prometheus Main", "value": "prom-main"},
                            {"text": "Prometheus DR", "value": "prom-dr"},
                        ],
                    },
                    {"type": "query", "label": "ignored-empty-name"},
                ]
            },
        }

    def test_dashboard_variable_inspection_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_variable_inspection_resolve_dashboard_uid_prefers_explicit_uid(
        self,
    ):
        self.assertEqual(
            variable_inspection.resolve_dashboard_uid(
                dashboard_uid="cpu-main",
                dashboard_url="https://grafana.example.com/d/ignored/slug",
            ),
            "cpu-main",
        )

    def test_dashboard_variable_inspection_resolve_dashboard_uid_supports_dashboard_url(
        self,
    ):
        self.assertEqual(
            variable_inspection.resolve_dashboard_uid(
                dashboard_url="https://grafana.example.com/d/cpu-main/cpu-overview?orgId=1"
            ),
            "cpu-main",
        )
        self.assertEqual(
            variable_inspection.resolve_dashboard_uid(
                dashboard_url="https://grafana.example.com/d-solo/cpu-main/cpu-overview?panelId=7"
            ),
            "cpu-main",
        )

    def test_dashboard_variable_inspection_resolve_dashboard_uid_rejects_missing_or_invalid_input(
        self,
    ):
        with self.assertRaises(GrafanaError):
            variable_inspection.resolve_dashboard_uid()
        with self.assertRaises(GrafanaError):
            variable_inspection.resolve_dashboard_uid(
                dashboard_url="https://grafana.example.com/explore"
            )

    def test_dashboard_variable_inspection_extract_dashboard_variables_normalizes_templating_rows(
        self,
    ):
        rows = variable_inspection.extract_dashboard_variables(self.sample_dashboard())
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["name"], "env")
        self.assertEqual(rows[0]["current"], "prod")
        self.assertEqual(rows[0]["query"], "label_values(up, env)")
        self.assertEqual(rows[0]["optionCount"], 4)
        self.assertEqual(rows[1]["datasource"], "grafana")
        self.assertEqual(rows[1]["options"], ["Prometheus Main", "Prometheus DR"])

    def test_dashboard_variable_inspection_apply_vars_query_overrides_updates_matching_current_values(
        self,
    ):
        rows = variable_inspection.extract_dashboard_variables(self.sample_dashboard())
        variable_inspection.apply_vars_query_overrides(
            rows,
            "orgId=1&var-env=stage&var-datasource=prom-dr",
        )
        self.assertEqual(rows[0]["current"], "stage")
        self.assertEqual(rows[1]["current"], "prom-dr")

    def test_dashboard_variable_inspection_parse_vars_query_ignores_non_var_keys(self):
        self.assertEqual(
            variable_inspection.parse_vars_query(
                "?panelId=7&var-env=prod&from=now-6h&var-host=web01"
            ),
            {"env": "prod", "host": "web01"},
        )

    def test_dashboard_variable_inspection_build_document_and_render_json(self):
        document = variable_inspection.build_dashboard_variable_document(
            self.sample_dashboard(),
            dashboard_uid="cpu-main",
        )
        rendered = variable_inspection.render_dashboard_variable_document(
            document,
            output_format="json",
        )
        parsed = json.loads(rendered)
        self.assertEqual(parsed["dashboardUid"], "cpu-main")
        self.assertEqual(parsed["dashboardTitle"], "CPU Main")
        self.assertEqual(parsed["variableCount"], 2)

    def test_dashboard_variable_inspection_render_table_and_csv_outputs(self):
        document = variable_inspection.build_dashboard_variable_document(
            self.sample_dashboard()
        )
        table_output = variable_inspection.render_dashboard_variable_document(
            document,
            output_format="table",
            include_header=True,
        )
        self.assertIn("NAME", table_output)
        self.assertIn("env", table_output)
        self.assertIn("prod,stage,dev (+1 more)", table_output)

        csv_output = variable_inspection.render_dashboard_variable_document(
            document,
            output_format="csv",
            include_header=False,
        )
        self.assertNotIn("name,type,label", csv_output)
        self.assertIn("env,query,Environment,prod", csv_output)

    def test_dashboard_variable_inspection_render_rejects_unknown_output_format(self):
        document = variable_inspection.build_dashboard_variable_document(
            self.sample_dashboard()
        )
        with self.assertRaises(GrafanaError):
            variable_inspection.render_dashboard_variable_document(
                document,
                output_format="yaml",
            )

    def test_dashboard_variable_inspection_inspect_dashboard_variables_with_client_fetches_and_overlays_query(
        self,
    ):
        client = FakeDashboardClient(
            payloads={"cpu-main": {"dashboard": self.sample_dashboard()}}
        )
        document = variable_inspection.inspect_dashboard_variables_with_client(
            client,
            dashboard_url="https://grafana.example.com/d/cpu-main/cpu-overview",
            vars_query="var-env=stage",
        )
        self.assertEqual(client.calls, ["cpu-main"])
        self.assertEqual(document["dashboardUid"], "cpu-main")
        self.assertEqual(document["variables"][0]["current"], "stage")


if __name__ == "__main__":
    unittest.main()
