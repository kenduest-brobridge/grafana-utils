import argparse
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
import importlib

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

common = importlib.import_module("grafana_utils.dashboards.common")
dashboard_topology = importlib.import_module("grafana_utils.dashboard_topology")


class DashboardTopologyTests(unittest.TestCase):
    def test_build_topology_document_rejects_missing_datasource_edges(self):
        with self.assertRaisesRegex(
            common.GrafanaError, "dashboardDatasourceEdges array"
        ):
            dashboard_topology.build_topology_document(
                {"dashboardGovernance": []},
                alert_contract_document=None,
            )

    def test_build_topology_document_rejects_missing_governance_array(self):
        with self.assertRaisesRegex(
            common.GrafanaError, "dashboardGovernance array"
        ):
            dashboard_topology.build_topology_document(
                {"dashboardDatasourceEdges": []},
                alert_contract_document=None,
            )

    def test_build_topology_document_and_renderers(self):
        governance_document = {
            "dashboardDatasourceEdges": [
                {
                    "datasourceUid": "prom-main",
                    "datasource": "Prometheus Main",
                    "dashboardUid": "dash-1",
                    "queryVariables": ["region"],
                }
            ],
            "dashboardGovernance": [
                {"dashboardUid": "dash-1", "dashboardTitle": "CPU Overview"}
            ],
            "dashboardDependencies": [
                {
                    "dashboardUid": "dash-1",
                    "panelIds": ["7"],
                    "panelVariables": ["region"],
                    "queryVariables": ["service"],
                }
            ],
        }
        alert_contract_document = {
            "resources": [
                {
                    "kind": "grafana-alert-rule",
                    "identity": "cpu-high",
                    "title": "CPU High",
                    "references": ["dash-1", "prom-main", "cp-rules"],
                },
                {
                    "kind": "grafana-contact-point",
                    "identity": "cp-rules",
                    "title": "PagerDuty",
                    "references": [],
                },
            ]
        }

        document = dashboard_topology.build_topology_document(
            governance_document,
            alert_contract_document=alert_contract_document,
        )
        self.assertEqual(document["summary"]["datasourceCount"], 1)
        self.assertEqual(document["summary"]["dashboardCount"], 1)
        self.assertEqual(document["summary"]["panelCount"], 1)
        self.assertEqual(document["summary"]["variableCount"], 2)
        self.assertEqual(document["summary"]["alertRuleCount"], 1)
        self.assertEqual(document["summary"]["contactPointCount"], 1)

        text = dashboard_topology.render_topology_text(document)
        mermaid = dashboard_topology.render_topology_mermaid(document)
        json_text = dashboard_topology.render_topology_json(document)
        dot = dashboard_topology.render_topology_dot(document)

        self.assertIn("Dashboard topology:", text)
        self.assertIn("datasource:prom-main --feeds--> dashboard:dash-1", text)
        self.assertIn("graph TD", mermaid)
        self.assertIn('["Prometheus Main"]', mermaid)
        parsed = json.loads(json_text)
        self.assertIn("nodes", parsed)
        self.assertIn("edges", parsed)
        self.assertIn("summary", parsed)
        self.assertIn("digraph grafana_topology {", dot)
        self.assertIn("->", dot)

    def test_run_dashboard_topology_prints_and_writes_output(self):
        governance_document = {
            "dashboardDatasourceEdges": [
                {
                    "datasourceUid": "prom-main",
                    "datasource": "Prometheus Main",
                    "dashboardUid": "dash-1",
                }
            ],
            "dashboardGovernance": [
                {"dashboardUid": "dash-1", "dashboardTitle": "CPU Overview"}
            ],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            governance_path = Path(tmpdir) / "governance.json"
            output_path = Path(tmpdir) / "topology.txt"
            governance_path.write_text(
                json.dumps(governance_document),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                governance=str(governance_path),
                alert_contract=None,
                output_format="text",
                output_file=str(output_path),
                interactive=False,
            )

            stream = io.StringIO()
            with redirect_stdout(stream):
                result = dashboard_topology.run_dashboard_topology(args)

            self.assertEqual(result, 0)
            self.assertTrue(output_path.exists())
            rendered_stdout = stream.getvalue()
            rendered_file = output_path.read_text(encoding="utf-8")
            self.assertIn("Dashboard topology:", rendered_stdout)
            self.assertIn("Dashboard topology:", rendered_file)
            self.assertIn("datasources=1", rendered_file)


if __name__ == "__main__":
    unittest.main()
