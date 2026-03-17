import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
FIXTURE_PATH = REPO_ROOT / "fixtures" / "datasource_contract_cases.json"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))
datasource_diff = importlib.import_module("grafana_utils.datasource_diff")
dashboard_cli = importlib.import_module("grafana_utils.dashboard_cli")


class FakeDatasourceClient(object):
    def __init__(self, org, datasources):
        self._org = org
        self._datasources = datasources

    def fetch_current_org(self):
        return self._org

    def list_datasources(self):
        return list(self._datasources)


class DatasourceDiffScaffoldTests(unittest.TestCase):
    def _load_contract_cases(self):
        return json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))

    def test_datasource_diff_load_datasource_diff_bundle_normalizes_records(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            (import_dir / "export-metadata.json").write_text(
                json.dumps(
                    {
                        "schemaVersion": 1,
                        "kind": "grafana-utils-datasource-export-index",
                        "resource": "datasource",
                    }
                ),
                encoding="utf-8",
            )
            (import_dir / "datasources.json").write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": True,
                            "orgId": 7,
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (import_dir / "index.json").write_text("{}", encoding="utf-8")

            bundle = datasource_diff.load_datasource_diff_bundle(import_dir)

        self.assertEqual(bundle["records"][0]["uid"], "prom-main")
        self.assertEqual(bundle["records"][0]["isDefault"], "true")
        self.assertEqual(bundle["records"][0]["orgId"], "7")

    def test_datasource_diff_normalize_datasource_record_matches_shared_contract_fixtures(
        self,
    ):
        for case in self._load_contract_cases():
            with self.subTest(case=case["name"]):
                self.assertEqual(
                    datasource_diff.normalize_datasource_record(case["rawDatasource"]),
                    case["expectedNormalizedRecord"],
                )

    def test_datasource_diff_load_datasource_diff_bundle_rejects_extra_contract_fields(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            (import_dir / "export-metadata.json").write_text(
                json.dumps(
                    {
                        "schemaVersion": 1,
                        "kind": "grafana-utils-datasource-export-index",
                        "resource": "datasource",
                    }
                ),
                encoding="utf-8",
            )
            (import_dir / "datasources.json").write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": True,
                            "org": "Main Org.",
                            "orgId": 7,
                            "password": "secret-password",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (import_dir / "index.json").write_text("{}", encoding="utf-8")

            with self.assertRaisesRegex(
                dashboard_cli.GrafanaError,
                "unsupported datasource field\\(s\\): password",
            ):
                datasource_diff.load_datasource_diff_bundle(import_dir)

    def test_datasource_diff_load_datasource_diff_bundle_rejects_wrong_metadata_kind(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            (import_dir / "export-metadata.json").write_text(
                json.dumps(
                    {
                        "schemaVersion": 1,
                        "kind": "wrong-kind",
                        "resource": "datasource",
                    }
                ),
                encoding="utf-8",
            )
            (import_dir / "datasources.json").write_text("[]", encoding="utf-8")
            (import_dir / "index.json").write_text("{}", encoding="utf-8")

            with self.assertRaises(dashboard_cli.GrafanaError):
                datasource_diff.load_datasource_diff_bundle(import_dir)

    def test_datasource_diff_build_live_datasource_diff_records_uses_export_shape(self):
        client = FakeDatasourceClient(
            org={"id": 3, "name": "Main Org."},
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
        )

        records = datasource_diff.build_live_datasource_diff_records(client)

        self.assertEqual(records[0]["uid"], "prom-main")
        self.assertEqual(records[0]["org"], "Main Org.")
        self.assertEqual(records[0]["orgId"], "3")

    def test_datasource_diff_compare_datasource_inventory_reports_match_difference_missing_and_extra(
        self,
    ):
        bundle_records = [
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
            {
                "uid": "loki-main",
                "name": "Loki Main",
                "type": "loki",
                "access": "proxy",
                "url": "http://loki:3100",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
            {
                "uid": "tempo-main",
                "name": "Tempo Main",
                "type": "tempo",
                "access": "proxy",
                "url": "http://tempo:3200",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
        ]
        live_records = [
            dict(bundle_records[0]),
            dict(bundle_records[1], url="http://loki-alt:3100"),
            {
                "uid": "pyroscope-main",
                "name": "Pyroscope Main",
                "type": "grafana-pyroscope-datasource",
                "access": "proxy",
                "url": "http://pyroscope:4040",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
        ]

        report = datasource_diff.compare_datasource_inventory(
            bundle_records, live_records
        )
        by_identity = dict((item["identity"], item) for item in report["items"])

        self.assertEqual(report["summary"]["matchCount"], 1)
        self.assertEqual(report["summary"]["differentCount"], 1)
        self.assertEqual(report["summary"]["missingLiveCount"], 1)
        self.assertEqual(report["summary"]["extraLiveCount"], 1)
        self.assertEqual(report["summary"]["diffCount"], 3)
        self.assertEqual(by_identity["prom-main"]["status"], "match")
        self.assertEqual(by_identity["loki-main"]["status"], "different")
        self.assertEqual(by_identity["loki-main"]["changedFields"], ["url"])
        self.assertEqual(by_identity["tempo-main"]["status"], "missing-live")
        self.assertEqual(by_identity["pyroscope-main"]["status"], "extra-live")

    def test_datasource_diff_compare_datasource_inventory_uses_name_fallback_and_flags_ambiguous_live_name(
        self,
    ):
        bundle_records = [
            {
                "uid": "",
                "name": "Shared Loki",
                "type": "loki",
                "access": "proxy",
                "url": "http://loki:3100",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
            {
                "uid": "",
                "name": "Duplicated",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "false",
                "org": "Main Org.",
                "orgId": "1",
            },
        ]
        live_records = [
            dict(bundle_records[0], uid="loki-main"),
            dict(bundle_records[1], uid="prom-a"),
            dict(bundle_records[1], uid="prom-b"),
        ]

        report = datasource_diff.compare_datasource_inventory(
            bundle_records, live_records
        )
        by_identity = dict((item["identity"], item) for item in report["items"])

        self.assertEqual(by_identity["Shared Loki"]["status"], "match")
        self.assertEqual(by_identity["Shared Loki"]["matchKey"], "name")
        self.assertEqual(by_identity["Duplicated"]["status"], "ambiguous-live-name")
        self.assertEqual(report["summary"]["ambiguousCount"], 1)


if __name__ == "__main__":
    unittest.main()
