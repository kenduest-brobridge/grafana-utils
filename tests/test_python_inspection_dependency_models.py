import unittest

from grafana_utils.dashboards.inspection_dependency_models import (
    build_dependency_rows_from_query_report,
)


class InspectionDependencyModelTests(unittest.TestCase):
    def test_build_dependency_rows_from_query_report_enriches_family_specific_records(self):
        query_rows = [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelId": "7",
                "panelTitle": "CPU",
                "panelType": "timeseries",
                "refId": "A",
                "datasource": "prom-main",
                "datasourceUid": "prom-main",
                "datasourceType": "prometheus",
                "datasourceFamily": "",
                "queryField": "expr",
                "query": "sum(rate(node_cpu_seconds_total[5m])) by (job)",
                "file": "platform/cpu.json",
            },
            {
                "dashboardUid": "logs-main",
                "dashboardTitle": "Logs Main",
                "folderPath": "Platform",
                "panelId": "2",
                "panelTitle": "Logs",
                "panelType": "logs",
                "refId": "A",
                "datasource": "loki-main",
                "datasourceUid": "loki-main",
                "datasourceType": "loki",
                "datasourceFamily": "",
                "queryField": "query",
                "query": '{job="grafana"} |~ "error" | count_over_time',
                "file": "platform/logs.json",
            },
        ]
        inventory = [
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "orgId": "1",
            },
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "orgId": "1",
            },
            {"uid": "unused-main", "name": "Unused Main", "type": "tempo"},
        ]
        report = build_dependency_rows_from_query_report(query_rows, inventory)
        summary = report.summary
        self.assertEqual(summary["queryCount"], 2)
        self.assertEqual(summary["datasourceCount"], 2)
        self.assertEqual(summary["orphanedCount"], 1)

        records = [row for row in report.queries]
        self.assertEqual(records[0].datasource_family, "prometheus")
        self.assertGreaterEqual(len(records[0].features.metrics), 1)
        self.assertEqual(records[1].datasource_family, "loki")
        self.assertGreaterEqual(len(records[1].features.labels), 1)

    def test_usage_summary_stable_orphan_detection(self):
        report = build_dependency_rows_from_query_report(
            [],
            [
                {"uid": "unused-main", "name": "Unused Main", "type": "tempo"},
            ],
        )
        self.assertEqual(report.summary["queryCount"], 0)
        self.assertEqual(report.summary["datasourceCount"], 0)
        self.assertEqual(report.summary["orphanedCount"], 1)
        self.assertEqual(report.orphaned[0].uid, "unused-main")


if __name__ == "__main__":
    unittest.main()
