import argparse
import base64
import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("grafana-utils.py")
SPEC = importlib.util.spec_from_file_location("grafana_utils_script", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Cannot load module from {MODULE_PATH}")
exporter = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(exporter)


class FakeGrafanaClient(exporter.GrafanaClient):
    def __init__(self, pages):
        self.pages = pages
        self.calls = []

    def request_json(self, path, params=None):
        self.calls.append((path, params))
        if path == "/api/search":
            page = params["page"]
            return self.pages.get(page, [])
        raise AssertionError(f"Unexpected path {path}")


class ExporterTests(unittest.TestCase):
    def test_parse_args_supports_import_mode(self):
        args = exporter.parse_args(["--import-dir", "dashboards"])

        self.assertEqual(args.import_dir, "dashboards")

    def test_parse_args_defaults_output_dir_to_dashboards(self):
        args = exporter.parse_args([])

        self.assertEqual(args.output_dir, "dashboards")

    def test_parse_args_supports_variant_switches(self):
        args = exporter.parse_args(["--without-raw", "--without-prompt"])

        self.assertTrue(args.without_raw)
        self.assertTrue(args.without_prompt)

    def test_parse_args_disables_ssl_verification_by_default(self):
        args = exporter.parse_args([])

        self.assertFalse(args.verify_ssl)

    def test_parse_args_can_enable_ssl_verification(self):
        args = exporter.parse_args(["--verify-ssl"])

        self.assertTrue(args.verify_ssl)

    def test_resolve_auth_prefers_token(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="user",
            password="pass",
        )

        headers = exporter.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer abc123")

    def test_resolve_auth_supports_basic_auth(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password="pass",
        )

        headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:pass").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")

    def test_sanitize_path_component(self):
        self.assertEqual(exporter.sanitize_path_component(" Ops / CPU % "), "Ops_CPU")
        self.assertEqual(exporter.sanitize_path_component("..."), "untitled")

    def test_build_output_path_keeps_folder_structure(self):
        path = exporter.build_output_path(
            Path("out"),
            {"folderTitle": "Infra Team", "title": "Cluster Health", "uid": "abc"},
            flat=False,
        )

        self.assertEqual(path, Path("out/Infra_Team/Cluster_Health__abc.json"))

    def test_build_export_variant_dirs(self):
        raw_dir, prompt_dir = exporter.build_export_variant_dirs(Path("dashboards"))

        self.assertEqual(raw_dir, Path("dashboards/raw"))
        self.assertEqual(prompt_dir, Path("dashboards/prompt"))

    def test_iter_dashboard_summaries_paginates_and_deduplicates(self):
        client = FakeGrafanaClient(
            {
                1: [{"uid": "a", "title": "A"}, {"uid": "b", "title": "B"}],
                2: [{"uid": "b", "title": "B2"}, {"uid": "c", "title": "C"}],
                3: [],
            }
        )

        dashboards = client.iter_dashboard_summaries(page_size=2)

        self.assertEqual([item["uid"] for item in dashboards], ["a", "b", "c"])
        self.assertEqual(
            client.calls,
            [
                ("/api/search", {"type": "dash-db", "limit": 2, "page": 1}),
                ("/api/search", {"type": "dash-db", "limit": 2, "page": 2}),
                ("/api/search", {"type": "dash-db", "limit": 2, "page": 3}),
            ],
        )

    def test_write_dashboard_obeys_overwrite_flag(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "dash.json"
            exporter.write_dashboard({"dashboard": {"uid": "x"}}, path, overwrite=False)
            with self.assertRaises(exporter.GrafanaError):
                exporter.write_dashboard({"dashboard": {"uid": "x"}}, path, overwrite=False)

    def test_discover_dashboard_files_ignores_index_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "index.json").write_text("[]", encoding="utf-8")
            dashboard_path = root / "team" / "dash.json"
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)
            dashboard_path.write_text('{"dashboard": {"uid": "x"}}', encoding="utf-8")

            files = exporter.discover_dashboard_files(root)

            self.assertEqual(files, [dashboard_path])

    def test_discover_dashboard_files_rejects_combined_export_root(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "raw").mkdir()
            (root / "prompt").mkdir()

            with self.assertRaises(exporter.GrafanaError):
                exporter.discover_dashboard_files(root)

    def test_export_dashboards_rejects_disabling_all_variants(self):
        args = exporter.parse_args(["--without-raw", "--without-prompt"])

        with self.assertRaises(exporter.GrafanaError):
            exporter.export_dashboards(args)

    def test_build_import_payload_uses_export_wrapper_and_override(self):
        payload = exporter.build_import_payload(
            document={
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"},
            },
            folder_uid_override="new-folder",
            replace_existing=True,
            message="sync dashboards",
        )

        self.assertEqual(payload["dashboard"]["id"], None)
        self.assertEqual(payload["dashboard"]["uid"], "abc")
        self.assertEqual(payload["folderUid"], "new-folder")
        self.assertTrue(payload["overwrite"])
        self.assertEqual(payload["message"], "sync dashboards")

    def test_build_import_payload_accepts_top_level_dashboard_document(self):
        payload = exporter.build_import_payload(
            document={"id": 7, "uid": "abc", "title": "CPU"},
            folder_uid_override=None,
            replace_existing=False,
            message="sync dashboards",
        )

        self.assertEqual(payload["dashboard"]["id"], None)
        self.assertEqual(payload["dashboard"]["uid"], "abc")
        self.assertEqual(payload["dashboard"]["title"], "CPU")

    def test_build_preserved_web_import_document_keeps_uid_and_title(self):
        document = exporter.build_preserved_web_import_document(
            {
                "dashboard": {
                    "id": 7,
                    "uid": "abc",
                    "title": "CPU",
                    "panels": [],
                }
            }
        )

        self.assertEqual(document["uid"], "abc")
        self.assertEqual(document["title"], "CPU")
        self.assertIsNone(document["id"])
        self.assertNotIn("dashboard", document)

    def test_build_import_payload_rejects_web_import_placeholders(self):
        with self.assertRaises(exporter.GrafanaError):
            exporter.build_import_payload(
                document={
                    "__inputs": [{"name": "DS_PROM"}],
                    "title": "CPU",
                },
                folder_uid_override=None,
                replace_existing=False,
                message="sync dashboards",
            )

    def test_build_external_export_document_adds_datasource_inputs(self):
        payload = {
            "dashboard": {
                "id": 9,
                "title": "Infra",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": {"type": "prometheus", "uid": "prom_uid"},
                        "targets": [
                            {
                                "datasource": {"type": "prometheus", "uid": "prom_uid"},
                                "expr": "up",
                            }
                        ],
                    },
                    {
                        "type": "stat",
                        "datasource": "Loki Logs",
                    },
                ],
            }
        }
        catalog = exporter.build_datasource_catalog(
            [
                {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"},
                {"uid": "loki_uid", "name": "Loki Logs", "type": "loki"},
            ]
        )

        document = exporter.build_external_export_document(payload, catalog)

        self.assertIsNone(document["id"])
        self.assertEqual(
            document["panels"][0]["datasource"]["uid"],
            "${DS_PROMETHEUS_1}",
        )
        self.assertEqual(
            document["panels"][0]["targets"][0]["datasource"]["uid"],
            "${DS_PROMETHEUS_1}",
        )
        self.assertEqual(document["panels"][1]["datasource"], "${DS_LOKI_1}")
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_LOKI_1", "DS_PROMETHEUS_1"],
        )
        self.assertEqual(
            [item["label"] for item in document["__inputs"]],
            ["Loki datasource", "Prometheus datasource"],
        )
        self.assertEqual(
            {item["id"] for item in document["__requires"] if item["type"] == "datasource"},
            {"loki", "prometheus"},
        )
        self.assertEqual(document["__elements"], {})

    def test_build_external_export_document_resolves_string_datasource_uid(self):
        payload = {
            "dashboard": {
                "id": 12,
                "title": "UID ref",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": "dehk4kxat5la8b",
                    }
                ],
            }
        }
        catalog = exporter.build_datasource_catalog(
            [
                {
                    "uid": "dehk4kxat5la8b",
                    "name": "Prod Prometheus",
                    "type": "prometheus",
                }
            ]
        )

        document = exporter.build_external_export_document(payload, catalog)

        self.assertEqual(document["panels"][0]["datasource"]["uid"], "$datasource")
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_PROMETHEUS_1"],
        )
        self.assertEqual(document["templating"]["list"][0]["type"], "datasource")
        self.assertEqual(document["templating"]["list"][0]["query"], "prometheus")

    def test_build_external_export_document_resolves_string_datasource_type_alias(self):
        payload = {
            "dashboard": {
                "id": 13,
                "title": "Type alias",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": "prom",
                    }
                ],
            }
        }

        document = exporter.build_external_export_document(payload, ({}, {}))

        self.assertEqual(document["panels"][0]["datasource"]["uid"], "$datasource")
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_PROMETHEUS_1"],
        )

    def test_build_external_export_document_converts_existing_datasource_variable(self):
        payload = {
            "dashboard": {
                "id": 10,
                "title": "Infra",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": {"type": "prometheus", "uid": "$datasource"},
                    }
                ],
            }
        }

        document = exporter.build_external_export_document(payload, ({}, {}))

        self.assertEqual(document["panels"][0]["datasource"]["uid"], "$datasource")
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_PROMETHEUS_1"],
        )

    def test_build_external_export_document_preserves_untyped_datasource_variable(self):
        payload = {
            "dashboard": {
                "id": 14,
                "title": "Infra",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": {"uid": "$datasource"},
                    }
                ],
            }
        }

        document = exporter.build_external_export_document(payload, ({}, {}))

        self.assertEqual(document["panels"][0]["datasource"]["uid"], "$datasource")
        self.assertEqual(document["__inputs"], [])

    def test_build_external_export_document_creates_input_from_datasource_template_variable(self):
        payload = {
            "dashboard": {
                "id": 15,
                "title": "Prometheus / Overview",
                "templating": {
                    "list": [
                        {
                            "current": {"text": "default", "value": "default"},
                            "hide": 0,
                            "label": "Data source",
                            "name": "datasource",
                            "options": [],
                            "query": "prometheus",
                            "refresh": 1,
                            "regex": "",
                            "type": "datasource",
                        },
                        {
                            "allValue": ".+",
                            "current": {"selected": True, "text": "All", "value": "$__all"},
                            "datasource": "$datasource",
                            "includeAll": True,
                            "label": "job",
                            "multi": True,
                            "name": "job",
                            "options": [],
                            "query": "label_values(prometheus_build_info, job)",
                            "refresh": 1,
                            "regex": "",
                            "sort": 2,
                            "type": "query",
                        },
                    ]
                },
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": "$datasource",
                        "targets": [{"refId": "A", "expr": "up"}],
                    }
                ],
            }
        }

        document = exporter.build_external_export_document(payload, ({}, {}))

        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_PROMETHEUS_1"],
        )
        self.assertEqual(document["templating"]["list"][0]["current"], {})
        self.assertEqual(document["templating"]["list"][0]["query"], "prometheus")
        self.assertEqual(
            document["templating"]["list"][1]["datasource"]["uid"],
            "${DS_PROMETHEUS_1}",
        )
        self.assertEqual(document["panels"][0]["datasource"]["uid"], "$datasource")

    def test_build_external_export_document_keeps_builtin_grafana_datasource_name(self):
        payload = {
            "dashboard": {
                "id": 11,
                "title": "Builtin",
                "panels": [
                    {
                        "type": "timeseries",
                        "datasource": "-- Grafana --",
                    }
                ],
            }
        }

        document = exporter.build_external_export_document(payload, ({}, {}))

        self.assertEqual(document["panels"][0]["datasource"], "-- Grafana --")
        self.assertEqual(document["__inputs"], [])


if __name__ == "__main__":
    unittest.main()
