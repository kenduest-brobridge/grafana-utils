import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
GOVERNANCE_MODULE = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_governance.py"
)
GOVERNANCE_RENDER_MODULE = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_governance_render.py"
)

inspection_governance = importlib.import_module(
    "grafana_utils.dashboards.inspection_governance"
)
inspection_governance_render = importlib.import_module(
    "grafana_utils.dashboards.inspection_governance_render"
)


class DashboardInspectionGovernanceTests(unittest.TestCase):
    def _assert_parses_as_python39(self, path):
        source = path.read_text(encoding="utf-8")
        ast.parse(source, filename=str(path), feature_version=(3, 9))

    def _build_fixture_documents(self):
        summary_document = {
            "summary": {
                "dashboardCount": 2,
                "datasourceInventoryCount": 3,
                "mixedDatasourceDashboardCount": 1,
                "orphanedDatasourceCount": 1,
            },
            "mixedDatasourceDashboards": [
                {
                    "uid": "mixed-main",
                    "title": "Mixed Main",
                    "folderPath": "Platform / Infra",
                    "datasources": ["prom-main", "loki-main"],
                }
            ],
            "orphanedDatasources": [
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                    "type": "tempo",
                }
            ],
            "datasourceInventory": [
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "referenceCount": 1,
                },
                {
                    "uid": "loki-main",
                    "name": "Loki Main",
                    "type": "loki",
                    "referenceCount": 1,
                },
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                    "type": "tempo",
                    "referenceCount": 0,
                },
            ],
        }
        report_document = {
            "summary": {"queryRecordCount": 3},
            "queries": [
                {
                    "dashboardUid": "cpu-main",
                    "panelId": "7",
                    "datasource": "prom-main",
                    "datasourceUid": "prom-main",
                    "queryField": "expr",
                    "metrics": ["node_cpu_seconds_total"],
                    "measurements": [],
                    "buckets": [],
                },
                {
                    "dashboardUid": "logs-main",
                    "panelId": "8",
                    "datasource": "loki-main",
                    "datasourceUid": "loki-main",
                    "queryField": "expr",
                    "metrics": ["count_over_time"],
                    "measurements": ['job="grafana"'],
                    "buckets": ["5m"],
                },
                {
                    "dashboardUid": "custom-main",
                    "panelId": "9",
                    "datasource": "custom-main",
                    "datasourceUid": "",
                    "queryField": "query",
                    "metrics": [],
                    "measurements": [],
                    "buckets": [],
                },
            ],
        }
        return summary_document, report_document

    def test_governance_modules_parse_as_python39_syntax(self):
        self._assert_parses_as_python39(GOVERNANCE_MODULE)
        self._assert_parses_as_python39(GOVERNANCE_RENDER_MODULE)

    def test_build_export_inspection_governance_document_summarizes_families_and_risks(self):
        summary_document, report_document = self._build_fixture_documents()

        document = inspection_governance.build_export_inspection_governance_document(
            summary_document, report_document
        )

        self.assertEqual(document["summary"]["dashboardCount"], 2)
        self.assertEqual(document["summary"]["queryRecordCount"], 3)
        self.assertEqual(document["summary"]["datasourceFamilyCount"], 3)
        self.assertEqual(document["summary"]["riskRecordCount"], 4)
        self.assertEqual(document["datasourceFamilies"][0]["family"], "loki")
        self.assertEqual(document["datasourceFamilies"][1]["family"], "prometheus")
        self.assertEqual(document["datasourceFamilies"][2]["family"], "unknown")
        datasource_rows = dict(
            (row["datasourceUid"], row) for row in document["datasources"]
        )
        self.assertEqual(datasource_rows["unused-main"]["orphaned"], True)
        self.assertEqual(datasource_rows["prom-main"]["queryCount"], 1)
        self.assertEqual(datasource_rows["loki-main"]["family"], "loki")
        risk_kinds = [row["kind"] for row in document["riskRecords"]]
        self.assertIn("mixed-datasource-dashboard", risk_kinds)
        self.assertIn("orphaned-datasource", risk_kinds)
        self.assertIn("unknown-datasource-family", risk_kinds)
        self.assertIn("empty-query-analysis", risk_kinds)
        orphaned = [
            row for row in document["riskRecords"] if row["kind"] == "orphaned-datasource"
        ][0]
        self.assertEqual(orphaned["category"], "inventory")
        self.assertIn("Remove the unused datasource", orphaned["recommendation"])
        unknown = [
            row
            for row in document["riskRecords"]
            if row["kind"] == "unknown-datasource-family"
        ][0]
        self.assertEqual(unknown["category"], "coverage")
        self.assertIn("Normalize the datasource type mapping", unknown["recommendation"])

    def test_render_export_inspection_governance_tables_renders_sections(self):
        summary_document, report_document = self._build_fixture_documents()
        document = inspection_governance.build_export_inspection_governance_document(
            summary_document, report_document
        )

        lines = inspection_governance_render.render_export_inspection_governance_tables(
            document, "dashboards/raw"
        )
        output = "\n".join(lines)

        self.assertIn("Export inspection governance: dashboards/raw", output)
        self.assertIn("# Summary", output)
        self.assertIn("# Datasource Families", output)
        self.assertIn("# Datasources", output)
        self.assertIn("# Risks", output)
        self.assertIn("prometheus", output)
        self.assertIn("loki", output)
        self.assertIn("mixed-datasource-dashboard", output)
        self.assertIn("unused-main", output)
        self.assertIn("CATEGORY", output)
        self.assertIn("RECOMMENDATION", output)
        self.assertIn("Remove the unused datasource", output)
        self.assertIn("CATEGORY", output)
        self.assertIn("RECOMMENDATION", output)
        self.assertIn("Split panel queries by datasource", output)


if __name__ == "__main__":
    unittest.main()
