import ast
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
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource_cli.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
datasource_cli = importlib.import_module("grafana_utils.datasource_cli")


class FakeDatasourceClient(object):
    def __init__(self, datasources=None, org=None):
        self._datasources = list(datasources or [])
        self._org = dict(org or {"id": 1, "name": "Main Org."})

    def list_datasources(self):
        return list(self._datasources)

    def fetch_current_org(self):
        return dict(self._org)


class DatasourceCliTests(unittest.TestCase):
    def test_datasource_module_parses_as_python36_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 6))

    def test_parse_args_supports_list_mode(self):
        args = datasource_cli.parse_args(["list", "--json"])

        self.assertEqual(args.command, "list")
        self.assertTrue(args.json)
        self.assertFalse(args.csv)
        self.assertFalse(args.table)
        self.assertFalse(args.no_header)

    def test_parse_args_supports_export_mode(self):
        args = datasource_cli.parse_args(["export", "--export-dir", "./datasources", "--overwrite"])

        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "./datasources")
        self.assertTrue(args.overwrite)
        self.assertFalse(args.dry_run)

    def test_parse_args_rejects_multiple_list_output_modes(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--table", "--json"])

        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--csv", "--json"])

    def test_list_datasources_prints_table_by_default(self):
        args = datasource_cli.parse_args(["list", "--url", "http://127.0.0.1:3000"])
        client = FakeDatasourceClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ]
        )

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.list_datasources(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "UID       NAME             TYPE        URL                     IS_DEFAULT",
                "--------  ---------------  ----------  ----------------------  ----------",
                "prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      ",
                "",
                "Listed 1 data source(s) from http://127.0.0.1:3000",
            ],
        )

    def test_export_datasources_writes_normalized_files(self):
        args = datasource_cli.parse_args(
            ["export", "--export-dir", "ignored", "--url", "http://127.0.0.1:3000"]
        )
        client = FakeDatasourceClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                },
                {
                    "uid": "loki_uid",
                    "name": "Loki Logs",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": False,
                },
            ],
            org={"id": 2, "name": "Observability"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.export_dir = tmpdir
            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.export_datasources(args)

            self.assertEqual(result, 0)
            self.assertIn("Exported 2 datasource(s).", stdout.getvalue())

            datasources_document = json.loads(
                (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).read_text(
                    encoding="utf-8"
                )
            )
            self.assertEqual(
                datasources_document,
                [
                    {
                        "uid": "prom_uid",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": "true",
                        "org": "Observability",
                        "orgId": "2",
                    },
                    {
                        "uid": "loki_uid",
                        "name": "Loki Logs",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki:3100",
                        "isDefault": "false",
                        "org": "Observability",
                        "orgId": "2",
                    },
                ],
            )

            index_document = json.loads(
                (Path(tmpdir) / "index.json").read_text(encoding="utf-8")
            )
            self.assertEqual(index_document["kind"], datasource_cli.ROOT_INDEX_KIND)
            self.assertEqual(index_document["schemaVersion"], datasource_cli.TOOL_SCHEMA_VERSION)
            self.assertEqual(index_document["datasourcesFile"], datasource_cli.DATASOURCE_EXPORT_FILENAME)
            self.assertEqual(index_document["count"], 2)

            metadata_document = json.loads(
                (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).read_text(
                    encoding="utf-8"
                )
            )
            self.assertEqual(metadata_document["resource"], "datasource")
            self.assertEqual(metadata_document["datasourceCount"], 2)
            self.assertEqual(
                metadata_document["datasourcesFile"],
                datasource_cli.DATASOURCE_EXPORT_FILENAME,
            )

    def test_export_datasources_dry_run_does_not_write_files(self):
        args = datasource_cli.parse_args(
            ["export", "--export-dir", "ignored", "--dry-run", "--url", "http://127.0.0.1:3000"]
        )
        client = FakeDatasourceClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.export_dir = tmpdir
            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.export_datasources(args)

            self.assertEqual(result, 0)
            self.assertIn("Would export 1 datasource(s).", stdout.getvalue())
            self.assertFalse((Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).exists())
            self.assertFalse((Path(tmpdir) / "index.json").exists())
            self.assertFalse((Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).exists())


if __name__ == "__main__":
    unittest.main()
