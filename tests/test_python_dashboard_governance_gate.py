import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

from grafana_utils import dashboard_governance_gate


class DashboardGovernanceGateTests(unittest.TestCase):
    def build_policy(self, **overrides):
        policy = {
            "version": 1,
            "datasources": {
                "allowedFamilies": ["prometheus", "loki"],
                "allowedUids": ["prom-main", "logs-main"],
                "forbidUnknown": True,
                "forbidMixedFamilies": True,
            },
            "queries": {
                "maxQueriesPerDashboard": 3,
                "maxQueriesPerPanel": 2,
                "forbidSelectStar": True,
                "requireSqlTimeFilter": True,
                "forbidBroadLokiRegex": True,
            },
            "enforcement": {"failOnWarnings": False},
        }
        for key, value in overrides.items():
            policy[key] = value
        return policy

    def build_governance_document(self):
        return {
            "summary": {
                "dashboardCount": 2,
                "queryRecordCount": 4,
                "datasourceFamilyCount": 3,
                "datasourceCoverageCount": 4,
                "riskRecordCount": 2,
            },
            "datasourceFamilies": [
                {"family": "prometheus"},
                {"family": "loki"},
                {"family": "postgres"},
            ],
            "datasources": [
                {"datasourceUid": "prom-main", "datasource": "prom-main", "family": "prometheus"},
                {"datasourceUid": "logs-main", "datasource": "logs-main", "family": "loki"},
                {"datasourceUid": "pg-main", "datasource": "pg-main", "family": "postgres"},
            ],
            "riskRecords": [
                {
                    "kind": "mixed-datasource-dashboard",
                    "dashboardUid": "mixed-main",
                    "panelId": "",
                    "datasource": "prom-main,logs-main",
                    "recommendation": "Split panel queries by datasource.",
                },
                {
                    "kind": "orphaned-datasource",
                    "dashboardUid": "",
                    "panelId": "",
                    "datasource": "unused-main",
                    "recommendation": "Remove the unused datasource.",
                },
            ],
        }

    def build_query_document(self):
        return {
            "summary": {"dashboardCount": 2, "queryRecordCount": 4},
            "queries": [
                {
                    "dashboardUid": "cpu-main",
                    "dashboardTitle": "CPU Main",
                    "panelId": "7",
                    "panelTitle": "CPU Usage",
                    "refId": "A",
                    "datasource": "prom-main",
                    "datasourceUid": "prom-main",
                    "datasourceType": "prometheus",
                    "datasourceFamily": "prometheus",
                    "query": "sum(rate(node_cpu_seconds_total[5m]))",
                },
                {
                    "dashboardUid": "logs-main",
                    "dashboardTitle": "Logs Main",
                    "panelId": "8",
                    "panelTitle": "Logs",
                    "refId": "A",
                    "datasource": "logs-main",
                    "datasourceUid": "logs-main",
                    "datasourceType": "loki",
                    "datasourceFamily": "loki",
                    "query": "{job=~\".*\"}",
                },
                {
                    "dashboardUid": "sql-main",
                    "dashboardTitle": "SQL Main",
                    "panelId": "9",
                    "panelTitle": "Latency",
                    "refId": "A",
                    "datasource": "pg-main",
                    "datasourceUid": "pg-main",
                    "datasourceType": "grafana-postgresql-datasource",
                    "datasourceFamily": "postgres",
                    "query": "select * from metrics",
                },
                {
                    "dashboardUid": "sql-main",
                    "dashboardTitle": "SQL Main",
                    "panelId": "9",
                    "panelTitle": "Latency",
                    "refId": "B",
                    "datasource": "unknown",
                    "datasourceUid": "",
                    "datasourceType": "",
                    "datasourceFamily": "unknown",
                    "query": "select value from metrics",
                },
            ],
        }

    def test_evaluate_policy_reports_blocking_violations(self):
        result = dashboard_governance_gate.evaluate_dashboard_governance_policy(
            self.build_policy(),
            self.build_governance_document(),
            self.build_query_document(),
        )

        self.assertFalse(result["ok"])
        codes = [item["code"] for item in result["violations"]]
        self.assertIn("MIXED_DATASOURCE_DASHBOARD", codes)
        self.assertIn("LOKI_BROAD_REGEX", codes)
        self.assertIn("DATASOURCE_FAMILY_NOT_ALLOWED", codes)
        self.assertIn("DATASOURCE_UID_NOT_ALLOWED", codes)
        self.assertIn("SQL_SELECT_STAR", codes)
        self.assertIn("SQL_MISSING_TIME_FILTER", codes)
        self.assertIn("DATASOURCE_UNKNOWN", codes)
        self.assertEqual(result["summary"]["warningCount"], 2)

    def test_evaluate_policy_honors_query_count_thresholds(self):
        policy = self.build_policy(
            datasources={},
            queries={"maxQueriesPerDashboard": 1, "maxQueriesPerPanel": 1},
        )
        result = dashboard_governance_gate.evaluate_dashboard_governance_policy(
            policy,
            {"summary": {}, "riskRecords": []},
            self.build_query_document(),
        )

        codes = [item["code"] for item in result["violations"]]
        self.assertIn("QUERY_COUNT_TOO_HIGH", codes)
        self.assertIn("PANEL_QUERY_COUNT_TOO_HIGH", codes)

    def test_render_dashboard_governance_check_formats_text_output(self):
        text = dashboard_governance_gate.render_dashboard_governance_check(
            {
                "ok": False,
                "summary": {
                    "dashboardCount": 2,
                    "queryRecordCount": 4,
                    "violationCount": 1,
                    "warningCount": 1,
                },
                "violations": [
                    {
                        "code": "DATASOURCE_UNKNOWN",
                        "dashboardUid": "cpu-main",
                        "panelId": "7",
                        "refId": "A",
                        "datasourceUid": "",
                        "datasource": "unknown",
                        "message": "Datasource identity could not be resolved for this query row.",
                    }
                ],
                "warnings": [
                    {
                        "riskKind": "orphaned-datasource",
                        "dashboardUid": "",
                        "panelId": "",
                        "datasource": "unused-main",
                        "message": "Remove the unused datasource.",
                    }
                ],
            }
        )

        self.assertIn("Dashboard governance check: FAIL", text)
        self.assertIn("ERROR [DATASOURCE_UNKNOWN]", text)
        self.assertIn("WARN [orphaned-datasource]", text)

    def test_evaluate_policy_can_fail_on_governance_warnings(self):
        policy = self.build_policy(
            datasources={},
            queries={},
            enforcement={"failOnWarnings": True},
        )

        result = dashboard_governance_gate.evaluate_dashboard_governance_policy(
            policy,
            self.build_governance_document(),
            {
                "summary": {"dashboardCount": 0, "queryRecordCount": 0},
                "queries": [],
            },
        )

        self.assertFalse(result["ok"])
        self.assertEqual(result["summary"]["violationCount"], 0)
        self.assertEqual(result["summary"]["warningCount"], 2)

    def test_main_writes_json_and_returns_failure_for_violations(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            policy_path = root / "policy.json"
            governance_path = root / "governance.json"
            queries_path = root / "queries.json"
            output_path = root / "result.json"
            policy_path.write_text(
                json.dumps(self.build_policy(), ensure_ascii=False),
                encoding="utf-8",
            )
            governance_path.write_text(
                json.dumps(self.build_governance_document(), ensure_ascii=False),
                encoding="utf-8",
            )
            queries_path.write_text(
                json.dumps(self.build_query_document(), ensure_ascii=False),
                encoding="utf-8",
            )

            buffer = io.StringIO()
            with redirect_stdout(buffer):
                code = dashboard_governance_gate.main(
                    [
                        "--policy",
                        str(policy_path),
                        "--governance",
                        str(governance_path),
                        "--queries",
                        str(queries_path),
                        "--output-format",
                        "json",
                        "--json-output",
                        str(output_path),
                    ]
                )

            self.assertEqual(code, 1)
            payload = json.loads(buffer.getvalue())
            self.assertFalse(payload["ok"])
            self.assertTrue(output_path.is_file())
            saved = json.loads(output_path.read_text(encoding="utf-8"))
            self.assertEqual(saved["summary"]["violationCount"], payload["summary"]["violationCount"])


if __name__ == "__main__":
    unittest.main()
