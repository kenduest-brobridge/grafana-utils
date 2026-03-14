import importlib
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

exporter = importlib.import_module("grafana_utils.dashboard_cli")


class FakeDashboardIntegrationClient:
    def __init__(self, dashboards=None, folders=None):
        self.dashboards = dashboards or {}
        self.folders = folders or {}
        self.headers = {}
        self.created_folders = []
        self.imported_payloads = []

    def fetch_dashboard_if_exists(self, uid):
        return self.dashboards.get(uid)

    def fetch_dashboard(self, uid):
        if uid not in self.dashboards:
            raise exporter.GrafanaApiError(404, "/api/dashboards/uid/%s" % uid, "not found")
        return self.dashboards[uid]

    def fetch_folder_if_exists(self, uid):
        return self.folders.get(uid)

    def create_folder(self, uid, title, parent_uid=None):
        record = {"uid": uid, "title": title}
        if parent_uid:
            record["parentUid"] = parent_uid
        self.created_folders.append(record)
        self.folders[uid] = dict(record)
        return {"status": "success", "uid": uid}

    def import_dashboard(self, payload):
        self.imported_payloads.append(payload)
        return {"status": "success", "uid": payload["dashboard"].get("uid")}

    def fetch_current_org(self):
        return {"id": 1, "name": "Main Org."}


class DashboardIntegrationFlowTests(unittest.TestCase):
    def test_main_inspect_export_json_summarizes_raw_inventory_and_mixed_dashboards(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                    datasources_file=exporter.DATASOURCE_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "platform",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                [
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
                        "uid": "loki-main",
                        "name": "Loki Logs",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki:3100",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.DATASOURCE_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {
                        "id": None,
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "panels": [
                            {
                                "id": 1,
                                "type": "timeseries",
                                "datasource": {"type": "prometheus", "uid": "prom-main"},
                                "targets": [{"refId": "A", "expr": "sum(up)"}],
                            }
                        ],
                    },
                    "meta": {},
                },
                import_dir / "General" / "CPU_Main__cpu-main.json",
            )
            exporter.write_json_document(
                {
                    "dashboard": {
                        "id": None,
                        "uid": "mixed-main",
                        "title": "Mixed Main",
                        "panels": [
                            {
                                "id": 2,
                                "type": "timeseries",
                                "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                                "targets": [
                                    {
                                        "refId": "A",
                                        "datasource": {"type": "prometheus", "uid": "prom-main"},
                                    },
                                    {
                                        "refId": "B",
                                        "datasource": {"type": "loki", "uid": "loki-main"},
                                    },
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                },
                import_dir / "Infra" / "Mixed_Main__mixed-main.json",
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.main(
                    ["inspect-export", "--import-dir", str(import_dir), "--json"]
                )

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["summary"]["dashboardCount"], 2)
            self.assertEqual(payload["summary"]["folderCount"], 3)
            self.assertEqual(payload["summary"]["panelCount"], 2)
            self.assertEqual(payload["summary"]["queryCount"], 3)
            self.assertEqual(payload["summary"]["mixedDatasourceDashboardCount"], 1)
            self.assertEqual(payload["summary"]["datasourceInventoryCount"], 2)
            self.assertEqual(payload["folders"][1]["path"], "Platform / Infra")
            self.assertEqual(payload["folders"][2]["path"], "General")
            self.assertEqual(payload["datasources"][0]["name"], "loki-main")
            self.assertEqual(payload["datasourceInventory"][0]["dashboardCount"], 1)
            self.assertEqual(
                payload["mixedDatasourceDashboards"][0]["folderPath"],
                "Platform / Infra",
            )
            self.assertEqual(
                payload["mixedDatasourceDashboards"][0]["datasources"],
                ["loki-main", "prom-main"],
            )

    def test_main_import_dry_run_json_reports_folder_checks_and_update_skip_actions(self):
        client = FakeDashboardIntegrationClient(
            dashboards={
                "cpu-main": {
                    "dashboard": {"uid": "cpu-main", "title": "CPU Main"},
                    "meta": {"folderUid": "infra", "folderTitle": "Infra"},
                }
            },
            folders={
                "platform": {"uid": "platform", "title": "Platform", "parentUid": "", "parents": []},
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "parentUid": "platform",
                    "parents": [{"title": "Platform"}],
                },
            },
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "platform",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "prod",
                        "title": "Prod",
                        "parentUid": "infra",
                        "path": "Platform / Infra / Prod",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "cpu-main", "title": "CPU Main", "panels": []},
                    "meta": {"folderUid": "infra"},
                },
                import_dir / "Infra" / "CPU_Main__cpu-main.json",
            )
            exporter.write_json_document(
                {
                    "dashboard": {
                        "id": None,
                        "uid": "prod-main",
                        "title": "Prod Main",
                        "panels": [],
                    },
                    "meta": {"folderUid": "prod"},
                },
                import_dir / "Prod" / "Prod_Main__prod-main.json",
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.main(
                        [
                            "import-dashboard",
                            "--import-dir",
                            str(import_dir),
                            "--dry-run",
                            "--json",
                            "--ensure-folders",
                            "--update-existing-only",
                        ]
                    )

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["mode"], "update-or-skip-missing")
            self.assertEqual(payload["summary"]["folderCount"], 3)
            self.assertEqual(payload["summary"]["missingFolders"], 1)
            self.assertEqual(payload["summary"]["dashboardCount"], 2)
            self.assertEqual(payload["summary"]["missingDashboards"], 1)
            self.assertEqual(payload["summary"]["skippedMissingDashboards"], 1)
            self.assertEqual(payload["folders"][0]["status"], "match")
            self.assertEqual(payload["folders"][2]["status"], "missing")
            self.assertEqual(payload["dashboards"][0]["action"], "update")
            self.assertEqual(payload["dashboards"][0]["folderPath"], "Platform / Infra")
            self.assertEqual(payload["dashboards"][1]["action"], "skip-missing")
            self.assertEqual(
                payload["dashboards"][1]["folderPath"], "Platform / Infra / Prod"
            )


if __name__ == "__main__":
    unittest.main()
