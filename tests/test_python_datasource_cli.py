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
PARSER_MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "parser.py"
WORKFLOWS_MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "workflows.py"
FIXTURE_PATH = REPO_ROOT / "tests" / "fixtures" / "datasource_contract_cases.json"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
datasource_cli = importlib.import_module("grafana_utils.datasource_cli")


class FakeDatasourceClient(object):
    def __init__(self, datasources=None, org=None, headers=None, org_clients=None):
        self._datasources = list(datasources or [])
        self._org = dict(org or {"id": 1, "name": "Main Org."})
        self.headers = dict(headers or {"Authorization": "Basic test"})
        self._org_clients = dict(org_clients or {})
        self.imported_payloads = []
        self.deleted_paths = []

    def list_datasources(self):
        return list(self._datasources)

    def fetch_current_org(self):
        return dict(self._org)

    def with_org_id(self, org_id):
        key = str(org_id)
        if key not in self._org_clients:
            raise AssertionError("Unexpected org id %s" % key)
        return self._org_clients[key]

    def request_json(self, path, params=None, method="GET", payload=None):
        if path == "/api/datasources" and method == "GET":
            return list(self._datasources)
        if path == "/api/org":
            return dict(self._org)
        if method in ("POST", "PUT"):
            self.imported_payloads.append(
                {
                    "path": path,
                    "method": method,
                    "params": dict(params or {}),
                    "payload": payload,
                }
            )
            return {"status": "success"}
        if method == "DELETE":
            self.deleted_paths.append(path)
            return {"status": "success"}
        raise AssertionError("Unexpected datasource request %s %s" % (method, path))


class DatasourceCliTests(unittest.TestCase):
    def _load_contract_cases(self):
        return json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))

    def test_datasource_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_datasource_parser_module_parses_as_python39_syntax(self):
        source = PARSER_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(PARSER_MODULE_PATH), feature_version=(3, 9))

    def test_datasource_workflows_module_parses_as_python39_syntax(self):
        source = WORKFLOWS_MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(WORKFLOWS_MODULE_PATH), feature_version=(3, 9))

    def test_parse_args_supports_list_mode(self):
        args = datasource_cli.parse_args(["list", "--json"])

        self.assertEqual(args.command, "list")
        self.assertTrue(args.json)
        self.assertFalse(args.csv)
        self.assertFalse(args.table)
        self.assertFalse(args.no_header)

    def test_parse_args_supports_list_output_format(self):
        args = datasource_cli.parse_args(["list", "--output-format", "csv"])

        self.assertEqual(args.output_format, "csv")
        self.assertTrue(args.csv)
        self.assertFalse(args.table)
        self.assertFalse(args.json)

    def test_parse_args_supports_export_mode(self):
        args = datasource_cli.parse_args(["export", "--export-dir", "./datasources", "--overwrite"])

        self.assertEqual(args.command, "export")
        self.assertEqual(args.export_dir, "./datasources")
        self.assertTrue(args.overwrite)
        self.assertFalse(args.dry_run)

    def test_parse_args_supports_import_mode(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "./datasources",
                "--replace-existing",
                "--dry-run",
                "--table",
            ]
        )

        self.assertEqual(args.command, "import")
        self.assertEqual(args.import_dir, "./datasources")
        self.assertTrue(args.replace_existing)
        self.assertTrue(args.dry_run)
        self.assertTrue(args.table)

    def test_parse_args_supports_add_mode(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--datasource-url",
                "http://prometheus:9090",
                "--dry-run",
                "--table",
            ]
        )

        self.assertEqual(args.command, "add")
        self.assertEqual(args.name, "Prometheus Main")
        self.assertEqual(args.type, "prometheus")
        self.assertEqual(args.datasource_url, "http://prometheus:9090")
        self.assertTrue(args.dry_run)
        self.assertTrue(args.table)

    def test_parse_args_supports_add_auth_and_header_flags(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--basic-auth",
                "--basic-auth-user",
                "metrics-user",
                "--basic-auth-password",
                "metrics-pass",
                "--user",
                "query-user",
                "--password",
                "query-pass",
                "--with-credentials",
                "--http-header",
                "X-Scope-OrgID=tenant-a",
                "--http-header",
                "X-Trace=enabled",
                "--tls-skip-verify",
                "--server-name",
                "prometheus.internal",
            ]
        )

        self.assertTrue(args.basic_auth)
        self.assertEqual(args.basic_auth_user, "metrics-user")
        self.assertEqual(args.basic_auth_password, "metrics-pass")
        self.assertEqual(args.user, "query-user")
        self.assertEqual(args.password, "query-pass")
        self.assertTrue(args.with_credentials)
        self.assertEqual(
            args.http_header,
            ["X-Scope-OrgID=tenant-a", "X-Trace=enabled"],
        )
        self.assertTrue(args.tls_skip_verify)
        self.assertEqual(args.server_name, "prometheus.internal")

    def test_parse_args_supports_delete_mode(self):
        args = datasource_cli.parse_args(
            ["delete", "--uid", "prom-main", "--dry-run", "--output-format", "json"]
        )

        self.assertEqual(args.command, "delete")
        self.assertEqual(args.uid, "prom-main")
        self.assertTrue(args.dry_run)
        self.assertTrue(args.json)

    def test_parse_args_supports_import_output_format(self):
        args = datasource_cli.parse_args(
            ["import", "--import-dir", "./datasources", "--dry-run", "--output-format", "json"]
        )

        self.assertEqual(args.output_format, "json")
        self.assertTrue(args.json)
        self.assertFalse(args.table)

    def test_parse_args_supports_import_output_columns(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "./datasources",
                "--dry-run",
                "--table",
                "--output-columns",
                "uid,action,org_id,file",
            ]
        )

        self.assertEqual(args.output_columns, ["uid", "action", "orgId", "file"])

    def test_parse_args_supports_import_org_and_export_org_guard(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "./datasources",
                "--org-id",
                "7",
                "--require-matching-export-org",
            ]
        )

        self.assertEqual(args.org_id, "7")
        self.assertTrue(args.require_matching_export_org)

    def test_parse_args_supports_diff_mode(self):
        args = datasource_cli.parse_args(
            ["diff", "--diff-dir", "./datasources", "--url", "http://127.0.0.1:3000"]
        )

        self.assertEqual(args.command, "diff")
        self.assertEqual(args.diff_dir, "./datasources")

    def test_parse_args_rejects_multiple_list_output_modes(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--table", "--json"])

        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--csv", "--json"])

    def test_parse_args_rejects_output_format_with_legacy_list_flags(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(["list", "--output-format", "table", "--json"])

    def test_parse_args_rejects_output_format_with_legacy_import_flags(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(
                ["import", "--import-dir", "./datasources", "--output-format", "table", "--json"]
            )

    def test_parse_args_rejects_import_output_columns_without_table_output(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(
                [
                    "import",
                    "--import-dir",
                    "./datasources",
                    "--dry-run",
                    "--output-columns",
                    "uid,action",
                ]
            )

    def test_parse_args_rejects_live_mutation_output_format_with_legacy_flags(self):
        with self.assertRaises(SystemExit):
            datasource_cli.parse_args(
                [
                    "add",
                    "--name",
                    "Prometheus Main",
                    "--type",
                    "prometheus",
                    "--output-format",
                    "table",
                    "--json",
                ]
            )

    def test_import_help_mentions_dry_run_and_org_guard_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                datasource_cli.parse_args(["import", "-h"])

        help_text = stream.getvalue()
        self.assertIn("--import-dir", help_text)
        self.assertIn("--org-id", help_text)
        self.assertIn("--require-matching-export-org", help_text)
        self.assertIn("--replace-existing", help_text)
        self.assertIn("--update-existing-only", help_text)
        self.assertIn("--dry-run", help_text)
        self.assertIn("--table", help_text)
        self.assertIn("--json", help_text)
        self.assertIn("--output-format", help_text)
        self.assertIn("--output-columns", help_text)
        self.assertIn("--progress", help_text)
        self.assertIn("--verbose", help_text)

    def test_add_help_mentions_live_mutation_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                datasource_cli.parse_args(["add", "-h"])

        help_text = stream.getvalue()
        self.assertIn("--name", help_text)
        self.assertIn("--type", help_text)
        self.assertIn("--datasource-url", help_text)
        self.assertIn("--basic-auth", help_text)
        self.assertIn("--basic-auth-user", help_text)
        self.assertIn("--basic-auth-password", help_text)
        self.assertIn("--user", help_text)
        self.assertIn("--password", help_text)
        self.assertIn("--with-credentials", help_text)
        self.assertIn("--http-header", help_text)
        self.assertIn("--tls-skip-verify", help_text)
        self.assertIn("--server-name", help_text)
        self.assertIn("--json-data", help_text)
        self.assertIn("--secure-json-data", help_text)
        self.assertIn("--dry-run", help_text)

    def test_delete_help_mentions_live_mutation_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                datasource_cli.parse_args(["delete", "-h"])

        help_text = stream.getvalue()
        self.assertIn("--uid", help_text)
        self.assertIn("--name", help_text)
        self.assertIn("--dry-run", help_text)

    def test_diff_help_mentions_diff_dir(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                datasource_cli.parse_args(["diff", "-h"])

        help_text = stream.getvalue()
        self.assertIn("--diff-dir", help_text)

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

    def test_add_datasource_dry_run_renders_table(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--uid",
                "prom-main",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--dry-run",
                "--table",
                "--datasource-url",
                "http://prometheus:9090",
            ]
        )
        client = FakeDatasourceClient(datasources=[])

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.add_datasource(args)

        self.assertEqual(result, 0)
        output = stdout.getvalue()
        self.assertIn("OPERATION", output)
        self.assertIn("would-create", output)
        self.assertEqual(client.imported_payloads, [])

    def test_add_datasource_live_posts_payload(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--uid",
                "prom-main",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--datasource-url",
                "http://prometheus:9090",
                "--json-data",
                "{\"httpMethod\": \"POST\"}",
            ]
        )
        client = FakeDatasourceClient(datasources=[])

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.add_datasource(args)

        self.assertEqual(result, 0)
        self.assertIn("Created datasource uid=prom-main name=Prometheus Main", stdout.getvalue())
        self.assertEqual(client.imported_payloads[0]["method"], "POST")
        self.assertEqual(client.imported_payloads[0]["payload"]["jsonData"], {"httpMethod": "POST"})

    def test_add_datasource_live_posts_common_auth_and_header_fields(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--uid",
                "prom-main",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--datasource-url",
                "http://prometheus:9090",
                "--basic-auth-user",
                "metrics-user",
                "--basic-auth-password",
                "metrics-pass",
                "--user",
                "query-user",
                "--password",
                "query-pass",
                "--with-credentials",
                "--http-header",
                "X-Scope-OrgID=tenant-a",
                "--http-header",
                "X-Trace=enabled",
                "--tls-skip-verify",
                "--server-name",
                "prometheus.internal",
            ]
        )
        client = FakeDatasourceClient(datasources=[])

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.add_datasource(args)

        self.assertEqual(result, 0)
        payload = client.imported_payloads[0]["payload"]
        self.assertTrue(payload["basicAuth"])
        self.assertEqual(payload["basicAuthUser"], "metrics-user")
        self.assertEqual(payload["user"], "query-user")
        self.assertTrue(payload["withCredentials"])
        self.assertEqual(
            payload["jsonData"],
            {
                "tlsSkipVerify": True,
                "serverName": "prometheus.internal",
                "httpHeaderName1": "X-Scope-OrgID",
                "httpHeaderName2": "X-Trace",
            },
        )
        self.assertEqual(
            payload["secureJsonData"],
            {
                "basicAuthPassword": "metrics-pass",
                "password": "query-pass",
                "httpHeaderValue1": "tenant-a",
                "httpHeaderValue2": "enabled",
            },
        )

    def test_add_datasource_merges_inline_json_with_common_auth_and_header_fields(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--json-data",
                "{\"httpMethod\": \"POST\"}",
                "--secure-json-data",
                "{\"token\": \"abc123\"}",
                "--http-header",
                "X-Scope-OrgID=tenant-a",
            ]
        )
        client = FakeDatasourceClient(datasources=[])

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            datasource_cli.add_datasource(args)

        payload = client.imported_payloads[0]["payload"]
        self.assertEqual(
            payload["jsonData"],
            {
                "httpMethod": "POST",
                "httpHeaderName1": "X-Scope-OrgID",
            },
        )
        self.assertEqual(
            payload["secureJsonData"],
            {
                "token": "abc123",
                "httpHeaderValue1": "tenant-a",
            },
        )

    def test_delete_datasource_dry_run_renders_json(self):
        args = datasource_cli.parse_args(
            ["delete", "--uid", "prom-main", "--dry-run", "--json"]
        )
        client = FakeDatasourceClient(
            datasources=[{"id": 7, "uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}]
        )

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.delete_datasource(args)

        self.assertEqual(result, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["summary"]["deleteCount"], 1)
        self.assertEqual(client.deleted_paths, [])

    def test_delete_datasource_live_calls_delete_endpoint(self):
        args = datasource_cli.parse_args(["delete", "--uid", "prom-main"])
        client = FakeDatasourceClient(
            datasources=[{"id": 7, "uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}]
        )

        with mock.patch.object(datasource_cli, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = datasource_cli.delete_datasource(args)

        self.assertEqual(result, 0)
        self.assertIn("Deleted datasource uid=prom-main name=Prometheus Main id=7", stdout.getvalue())
        self.assertEqual(client.deleted_paths, ["/api/datasources/7"])

    def test_add_datasource_rejects_invalid_json_data(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--json-data",
                "[]",
            ]
        )

        with self.assertRaisesRegex(datasource_cli.GrafanaError, "must decode to a JSON object"):
            datasource_cli.add_datasource(args)

    def test_add_datasource_rejects_basic_auth_password_without_user(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--basic-auth-password",
                "metrics-pass",
            ]
        )

        with self.assertRaisesRegex(datasource_cli.GrafanaError, "requires --basic-auth-user"):
            datasource_cli.add_datasource(args)

    def test_add_datasource_rejects_invalid_http_header_format(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--http-header",
                "missing-separator",
            ]
        )

        with self.assertRaisesRegex(datasource_cli.GrafanaError, "requires NAME=VALUE"):
            datasource_cli.add_datasource(args)

    def test_add_datasource_rejects_json_data_key_conflicts_with_header_flags(self):
        args = datasource_cli.parse_args(
            [
                "add",
                "--name",
                "Prometheus Main",
                "--type",
                "prometheus",
                "--json-data",
                "{\"httpHeaderName1\": \"X-Existing\"}",
                "--http-header",
                "X-Scope-OrgID=tenant-a",
            ]
        )

        with self.assertRaisesRegex(datasource_cli.GrafanaError, "would overwrite existing key"):
            datasource_cli.add_datasource(args)

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

    def test_normalize_datasource_record_matches_shared_contract_fixtures(self):
        for case in self._load_contract_cases():
            with self.subTest(case=case["name"]):
                self.assertEqual(
                    datasource_cli.normalize_datasource_record(case["rawDatasource"]),
                    case["expectedNormalizedRecord"],
                )

    def test_build_import_payload_matches_shared_contract_fixtures(self):
        for case in self._load_contract_cases():
            with self.subTest(case=case["name"]):
                self.assertEqual(
                    datasource_cli.build_import_payload(case["expectedNormalizedRecord"]),
                    case["expectedImportPayload"],
                )

    def test_load_import_bundle_rejects_extra_secret_or_server_managed_fields(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            (import_dir / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    }
                ),
                encoding="utf-8",
            )
            (import_dir / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
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
                            "id": 7,
                            "jsonData": {"httpMethod": "POST"},
                            "secureJsonData": {"httpHeaderValue1": "secret"},
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (import_dir / "index.json").write_text("{}", encoding="utf-8")

            with self.assertRaisesRegex(
                datasource_cli.GrafanaError,
                "unsupported datasource field\\(s\\): id, jsonData, secureJsonData",
            ):
                datasource_cli.load_import_bundle(import_dir)

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

    def test_import_datasources_rejects_export_org_mismatch_for_token_scope(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "ignored",
                "--dry-run",
                "--require-matching-export-org",
            ]
        )
        client = FakeDatasourceClient(
            datasources=[],
            org={"id": 2, "name": "Ops Org"},
            headers={"Authorization": "Bearer token"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom_uid",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Main Org.",
                            "orgId": "1",
                        }
                    ],
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "count": 1,
                        "items": [
                            {
                                "uid": "prom_uid",
                                "name": "Prometheus Main",
                                "type": "prometheus",
                                "org": "Main Org.",
                                "orgId": "1",
                            }
                        ],
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    datasource_cli.GrafanaError,
                    "Raw export orgId 1 does not match target Grafana org id 2",
                ):
                    datasource_cli.import_datasources(args)

    def test_import_datasources_dry_run_uses_org_scoped_client(self):
        scoped_client = FakeDatasourceClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ],
            org={"id": 7, "name": "Observability"},
            headers={"Authorization": "Basic scoped"},
        )
        client = FakeDatasourceClient(
            datasources=[],
            headers={"Authorization": "Basic root"},
            org_clients={"7": scoped_client},
        )
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "ignored",
                "--org-id",
                "7",
                "--dry-run",
                "--verbose",
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom_uid",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Observability",
                            "orgId": "7",
                        }
                    ],
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "count": 1,
                        "items": [
                            {
                                "uid": "prom_uid",
                                "name": "Prometheus Main",
                                "type": "prometheus",
                                "org": "Observability",
                                "orgId": "7",
                            }
                        ],
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.import_datasources(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.imported_payloads, [])
            self.assertEqual(scoped_client.imported_payloads, [])
            self.assertIn("Import mode: create-only", stdout.getvalue())

    def test_import_datasources_rejects_name_match_with_different_uid(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "ignored",
                "--replace-existing",
            ]
        )
        client = FakeDatasourceClient(
            datasources=[
                {
                    "id": 9,
                    "uid": "prom-live",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    }
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom-export",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Main Org.",
                            "orgId": "1",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text("{}", encoding="utf-8")

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    datasource_cli.GrafanaError,
                    "action=would-fail-uid-mismatch",
                ):
                    datasource_cli.import_datasources(args)

    def test_diff_datasources_returns_zero_when_inventory_matches(self):
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
        args = datasource_cli.parse_args(
            ["diff", "--diff-dir", "ignored", "--url", "http://127.0.0.1:3000"]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.diff_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    }
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom_uid",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Main Org.",
                            "orgId": "1",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text("{}", encoding="utf-8")

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.diff_datasources(args)

        self.assertEqual(result, 0)
        self.assertIn("Diff same", stdout.getvalue())
        self.assertIn("No datasource differences across 1 exported datasource(s).", stdout.getvalue())

    def test_diff_datasources_returns_one_when_inventory_differs(self):
        client = FakeDatasourceClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus-alt:9090",
                    "isDefault": True,
                }
            ]
        )
        args = datasource_cli.parse_args(
            ["diff", "--diff-dir", "ignored", "--url", "http://127.0.0.1:3000"]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.diff_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    }
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom_uid",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Main Org.",
                            "orgId": "1",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text("{}", encoding="utf-8")

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.diff_datasources(args)

        self.assertEqual(result, 1)
        self.assertIn("Diff different", stdout.getvalue())
        self.assertIn("--- remote/prom_uid", stdout.getvalue())
        self.assertIn("+++ local/prom_uid", stdout.getvalue())
        self.assertIn("Found 1 datasource difference(s) across 1 exported datasource(s).", stdout.getvalue())

    def test_import_datasources_dry_run_table_output_columns_limits_rendered_fields(self):
        args = datasource_cli.parse_args(
            [
                "import",
                "--import-dir",
                "./datasources",
                "--dry-run",
                "--table",
                "--output-columns",
                "uid,action,file",
            ]
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
            args.import_dir = tmpdir
            (Path(tmpdir) / datasource_cli.EXPORT_METADATA_FILENAME).write_text(
                json.dumps(
                    {
                        "kind": datasource_cli.ROOT_INDEX_KIND,
                        "schemaVersion": datasource_cli.TOOL_SCHEMA_VERSION,
                        "variant": "root",
                        "resource": "datasource",
                        "datasourceCount": 1,
                        "datasourcesFile": datasource_cli.DATASOURCE_EXPORT_FILENAME,
                        "indexFile": "index.json",
                        "format": "grafana-datasource-inventory-v1",
                    }
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / datasource_cli.DATASOURCE_EXPORT_FILENAME).write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom_uid",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": "true",
                            "org": "Main Org.",
                            "orgId": "1",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (Path(tmpdir) / "index.json").write_text("{}", encoding="utf-8")

            with mock.patch.object(datasource_cli, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = datasource_cli.import_datasources(args)

        self.assertEqual(result, 0)
        output = stdout.getvalue()
        self.assertIn("UID", output)
        self.assertIn("ACTION", output)
        self.assertIn("FILE", output)
        self.assertNotIn("NAME", output)
        self.assertNotIn("TYPE", output)
        self.assertNotIn("ORG_ID", output)


if __name__ == "__main__":
    unittest.main()
