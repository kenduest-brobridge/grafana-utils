import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

from tests.test_python_dashboard_cli import FakeDashboardWorkflowClient, exporter


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
            exporter.build_export_metadata(
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
            exporter.build_export_metadata(
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
            self.assertIn("Platform / Infra (1 dashboards)", output)
            self.assertIn("prom-main (2 refs across 2 dashboards)", output)
            self.assertIn("logs-main (1 refs across 1 dashboards)", output)
            self.assertIn("Datasource inventory: 2", output)
            self.assertIn("Prometheus Main uid=prom-main", output)
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
                    }
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
            self.assertEqual(payload["summary"]["datasourceInventoryCount"], 1)
            self.assertEqual(payload["folders"][0]["path"], "General")
            self.assertEqual(payload["datasources"][0]["name"], "prom-main")
            self.assertEqual(payload["datasourceInventory"][0]["name"], "Prometheus Main")
            self.assertEqual(payload["datasourceInventory"][0]["referenceCount"], 1)
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
            self.assertIn("Prometheus Main", output)
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
            self.assertEqual(payload["queries"][0]["metrics"], ["node_cpu_seconds_total"])
            self.assertEqual(payload["queries"][1]["buckets"], ["prod"])
            self.assertEqual(payload["queries"][1]["measurements"], ["cpu"])

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
                    "dashboard_uid,datasource_uid,datasource,query",
                ]
            )
            _, csv_columns_output = self.run_inspect(csv_columns_args)
            self.assertIn(
                "dashboard_uid,datasource_uid,datasource,query",
                csv_columns_output.splitlines()[0],
            )
            self.assertIn("infra-main,prom-main,prom-main,up", csv_columns_output)

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

            csv_args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "csv",
                    "--report-filter-datasource",
                    "prom-main",
                ]
            )
            _, csv_output = self.run_inspect(csv_args)
            self.assertIn("prom-main", csv_output)
            self.assertNotIn("logs-main", csv_output)

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
                "--no-header is only supported with --table, --report table, --report csv, or --report tree-table",
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
                ["inspect-export", "--import-dir", "dashboards/raw", "--report-columns", "dashboardUid,datasource"],
                "--report-columns is only supported with --report",
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
                "--report-columns is only supported with --report table, --report csv, or --report tree-table",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report-filter-datasource",
                    "prom-main",
                ],
                "--report-filter-datasource is only supported with --report",
            ),
            (
                [
                    "inspect-export",
                    "--import-dir",
                    "dashboards/raw",
                    "--report-filter-panel-id",
                    "7",
                ],
                "--report-filter-panel-id is only supported with --report",
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
