import argparse
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

from tests.test_python_dashboard_cli import FakeDashboardWorkflowClient, build_export_metadata, exporter


class DashboardInspectionTests(unittest.TestCase):
    def write_summary_fixture(
        self,
        import_dir,
        dashboards,
        folders=None,
        datasources=None,
        index=None,
    ):
        exporter.write_json_document(
            build_export_metadata(
                variant=exporter.RAW_EXPORT_SUBDIR,
                dashboard_count=len(dashboards),
                format_name="grafana-web-import-preserve-uid",
                folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                datasources_file=exporter.DATASOURCE_INVENTORY_FILENAME,
            ),
            import_dir / exporter.EXPORT_METADATA_FILENAME,
        )
        exporter.write_json_document(
            list(index or []),
            import_dir / "index.json",
        )
        exporter.write_json_document(
            list(folders or []),
            import_dir / exporter.FOLDER_INVENTORY_FILENAME,
        )
        exporter.write_json_document(
            list(datasources or []),
            import_dir / exporter.DATASOURCE_INVENTORY_FILENAME,
        )
        for item in dashboards:
            exporter.write_json_document(
                {"dashboard": item["dashboard"], "meta": item.get("meta") or {}},
                import_dir / item["path"],
            )

    def write_report_fixture(self, import_dir, dashboard):
        exporter.write_json_document(
            build_export_metadata(
                variant=exporter.RAW_EXPORT_SUBDIR,
                dashboard_count=1,
                format_name="grafana-web-import-preserve-uid",
                folders_file=exporter.FOLDER_INVENTORY_FILENAME,
            ),
            import_dir / exporter.EXPORT_METADATA_FILENAME,
        )
        exporter.write_json_document([], import_dir / exporter.FOLDER_INVENTORY_FILENAME)
        exporter.write_json_document(
            {"dashboard": dashboard, "meta": {}},
            import_dir
            / "General"
            / (
                "%s__%s.json"
                % (
                str(dashboard.get("title") or "Dashboard").replace(" ", "_"),
                str(dashboard.get("uid") or "dashboard"),
                )
            ),
        )

    def run_inspect(self, args):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            result = exporter.inspect_export(args)
        return result, stdout.getvalue()

    def write_governance_fixture(self, import_dir):
        self.write_summary_fixture(
            import_dir,
            dashboards=[
                {
                    "path": Path("General") / "CPU_Main__cpu-main.json",
                    "dashboard": {
                        "id": None,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "templating": {
                            "list": [
                                {
                                    "name": "prom_ds",
                                    "type": "datasource",
                                    "query": "prometheus",
                                    "current": {},
                                }
                            ]
                        },
                        "panels": [
                            {
                                "id": 7,
                                "title": "CPU Usage",
                                "type": "timeseries",
                                "datasource": {
                                    "type": "prometheus",
                                    "uid": "prom-main",
                                },
                                "targets": [{"refId": "A", "expr": "up"}],
                            }
                        ],
                    },
                },
                {
                    "path": Path("Infra") / "Mixed_Main__mixed-main.json",
                    "dashboard": {
                        "id": None,
                        "uid": "mixed-main",
                        "title": "Mixed Main",
                        "templating": {
                            "list": [
                                {
                                    "name": "logs_ds",
                                    "type": "datasource",
                                    "query": "loki",
                                    "current": {},
                                }
                            ]
                        },
                        "panels": [
                            {
                                "id": 8,
                                "title": "Logs",
                                "type": "logs",
                                "datasource": "$logs_ds",
                                "targets": [
                                    {
                                        "refId": "A",
                                        "expr": '{job="grafana"}',
                                        "datasource": {
                                            "type": "loki",
                                            "uid": "logs-main",
                                        },
                                    },
                                    {
                                        "refId": "B",
                                        "query": "custom_query",
                                        "datasource": {
                                            "type": "custom-plugin",
                                            "uid": "custom-main",
                                        },
                                    },
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                },
            ],
            folders=[
                {
                    "uid": "infra",
                    "title": "Infra",
                    "parentUid": "platform",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                }
            ],
            datasources=[
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "logs-main",
                    "name": "Logs Main",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                    "type": "tempo",
                    "access": "proxy",
                    "url": "http://tempo:3200",
                    "isDefault": "false",
                    "org": "Main Org.",
                    "orgId": "1",
                },
            ],
        )

    def test_inspect_export_renders_human_summary(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_summary_fixture(
                import_dir,
                dashboards=[
                    {
                        "path": Path("General") / "CPU_Main__cpu-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "type": "timeseries",
                                    "datasource": {
                                        "type": "prometheus",
                                        "uid": "prom-main",
                                    },
                                    "targets": [{"refId": "A"}],
                                }
                            ],
                        },
                    },
                    {
                        "path": Path("Infra") / "Mixed_Main__mixed-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "mixed-main",
                            "title": "Mixed Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "type": "timeseries",
                                    "datasource": {
                                        "type": "datasource",
                                        "uid": "-- Mixed --",
                                    },
                                    "targets": [
                                        {
                                            "refId": "A",
                                            "datasource": {
                                                "type": "prometheus",
                                                "uid": "prom-main",
                                            },
                                        },
                                        {
                                            "refId": "B",
                                            "datasource": {
                                                "type": "loki",
                                                "uid": "logs-main",
                                            },
                                        },
                                    ],
                                }
                            ],
                        },
                        "meta": {"folderUid": "infra"},
                    },
                ],
                folders=[
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                datasources=[
                    {
                        "uid": "logs-main",
                        "name": "Logs Main",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki:3100",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": "true",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "unused-main",
                        "name": "Unused Main",
                        "type": "tempo",
                        "access": "proxy",
                        "url": "http://tempo:3200",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                index=[{"uid": "abc", "title": "CPU", "path": "General", "kind": "raw"}],
            )

            args = exporter.parse_args(["inspect-export", "--import-dir", str(import_dir)])
            result, output = self.run_inspect(args)

            self.assertEqual(result, 0)
            self.assertIn("Dashboards: 2", output)
            self.assertIn("Folders: 2", output)
            self.assertIn("Panels: 2", output)
            self.assertIn("Queries: 3", output)
            self.assertIn("Mixed datasource dashboards: 1", output)
            self.assertIn("Orphaned datasources: 1", output)
            self.assertIn("Platform / Infra (1 dashboards)", output)
            self.assertIn("prom-main (2 refs across 2 dashboards)", output)
            self.assertIn("logs-main (1 refs across 1 dashboards)", output)
            self.assertIn("Datasource inventory: 3", output)
            self.assertIn("Prometheus Main uid=prom-main", output)
            self.assertIn("Unused Main uid=unused-main", output)
            self.assertIn("Mixed Main (mixed-main) path=Platform / Infra", output)

    def test_inspect_export_renders_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_summary_fixture(
                import_dir,
                dashboards=[
                    {
                        "path": Path("General") / "CPU_Main__cpu-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "type": "timeseries",
                                    "datasource": {
                                        "type": "prometheus",
                                        "uid": "prom-main",
                                    },
                                    "targets": [{"refId": "A"}, {"refId": "B"}],
                                }
                            ],
                        },
                    }
                ],
                datasources=[
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": "true",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "unused-main",
                        "name": "Unused Main",
                        "type": "tempo",
                        "access": "proxy",
                        "url": "http://tempo:3200",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--json"]
            )
            result, output = self.run_inspect(args)
            payload = json.loads(output)

            self.assertEqual(result, 0)
            self.assertEqual(payload["summary"]["dashboardCount"], 1)
            self.assertEqual(payload["summary"]["panelCount"], 1)
            self.assertEqual(payload["summary"]["queryCount"], 2)
            self.assertEqual(payload["summary"]["datasourceInventoryCount"], 2)
            self.assertEqual(payload["summary"]["orphanedDatasourceCount"], 1)
            self.assertEqual(payload["folders"][0]["path"], "General")
            self.assertEqual(payload["datasources"][0]["name"], "prom-main")
            self.assertEqual(payload["datasourceInventory"][0]["name"], "Prometheus Main")
            self.assertEqual(payload["datasourceInventory"][0]["referenceCount"], 1)
            self.assertEqual(payload["orphanedDatasources"][0]["uid"], "unused-main")
            self.assertEqual(payload["orphanedDatasources"][0]["name"], "Unused Main")
            self.assertEqual(payload["dashboards"][0]["folderPath"], "General")
            self.assertFalse(payload["dashboards"][0]["mixedDatasource"])

    def test_inspect_export_renders_table_sections(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_summary_fixture(
                import_dir,
                dashboards=[
                    {
                        "path": Path("General") / "CPU_Main__cpu-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "type": "timeseries",
                                    "datasource": {
                                        "type": "prometheus",
                                        "uid": "prom-main",
                                    },
                                    "targets": [{"refId": "A"}],
                                }
                            ],
                        },
                    },
                    {
                        "path": Path("Infra") / "Mixed_Main__mixed-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "mixed-main",
                            "title": "Mixed Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "type": "timeseries",
                                    "datasource": {
                                        "type": "datasource",
                                        "uid": "-- Mixed --",
                                    },
                                    "targets": [
                                        {
                                            "refId": "A",
                                            "datasource": {
                                                "type": "prometheus",
                                                "uid": "prom-main",
                                            },
                                        },
                                        {
                                            "refId": "B",
                                            "datasource": {
                                                "type": "loki",
                                                "uid": "logs-main",
                                            },
                                        },
                                    ],
                                }
                            ],
                        },
                        "meta": {"folderUid": "infra"},
                    },
                ],
                folders=[
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                datasources=[
                    {
                        "uid": "logs-main",
                        "name": "Logs Main",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki:3100",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "unused-main",
                        "name": "Unused Main",
                        "type": "tempo",
                        "access": "proxy",
                        "url": "http://tempo:3200",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": "true",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--table"]
            )
            result, output = self.run_inspect(args)

            self.assertEqual(result, 0)
            self.assertIn("# Summary", output)
            self.assertIn("METRIC", output)
            self.assertIn("FOLDER_PATH", output)
            self.assertIn("DATASOURCE", output)
            self.assertIn("UID", output)
            self.assertIn("Platform / Infra", output)
            self.assertIn("prom-main", output)
            self.assertIn("# Datasource inventory", output)
            self.assertIn("# Orphaned datasources", output)
            self.assertIn("Prometheus Main", output)
            self.assertIn("Unused Main", output)
            self.assertIn("mixed-main", output)

    def test_inspect_export_renders_query_report_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "infra-main",
                    "title": "Infra Main",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU Usage",
                            "type": "timeseries",
                            "datasource": {"type": "prometheus", "uid": "prom-main"},
                            "targets": [
                                {
                                    "refId": "A",
                                    "expr": 'sum(rate(node_cpu_seconds_total{job="node"}[5m]))',
                                }
                            ],
                        },
                        {
                            "id": 8,
                            "title": "Flux Query",
                            "type": "table",
                            "datasource": {"type": "influxdb", "uid": "influx-main"},
                            "targets": [
                                {
                                    "refId": "B",
                                    "query": 'from(bucket: "prod") |> filter(fn: (r) => r._measurement == "cpu")',
                                }
                            ],
                        },
                    ],
                },
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "json"]
            )
            result, output = self.run_inspect(args)
            payload = json.loads(output)

            self.assertEqual(result, 0)
            self.assertEqual(payload["summary"]["dashboardCount"], 1)
            self.assertEqual(payload["summary"]["queryRecordCount"], 2)
            self.assertEqual(payload["queries"][0]["dashboardUid"], "infra-main")
            self.assertEqual(payload["queries"][0]["panelId"], "7")
            self.assertEqual(payload["queries"][0]["datasourceUid"], "prom-main")
            self.assertEqual(payload["queries"][0]["datasourceType"], "prometheus")
            self.assertEqual(payload["queries"][0]["datasourceFamily"], "prometheus")
            self.assertEqual(payload["queries"][0]["metrics"], ["node_cpu_seconds_total"])
            self.assertEqual(payload["queries"][1]["datasourceType"], "influxdb")
            self.assertEqual(payload["queries"][1]["datasourceFamily"], "influxdb")
            self.assertEqual(payload["queries"][1]["buckets"], ["prod"])
            self.assertEqual(payload["queries"][1]["measurements"], ["cpu"])

    def test_parse_args_supports_governance_report_formats(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report", "governance"]
        )
        self.assertEqual(args.report, "governance")

        governance_json_args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--basic-user",
                "admin",
                "--basic-password",
                "admin",
                "--report",
                "governance-json",
            ]
        )
        self.assertEqual(governance_json_args.report, "governance-json")

    def test_inspect_export_prometheus_metrics_ignore_grouping_labels_and_values(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "prom-main",
                    "title": "Prometheus Main",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU Usage",
                            "type": "timeseries",
                            "datasource": {"type": "prometheus", "uid": "prom-main"},
                            "targets": [
                                {
                                    "refId": "A",
                                    "expr": (
                                        'sum by (instance) (rate(node_cpu_seconds_total'
                                        '{job=~"node|api",mode!="idle"}[5m]))'
                                    ),
                                },
                                {
                                    "refId": "B",
                                    "expr": (
                                        'up{job="prometheus_build_info"} '
                                        '/ ignoring(job) group_left(instance) '
                                        'kube_pod_info{pod=~"node_cpu_seconds_total|api"}'
                                    ),
                                },
                                {
                                    "refId": "C",
                                    "expr": '{__name__="prometheus_build_info",job="api"}',
                                },
                            ],
                        }
                    ],
                },
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "json"]
            )
            result, output = self.run_inspect(args)
            payload = json.loads(output)

            self.assertEqual(result, 0)
            self.assertEqual(payload["queries"][0]["metrics"], ["node_cpu_seconds_total"])
            self.assertEqual(payload["queries"][1]["metrics"], ["up", "kube_pod_info"])
            self.assertEqual(payload["queries"][2]["metrics"], ["prometheus_build_info"])

    def test_inspect_export_renders_query_report_table(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "infra-main",
                    "title": "Infra Main",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU Usage",
                            "type": "timeseries",
                            "datasource": {"type": "prometheus", "uid": "prom-main"},
                            "targets": [{"refId": "A", "expr": "node_cpu_seconds_total"}],
                        }
                    ],
                },
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report"]
            )
            result, output = self.run_inspect(args)

            self.assertEqual(result, 0)
            self.assertIn("Export inspection report:", output)
            self.assertIn("# Query report", output)
            self.assertIn("DASHBOARD_UID", output)
            self.assertIn("CPU Usage", output)

    def test_inspect_export_renders_governance_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_governance_fixture(import_dir)

            args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "governance-json",
                    "--report-filter-datasource",
                    "logs-main",
                ]
            )
            result, output = self.run_inspect(args)
            payload = json.loads(output)

            self.assertEqual(result, 0)
            self.assertEqual(payload["summary"]["dashboardCount"], 2)
            self.assertEqual(payload["summary"]["queryRecordCount"], 1)
            self.assertEqual(payload["summary"]["datasourceFamilyCount"], 1)
            self.assertEqual(payload["datasourceFamilies"][0]["family"], "loki")
            self.assertEqual(len(payload["dashboardDependencies"]), 2)
            self.assertEqual(payload["dashboardDependencies"][0]["dashboardUid"], "cpu-main")
            self.assertEqual(
                payload["dashboardDependencies"][0]["pluginIds"],
                ["timeseries"],
            )
            self.assertEqual(
                payload["dashboardDependencies"][0]["datasourceVariables"],
                ["prom_ds"],
            )
            self.assertEqual(
                payload["dashboardDependencies"][1]["pluginIds"],
                ["logs"],
            )
            self.assertEqual(
                payload["dashboardDependencies"][1]["datasourceVariables"],
                ["logs_ds"],
            )
            self.assertEqual(
                payload["dashboardDependencies"][1]["datasourceVariableRefs"],
                ["logs_ds"],
            )
            self.assertEqual(payload["datasources"][0]["datasourceUid"], "logs-main")
            self.assertEqual(len(payload["riskRecords"]), 2)
            self.assertEqual(payload["riskRecords"][0]["kind"], "orphaned-datasource")
            self.assertEqual(payload["riskRecords"][0]["category"], "inventory")
            self.assertIn(
                "Remove the unused datasource",
                payload["riskRecords"][0]["recommendation"],
            )
            self.assertEqual(
                payload["riskRecords"][1]["kind"], "mixed-datasource-dashboard"
            )
            self.assertEqual(payload["riskRecords"][1]["category"], "topology")
            self.assertIn(
                "Split panel queries by datasource",
                payload["riskRecords"][1]["recommendation"],
            )

    def test_inspect_export_renders_governance_tables(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_governance_fixture(import_dir)

            args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "governance",
                ]
            )
            result, output = self.run_inspect(args)

            self.assertEqual(result, 0)
            self.assertIn("Export inspection governance: %s" % import_dir, output)
            self.assertIn("# Summary", output)
            self.assertIn("# Datasource Families", output)
            self.assertIn("# Datasources", output)
            self.assertIn("# Risks", output)
            self.assertIn("CATEGORY", output)
            self.assertIn("RECOMMENDATION", output)
            self.assertIn("mixed-datasource-dashboard", output)
            self.assertIn("orphaned-datasource", output)
            self.assertIn("unknown-datasource-family", output)
            self.assertIn("Remove the unused datasource", output)
            self.assertIn("CATEGORY", output)
            self.assertIn("RECOMMENDATION", output)
            self.assertIn("Normalize the datasource type mapping", output)

    def test_inspect_export_renders_tree_and_tree_table_reports(self):
        dashboard = {
            "id": None,
            "uid": "infra-main",
            "title": "Infra Main",
            "panels": [
                {
                    "id": 7,
                    "title": "CPU Usage",
                    "type": "timeseries",
                    "datasource": {"type": "prometheus", "uid": "prom-main"},
                    "targets": [
                        {
                            "refId": "A",
                            "expr": 'sum(rate(node_cpu_seconds_total{job="node"}[5m]))',
                        }
                    ],
                },
                {
                    "id": 8,
                    "title": "Logs",
                    "type": "logs",
                    "datasource": {"type": "loki", "uid": "logs-main"},
                    "targets": [{"refId": "B", "expr": '{job="grafana"}'}],
                },
            ],
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(import_dir, dashboard)

            tree_args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "tree"]
            )
            tree_result, tree_output = self.run_inspect(tree_args)
            self.assertEqual(tree_result, 0)
            self.assertIn("Export inspection tree report:", tree_output)
            self.assertIn("[1] Dashboard infra-main", tree_output)
            self.assertIn("Panel 7 title=CPU Usage", tree_output)

            table_args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "tree-table"]
            )
            table_result, table_output = self.run_inspect(table_args)
            self.assertEqual(table_result, 0)
            self.assertIn("Export inspection tree-table report:", table_output)
            self.assertIn("# Dashboard sections", table_output)
            self.assertIn("DASHBOARD_UID", table_output)
            self.assertIn('{job="grafana"}', table_output)

    def test_inspect_export_tree_and_tree_table_filters_and_columns(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "multi-panel",
                    "title": "Multi Panel",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU",
                            "type": "timeseries",
                            "datasource": {"type": "prometheus", "uid": "prom-main"},
                            "targets": [{"refId": "A", "expr": "up"}],
                        },
                        {
                            "id": 8,
                            "title": "Logs",
                            "type": "logs",
                            "datasource": {"type": "loki", "uid": "logs-main"},
                            "targets": [{"refId": "B", "expr": '{job="grafana"}'}],
                        },
                    ],
                },
            )

            tree_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "tree",
                    "--report-filter-panel-id",
                    "7",
                ]
            )
            _, tree_output = self.run_inspect(tree_args)
            self.assertIn("Panel 7 title=CPU", tree_output)
            self.assertNotIn("Panel 8 title=Logs", tree_output)

            table_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "tree-table",
                    "--no-header",
                    "--report-columns",
                    "panel_id,query",
                ]
            )
            _, table_output = self.run_inspect(table_args)
            self.assertIn("[1] Dashboard multi-panel", table_output)
            self.assertIn("7         up", table_output)
            self.assertNotIn("PANEL_ID", table_output)

    def test_inspect_export_renders_csv_and_selected_columns(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "infra-main",
                    "title": "Infra Main",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU Usage",
                            "type": "timeseries",
                            "datasource": {"type": "prometheus", "uid": "prom-main"},
                            "targets": [{"refId": "A", "expr": "up"}],
                        }
                    ],
                },
            )

            csv_args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "csv"]
            )
            _, csv_output = self.run_inspect(csv_args)
            self.assertIn("dashboard_uid,dashboard_title,folder_path,panel_id", csv_output)
            self.assertIn("infra-main,Infra Main,General,7", csv_output)

            table_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "--report-columns",
                    "dashboardUid,datasource,metrics",
                ]
            )
            _, table_output = self.run_inspect(table_args)
            self.assertIn("DASHBOARD_UID", table_output)
            self.assertNotIn("PANEL_TITLE", table_output)

            csv_columns_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "csv",
                    "--report-columns",
                    "dashboard_uid,datasource_uid,datasource,datasource_family,query",
                ]
            )
            _, csv_columns_output = self.run_inspect(csv_columns_args)
            self.assertIn(
                "dashboard_uid,datasource_uid,datasource,datasource_family,query",
                csv_columns_output.splitlines()[0],
            )
            self.assertIn(
                "infra-main,prom-main,prom-main,prometheus,up",
                csv_columns_output,
            )

    def test_inspect_export_filters_query_report(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "mixed-main",
                    "title": "Mixed Main",
                    "panels": [
                        {
                            "id": 1,
                            "title": "Mixed Panel",
                            "type": "timeseries",
                            "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                            "targets": [
                                {
                                    "refId": "A",
                                    "datasource": {"type": "prometheus", "uid": "prom-main"},
                                    "expr": "up",
                                },
                                {
                                    "refId": "B",
                                    "datasource": {"type": "loki", "uid": "logs-main"},
                                    "expr": '{job="grafana"}',
                                },
                            ],
                        }
                    ],
                },
            )

            json_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "json",
                    "--report-filter-datasource",
                    "prom-main",
                ]
            )
            _, json_output = self.run_inspect(json_args)
            payload = json.loads(json_output)
            self.assertEqual(payload["summary"]["queryRecordCount"], 1)
            self.assertEqual(payload["queries"][0]["datasource"], "prom-main")

    def test_filter_export_inspection_report_document_matches_datasource_uid_type_and_family(self):
        document = {
            "summary": {"dashboardCount": 2, "queryRecordCount": 2},
            "queries": [
                {
                    "dashboardUid": "cpu-main",
                    "panelId": "7",
                    "datasource": "prom-main",
                    "datasourceUid": "prom-uid",
                    "datasourceType": "prometheus",
                    "datasourceFamily": "prometheus",
                },
                {
                    "dashboardUid": "logs-main",
                    "panelId": "8",
                    "datasource": "logs-main",
                    "datasourceUid": "logs-uid",
                    "datasourceType": "loki",
                    "datasourceFamily": "loki",
                },
            ],
        }

        for value in ("prom-main", "prom-uid", "prometheus"):
            filtered = exporter.filter_export_inspection_report_document(
                document,
                datasource_label=value,
            )
            self.assertEqual(filtered["summary"]["dashboardCount"], 1)
            self.assertEqual(filtered["summary"]["queryRecordCount"], 1)
            self.assertEqual(filtered["queries"][0]["datasource"], "prom-main")

    def test_inspect_export_help_describes_shared_report_columns_and_datasource_filter(self):
        parser = argparse.ArgumentParser()
        exporter.add_inspect_export_cli_args(parser)

        help_text = parser.format_help()

        self.assertIn("datasourceType", help_text)
        self.assertIn("datasourceFamily", help_text)
        self.assertIn("file", help_text)
        self.assertIn("datasource label,", help_text)
        self.assertIn("uid, type,", help_text)
        self.assertIn("or family exactly matches this value", help_text)
        self.assertIn(
            "dashboard_uid,datasource_uid,datasource_family,query,file",
            exporter.INSPECT_EXPORT_HELP_FULL_EXAMPLES,
        )

    def test_inspect_export_filters_query_report_by_datasource_uid_type_or_family(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_summary_fixture(
                import_dir,
                dashboards=[
                    {
                        "path": Path("General") / "Mixed_Main__mixed-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "mixed-main",
                            "title": "Mixed Main",
                            "panels": [
                                {
                                    "id": 1,
                                    "title": "Mixed Panel",
                                    "type": "timeseries",
                                    "targets": [
                                        {
                                            "refId": "A",
                                            "datasource": {
                                                "type": "grafana-postgresql-datasource",
                                                "uid": "pg-main",
                                            },
                                            "rawSql": "select * from cpu_usage",
                                        },
                                        {
                                            "refId": "B",
                                            "datasource": {
                                                "type": "loki",
                                                "uid": "logs-main",
                                            },
                                            "expr": '{job="grafana"}',
                                        },
                                    ],
                                }
                            ],
                        },
                    }
                ],
                datasources=[
                    {"uid": "pg-main", "name": "Postgres Main", "type": "grafana-postgresql-datasource"},
                    {"uid": "logs-main", "name": "Logs Main", "type": "loki"},
                ],
            )

            cases = [
                ("pg-main", "pg-main", "grafana-postgresql-datasource", "postgres"),
                (
                    "grafana-postgresql-datasource",
                    "pg-main",
                    "grafana-postgresql-datasource",
                    "postgres",
                ),
                ("postgres", "pg-main", "grafana-postgresql-datasource", "postgres"),
            ]
            for filter_value, expected_uid, expected_type, expected_family in cases:
                args = exporter.parse_args(
                    [
                        "inspect-export",
                        "--import-dir",
                        str(import_dir),
                        "--report",
                        "json",
                        "--report-filter-datasource",
                        filter_value,
                    ]
                )
                _, json_output = self.run_inspect(args)
                payload = json.loads(json_output)

                self.assertEqual(payload["summary"]["queryRecordCount"], 1)
                self.assertEqual(payload["queries"][0]["datasourceUid"], expected_uid)
                self.assertEqual(payload["queries"][0]["datasourceType"], expected_type)
                self.assertEqual(payload["queries"][0]["datasourceFamily"], expected_family)

    def test_inspect_export_filters_query_report_by_datasource_uid_type_and_family(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "mixed-main",
                    "title": "Mixed Main",
                    "panels": [
                        {
                            "id": 1,
                            "title": "Mixed Panel",
                            "type": "timeseries",
                            "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                            "targets": [
                                {
                                    "refId": "A",
                                    "datasource": {"type": "prometheus", "uid": "prom-main"},
                                    "expr": "up",
                                },
                                {
                                    "refId": "B",
                                    "datasource": {"type": "loki", "uid": "logs-main"},
                                    "expr": '{job="grafana"}',
                                },
                            ],
                        }
                    ],
                },
            )

            for datasource_filter, expected_datasource in [
                ("prom-main", "prom-main"),
                ("prometheus", "prom-main"),
                ("loki", "logs-main"),
            ]:
                args = exporter.parse_args(
                    [
                        "inspect-export",
                        "--import-dir",
                        str(import_dir),
                        "--report",
                        "json",
                        "--report-filter-datasource",
                        datasource_filter,
                    ]
                )
                _, json_output = self.run_inspect(args)
                payload = json.loads(json_output)
                self.assertEqual(
                    payload["queries"][0]["datasource"],
                    expected_datasource,
                    datasource_filter,
                )

    def test_inspect_export_help_lists_supported_report_columns_and_filter_matching(self):
        parser = argparse.ArgumentParser()
        exporter.add_inspect_export_cli_args(parser)

        help_text = parser.format_help()

        self.assertIn("datasourceUid", help_text)
        self.assertIn("datasourceType", help_text)
        self.assertIn("datasourceFamily", help_text)
        self.assertIn("file", help_text)
        self.assertIn("dashboard_uid", help_text)
        self.assertIn("datasource label,", help_text)
        self.assertIn("uid, type,", help_text)
        self.assertIn("or family exactly matches this value", help_text)

    def test_inspect_live_help_lists_supported_report_columns_and_filter_matching(self):
        parser = argparse.ArgumentParser()
        exporter.add_inspect_live_cli_args(parser)

        help_text = parser.format_help()

        self.assertIn("datasourceUid", help_text)
        self.assertIn("datasourceType", help_text)
        self.assertIn("datasourceFamily", help_text)
        self.assertIn("file", help_text)
        self.assertIn("dashboard_uid", help_text)
        self.assertIn("datasource label,", help_text)
        self.assertIn("uid, type,", help_text)
        self.assertIn("or family exactly matches this value", help_text)

    def test_inspect_live_renders_report_json_from_mocked_client(self):
        fake_client = FakeDashboardWorkflowClient(
            summaries=[
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "folderUid": "infra",
                    "folderTitle": "Infra",
                }
            ],
            dashboards={
                "cpu-main": {
                    "dashboard": {
                        "id": 1,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "panels": [
                            {
                                "id": 7,
                                "title": "CPU Panel",
                                "type": "timeseries",
                                "datasource": {"type": "prometheus", "uid": "prom-main"},
                                "targets": [{"refId": "A", "expr": "up"}],
                            }
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                }
            },
            datasources=[
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ],
            folders={
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                }
            },
        )
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--report",
                "json",
                "--report-filter-panel-id",
                "7",
            ]
        )
        stdout = io.StringIO()
        with mock.patch.object(exporter, "build_client", return_value=fake_client):
            with redirect_stdout(stdout):
                result = exporter.inspect_live(args)

        payload = json.loads(stdout.getvalue())
        self.assertEqual(result, 0)
        self.assertEqual(payload["summary"]["dashboardCount"], 1)
        self.assertEqual(payload["queries"][0]["folderPath"], "Platform / Infra")
        self.assertEqual(payload["queries"][0]["metrics"], ["up"])

    def test_inspect_export_validation_errors(self):
        cases = [
            (
                ["inspect-export", "--import-dir", "dashboards/raw", "--no-header"],
                "--no-header is only supported with --table, table-like --report, or compatible --output-format values",
            ),
            (
                ["inspect-export", "--import-dir", "dashboards/raw", "--table", "--json"],
                "--table and --json are mutually exclusive",
            ),
            (
                ["inspect-export", "--import-dir", "dashboards/raw", "--report", "--table"],
                "--report cannot be combined with --table or --json",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--output-format",
                    "json",
                    "--table",
                ],
                "--output-format cannot be combined with --json, --table, or --report",
            ),
            (
                ["inspect-export", "--import-dir", "dashboards/raw", "--report-columns", "dashboardUid,datasource"],
                "--report-columns is only supported with --report or report-like --output-format",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report",
                    "json",
                    "--report-columns",
                    "dashboardUid,datasource",
                ],
                "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report",
                    "governance",
                    "--report-columns",
                    "dashboardUid,datasource",
                ],
                "--report-columns is not supported with governance output",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report-filter-datasource",
                    "prom-main",
                ],
                "--report-filter-datasource is only supported with --report or report-like --output-format",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report-filter-panel-id",
                    "7",
                ],
                "--report-filter-panel-id is only supported with --report or report-like --output-format",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report",
                    "--report-columns",
                    "dashboardUid,unknown",
                ],
                "Unsupported report column\\(s\\): unknown",
            ),
        ]

        for argv, message in cases:
            args = exporter.parse_args(argv)
            with self.assertRaisesRegex(exporter.GrafanaError, message):
                exporter.inspect_export(args)

    def test_inspect_export_accepts_report_like_output_format(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--output-format",
                "report-json",
                "--report-filter-datasource",
                "prom-main",
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_report_fixture(
                import_dir,
                {
                    "id": None,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "panels": [
                        {
                            "id": 7,
                            "title": "CPU",
                            "type": "timeseries",
                            "targets": [{"refId": "A", "expr": "up"}],
                            "datasource": {"uid": "prom-main", "type": "prometheus"},
                        }
                    ],
                },
            )
            args.import_dir = str(import_dir)
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

        self.assertEqual(result, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["summary"]["dashboardCount"], 1)

    def test_inspect_export_outputs_dependency_contract_json(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--report",
                "dependency",
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self.write_summary_fixture(
                import_dir,
                dashboards=[
                    {
                        "path": Path("General") / "CPU_Main__cpu-main.json",
                        "dashboard": {
                            "id": None,
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "panels": [
                                {
                                    "id": 7,
                                    "title": "CPU",
                                    "type": "timeseries",
                                    "targets": [
                                        {
                                            "refId": "A",
                                            "expr": "sum(rate(node_cpu_seconds_total[5m]))",
                                        }
                                    ],
                                    "datasource": {"uid": "prom-main", "type": "prometheus"},
                                }
                            ],
                        },
                    }
                ],
                datasources=[
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": True,
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                folders=[],
            )
            args.import_dir = str(import_dir)
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

        payload = json.loads(stdout.getvalue())
        self.assertEqual(result, 0)
        self.assertEqual(
            payload["kind"],
            "grafana-utils-dashboard-dependency-contract",
        )
        self.assertEqual(payload["queryCount"], 1)
        self.assertEqual(payload["datasourceCount"], 1)
        self.assertEqual(len(payload["queries"]), 1)
        self.assertEqual(payload["queries"][0]["datasourceFamily"], "prometheus")
        self.assertEqual(len(payload["datasourceUsage"]), 1)
        self.assertEqual(payload["datasourceUsage"][0]["datasource"], "prom-main")
