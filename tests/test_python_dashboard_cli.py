import argparse
import ast
import base64
import io
import importlib
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboard_cli.py"
TRANSPORT_MODULE_PATH = REPO_ROOT / "grafana_utils" / "http_transport.py"
CLIENT_MODULE_PATH = REPO_ROOT / "grafana_utils" / "clients" / "dashboard_client.py"
TRANSFORMER_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "transformer.py"
WRAPPER_PATH = REPO_ROOT / "python" / "grafana-utils.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
transport_module = importlib.import_module("grafana_utils.http_transport")
exporter = importlib.import_module("grafana_utils.dashboard_cli")


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


class FakeDashboardWorkflowClient:
    def __init__(
        self,
        summaries=None,
        dashboards=None,
        datasources=None,
        folders=None,
        org=None,
        orgs=None,
        org_clients=None,
        headers=None,
    ):
        self.summaries = summaries or []
        self.dashboards = dashboards or {}
        self.datasources = datasources or []
        self.folders = folders or {}
        self.org = org or {"id": 1, "name": "Main Org."}
        self.orgs = orgs or [self.org]
        self.org_clients = org_clients or {}
        self.headers = headers or {"Authorization": "Basic test"}
        self.imported_payloads = []
        self.created_folders = []

    def iter_dashboard_summaries(self, page_size):
        return list(self.summaries)

    def fetch_dashboard(self, uid):
        if uid not in self.dashboards:
            raise exporter.GrafanaApiError(404, f"/api/dashboards/uid/{uid}", "not found")
        return self.dashboards[uid]

    def fetch_dashboard_if_exists(self, uid):
        return self.dashboards.get(uid)

    def fetch_folder_if_exists(self, uid):
        return self.folders.get(uid)

    def create_folder(self, uid, title, parent_uid=None):
        record = {"uid": uid, "title": title}
        if parent_uid:
            record["parentUid"] = parent_uid
        self.created_folders.append(record)
        self.folders[uid] = dict(record)
        return {"status": "success", "uid": uid, "title": title}

    def list_datasources(self):
        return list(self.datasources)

    def fetch_current_org(self):
        return dict(self.org)

    def list_orgs(self):
        return list(self.orgs)

    def with_org_id(self, org_id):
        key = str(org_id)
        if key not in self.org_clients:
            raise AssertionError("Unexpected org id %s" % key)
        return self.org_clients[key]

    def import_dashboard(self, payload):
        self.imported_payloads.append(payload)
        return {"status": "success", "uid": payload["dashboard"].get("uid")}


class ExporterTests(unittest.TestCase):
    def test_dashboard_script_parses_as_python36_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 6))

    def test_transport_module_parses_as_python36_syntax(self):
        source = TRANSPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(TRANSPORT_MODULE_PATH), feature_version=(3, 6))

    def test_dashboard_client_module_parses_as_python36_syntax(self):
        source = CLIENT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(CLIENT_MODULE_PATH), feature_version=(3, 6))

    def test_dashboard_transformer_module_parses_as_python36_syntax(self):
        source = TRANSFORMER_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(TRANSFORMER_MODULE_PATH), feature_version=(3, 6))

    def test_dashboard_wrapper_script_parses_as_python36_syntax(self):
        source = WRAPPER_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(WRAPPER_PATH), feature_version=(3, 6))

    def test_parse_args_requires_subcommand(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args([])

    def test_top_level_help_includes_basic_and_token_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["-h"])

        help_text = stream.getvalue()
        self.assertIn("Export dashboards from local Grafana with Basic auth", help_text)
        self.assertIn("Export dashboards with an API token", help_text)
        self.assertIn("http://localhost:3000", help_text)

    def test_export_help_includes_basic_and_token_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["export-dashboard", "-h"])

        help_text = stream.getvalue()
        self.assertIn("Export dashboards from local Grafana with Basic auth", help_text)
        self.assertIn("Export dashboards with an API token", help_text)
        self.assertIn("--basic-user admin --basic-password admin", help_text)

    def test_import_help_explains_common_operator_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["import-dashboard", "-h"])

        help_text = stream.getvalue()
        self.assertIn("combined", help_text)
        self.assertIn("export root", help_text)
        self.assertIn("missing/match/mismatch", help_text)
        self.assertIn("skipped/blocked", help_text)
        self.assertIn("table form", help_text)

    def test_inspect_export_help_mentions_raw_export_directory(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["inspect-export", "-h"])

        help_text = stream.getvalue()
        self.assertIn("raw/ export directory explicitly", help_text)
        self.assertIn("--json", help_text)
        self.assertIn("--table", help_text)


    def test_parse_args_supports_import_mode(self):
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards"])

        self.assertEqual(args.import_dir, "dashboards")
        self.assertEqual(args.command, "import-dashboard")

    def test_parse_args_supports_preferred_auth_aliases(self):
        args = exporter.parse_args(
            [
                "export-dashboard",
                "--token",
                "abc123",
                "--basic-user",
                "user",
                "--basic-password",
                "pass",
            ]
        )

        self.assertEqual(args.api_token, "abc123")
        self.assertEqual(args.username, "user")
        self.assertEqual(args.password, "pass")
        self.assertFalse(args.prompt_password)

    def test_parse_args_supports_prompt_password(self):
        args = exporter.parse_args(
            [
                "export-dashboard",
                "--basic-user",
                "user",
                "--prompt-password",
            ]
        )

        self.assertEqual(args.username, "user")
        self.assertIsNone(args.password)
        self.assertTrue(args.prompt_password)

    def test_parse_args_supports_list_mode(self):
        args = exporter.parse_args(["list-dashboard", "--page-size", "25"])

        self.assertEqual(args.command, "list-dashboard")
        self.assertEqual(args.page_size, 25)
        self.assertFalse(args.table)
        self.assertFalse(args.with_sources)
        self.assertFalse(args.csv)
        self.assertFalse(args.json)
        self.assertFalse(args.no_header)
        self.assertIsNone(args.org_id)
        self.assertFalse(args.all_orgs)

    def test_parse_args_supports_list_org_selection(self):
        org_args = exporter.parse_args(["list-dashboard", "--org-id", "2"])
        all_args = exporter.parse_args(["list-dashboard", "--all-orgs"])

        self.assertEqual(org_args.org_id, "2")
        self.assertFalse(org_args.all_orgs)
        self.assertTrue(all_args.all_orgs)
        self.assertIsNone(all_args.org_id)

    def test_parse_args_supports_list_data_sources_mode(self):
        args = exporter.parse_args(["list-data-sources"])

        self.assertEqual(args.command, "list-data-sources")
        self.assertFalse(args.table)
        self.assertFalse(args.csv)
        self.assertFalse(args.json)
        self.assertFalse(args.no_header)

    def test_parse_args_supports_list_csv_and_json_modes(self):
        csv_args = exporter.parse_args(["list-dashboard", "--csv"])
        json_args = exporter.parse_args(["list-dashboard", "--json"])
        source_args = exporter.parse_args(["list-dashboard", "--with-sources"])
        no_header_args = exporter.parse_args(["list-dashboard", "--no-header"])

        self.assertTrue(csv_args.csv)
        self.assertFalse(csv_args.table)
        self.assertFalse(csv_args.json)
        self.assertTrue(json_args.json)
        self.assertFalse(json_args.table)
        self.assertFalse(json_args.csv)
        self.assertTrue(source_args.with_sources)
        self.assertTrue(no_header_args.no_header)

    def test_parse_args_supports_export_and_import_progress(self):
        export_args = exporter.parse_args(["export-dashboard", "--progress"])
        import_args = exporter.parse_args(["import-dashboard", "--import-dir", "./dashboards/raw", "--progress"])
        verbose_export_args = exporter.parse_args(["export-dashboard", "--verbose"])
        verbose_import_args = exporter.parse_args(["import-dashboard", "--import-dir", "./dashboards/raw", "--verbose"])

        self.assertTrue(export_args.progress)
        self.assertTrue(import_args.progress)
        self.assertTrue(verbose_export_args.verbose)
        self.assertTrue(verbose_import_args.verbose)

    def test_parse_args_rejects_multiple_list_output_modes(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--csv", "--json"])

    def test_parse_args_rejects_multiple_list_data_sources_output_modes(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--csv", "--json"])

    def test_parse_args_supports_diff_mode(self):
        args = exporter.parse_args(["diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(args.import_dir, "dashboards/raw")
        self.assertEqual(args.command, "diff")
        self.assertEqual(args.context_lines, 3)

    def test_parse_args_defaults_export_dir_to_dashboards(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.export_dir, "dashboards")
        self.assertEqual(args.command, "export-dashboard")
        self.assertIsNone(args.org_id)
        self.assertFalse(args.all_orgs)

    def test_parse_args_supports_export_org_selection(self):
        org_args = exporter.parse_args(["export-dashboard", "--org-id", "2"])
        all_args = exporter.parse_args(["export-dashboard", "--all-orgs"])

        self.assertEqual(org_args.org_id, "2")
        self.assertFalse(org_args.all_orgs)
        self.assertTrue(all_args.all_orgs)
        self.assertIsNone(all_args.org_id)

    def test_parse_args_defaults_url_to_local_grafana(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.url, "http://localhost:3000")

    def test_parse_args_supports_variant_switches(self):
        args = exporter.parse_args(
            ["export-dashboard", "--without-dashboard-raw", "--without-dashboard-prompt"]
        )

        self.assertTrue(args.without_dashboard_raw)
        self.assertTrue(args.without_dashboard_prompt)

    def test_parse_args_supports_export_dry_run(self):
        args = exporter.parse_args(["export-dashboard", "--dry-run"])

        self.assertTrue(args.dry_run)

    def test_parse_args_supports_import_dry_run(self):
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run"])

        self.assertTrue(args.dry_run)

    def test_parse_args_supports_import_dry_run_table_flags(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--table", "--no-header"]
        )

        self.assertTrue(args.dry_run)
        self.assertTrue(args.table)
        self.assertTrue(args.no_header)

    def test_parse_args_supports_import_dry_run_json(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--json"]
        )

        self.assertTrue(args.dry_run)
        self.assertTrue(args.json)

    def test_parse_args_supports_update_existing_only(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--update-existing-only"]
        )

        self.assertTrue(args.update_existing_only)

    def test_parse_args_supports_inspect_export_json(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--json"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertTrue(args.json)

    def test_parse_args_supports_inspect_export_table(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--table", "--no-header"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertTrue(args.table)
        self.assertTrue(args.no_header)

    def test_parse_args_supports_ensure_folders(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--ensure-folders"]
        )

        self.assertTrue(args.ensure_folders)

    def test_describe_dashboard_import_mode(self):
        self.assertEqual(
            exporter.describe_dashboard_import_mode(False, False),
            "create-only",
        )
        self.assertEqual(
            exporter.describe_dashboard_import_mode(True, False),
            "create-or-update",
        )
        self.assertEqual(
            exporter.describe_dashboard_import_mode(False, True),
            "update-or-skip-missing",
        )

    def test_parse_args_disables_ssl_verification_by_default(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertFalse(args.verify_ssl)

    def test_parse_args_can_enable_ssl_verification(self):
        args = exporter.parse_args(["export-dashboard", "--verify-ssl"])

        self.assertTrue(args.verify_ssl)

    def test_parse_args_rejects_old_list_subcommand_name(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list", "--json"])

    def test_build_json_http_transport_defaults_to_requests(self):
        transport = exporter.build_json_http_transport(
            base_url="http://127.0.0.1:3000",
            headers={},
            timeout=30,
            verify_ssl=False,
        )

        expected = (
            "HttpxJsonHttpTransport"
            if transport_module.httpx_is_available() and transport_module.http2_is_available()
            else "RequestsJsonHttpTransport"
        )
        self.assertEqual(type(transport).__name__, expected)

    def test_build_json_http_transport_supports_httpx(self):
        transport = exporter.build_json_http_transport(
            base_url="http://127.0.0.1:3000",
            headers={},
            timeout=30,
            verify_ssl=False,
            transport_name="httpx",
        )

        self.assertEqual(type(transport).__name__, "HttpxJsonHttpTransport")

    def test_http2_capability_helper_returns_boolean(self):
        self.assertIsInstance(transport_module.http2_is_available(), bool)

    def test_client_accepts_injected_transport(self):
        class FakeTransport:
            def request_json(self, path, params=None, method="GET", payload=None):
                return {"dashboard": {"uid": "abc"}}

        client = exporter.GrafanaClient(
            base_url="http://127.0.0.1:3000",
            headers={},
            timeout=30,
            verify_ssl=False,
            transport=FakeTransport(),
        )

        result = client.fetch_dashboard("abc")

        self.assertEqual(result["dashboard"]["uid"], "abc")

    def test_resolve_auth_supports_token_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            username=None,
            password=None,
        )

        headers = exporter.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer abc123")

    def test_resolve_auth_supports_basic_auth(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password="pass",
            prompt_password=False,
        )

        headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:pass").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")

    def test_resolve_auth_rejects_mixed_token_and_basic_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="user",
            password="pass",
            prompt_password=False,
        )

        with self.assertRaisesRegex(exporter.GrafanaError, "Choose either token auth"):
            exporter.resolve_auth(args)

    def test_resolve_auth_rejects_user_without_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password=None,
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user / --username and --basic-password / --password or --prompt-password.",
        ):
            exporter.resolve_auth(args)

    def test_resolve_auth_rejects_password_without_user(self):
        args = argparse.Namespace(
            api_token=None,
            username=None,
            password="pass",
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user / --username and --basic-password / --password or --prompt-password.",
        ):
            exporter.resolve_auth(args)

    def test_resolve_auth_supports_prompt_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password=None,
            prompt_password=True,
        )

        with mock.patch("grafana_utils.dashboard_cli.getpass.getpass", return_value="secret") as prompt:
            headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:secret").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")
        prompt.assert_called_once_with("Grafana Basic auth password: ")

    def test_resolve_auth_rejects_prompt_without_username(self):
        args = argparse.Namespace(
            api_token=None,
            username=None,
            password=None,
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "--prompt-password requires --basic-user / --username.",
        ):
            exporter.resolve_auth(args)

    def test_resolve_auth_rejects_prompt_with_explicit_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password="pass",
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Choose either --basic-password / --password or --prompt-password, not both.",
        ):
            exporter.resolve_auth(args)

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

    def test_build_all_orgs_output_dir_uses_org_id_and_name(self):
        path = exporter.build_all_orgs_output_dir(
            Path("out"),
            {"id": 2, "name": "Ops Org"},
        )

        self.assertEqual(path, Path("out/org_2_Ops_Org"))

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

    def test_format_dashboard_summary_line_uses_defaults(self):
        line = exporter.format_dashboard_summary_line({"uid": "abc"})

        self.assertEqual(
            line,
            "uid=abc name=dashboard folder=General folderUid=general path=General org=Main Org. orgId=1",
        )

    def test_format_dashboard_summary_line_includes_sources_when_present(self):
        line = exporter.format_dashboard_summary_line(
            {
                "uid": "abc",
                "title": "CPU",
                "sources": ["Loki Logs", "Prometheus Main"],
            }
        )

        self.assertEqual(
            line,
            (
                "uid=abc name=CPU folder=General folderUid=general path=General "
                "org=Main Org. orgId=1 sources=Loki Logs,Prometheus Main"
            ),
        )

    def test_build_folder_path_joins_parents_and_title(self):
        path = exporter.build_folder_path(
            {
                "title": "Child",
                "parents": [{"title": "Root"}, {"title": "Team"}],
            },
            fallback_title="Child",
        )

        self.assertEqual(path, "Root / Team / Child")

    def test_attach_dashboard_folder_paths_uses_folder_hierarchy(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "child": {
                    "title": "Child",
                    "parents": [{"title": "Root"}],
                }
            }
        )

        summaries = exporter.attach_dashboard_folder_paths(
            client,
            [
                {"uid": "abc", "folderTitle": "Child", "folderUid": "child", "title": "CPU"},
                {"uid": "xyz", "title": "Overview"},
            ],
        )

        self.assertEqual(summaries[0]["folderPath"], "Root / Child")
        self.assertEqual(summaries[1]["folderPath"], "General")

    def test_attach_dashboard_org_uses_current_org(self):
        client = FakeDashboardWorkflowClient(org={"id": 7, "name": "Ops Org"})

        summaries = exporter.attach_dashboard_org(
            client,
            [{"uid": "abc", "title": "CPU"}],
        )

        self.assertEqual(summaries[0]["orgName"], "Ops Org")
        self.assertEqual(summaries[0]["orgId"], "7")

    def test_format_data_source_line_uses_expected_fields(self):
        line = exporter.format_data_source_line(
            {
                "uid": "prom_uid",
                "name": "Prometheus Main",
                "type": "prometheus",
                "url": "http://prometheus:9090",
                "isDefault": True,
            }
        )

        self.assertEqual(
            line,
            "uid=prom_uid name=Prometheus Main type=prometheus url=http://prometheus:9090 isDefault=true",
        )

    def test_render_data_source_table_uses_headers_and_values(self):
        lines = exporter.render_data_source_table(
            [
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                },
                {
                    "uid": "loki_uid",
                    "name": "Loki Logs",
                    "type": "loki",
                    "url": "http://loki:3100",
                    "isDefault": False,
                },
            ]
        )

        self.assertEqual(lines[0], "UID       NAME             TYPE        URL                     IS_DEFAULT")
        self.assertEqual(lines[2], "prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      ")
        self.assertEqual(lines[3], "loki_uid  Loki Logs        loki        http://loki:3100        false     ")

    def test_render_data_source_table_can_omit_header(self):
        lines = exporter.render_data_source_table(
            [
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ],
            include_header=False,
        )

        self.assertEqual(lines, ["prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      "])

    def test_render_data_source_csv_uses_expected_fields(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exporter.render_data_source_csv(
                [
                    {
                        "uid": "prom_uid",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "url": "http://prometheus:9090",
                        "isDefault": True,
                    }
                ]
            )

        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "uid,name,type,url,isDefault",
                "prom_uid,Prometheus Main,prometheus,http://prometheus:9090,true",
            ],
        )

    def test_render_data_source_json_uses_expected_fields(self):
        document = exporter.render_data_source_json(
            [
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                }
            ]
        )

        self.assertEqual(
            json.loads(document),
            [
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                }
            ],
        )

    def test_inspect_export_renders_human_summary(self):
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
                [{"uid": "abc", "title": "CPU", "path": "General", "kind": "raw"}],
                import_dir / "index.json",
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                [
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
                                "targets": [{"refId": "A"}],
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
                                "id": 1,
                                "type": "timeseries",
                                "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                                "targets": [
                                    {"refId": "A", "datasource": {"type": "prometheus", "uid": "prom-main"}},
                                    {"refId": "B", "datasource": {"type": "loki", "uid": "logs-main"}},
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                },
                import_dir / "Infra" / "Mixed_Main__mixed-main.json",
            )

            args = exporter.parse_args(["inspect-export", "--import-dir", str(import_dir)])
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            output = stdout.getvalue()
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
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                    datasources_file=exporter.DATASOURCE_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document([], import_dir / exporter.FOLDER_INVENTORY_FILENAME)
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
                    }
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
                                "targets": [{"refId": "A"}, {"refId": "B"}],
                            }
                        ],
                    },
                    "meta": {},
                },
                import_dir / "General" / "CPU_Main__cpu-main.json",
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--json"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            payload = json.loads(stdout.getvalue())
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
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                [
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
                                "targets": [{"refId": "A"}],
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
                                "id": 1,
                                "type": "timeseries",
                                "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                                "targets": [
                                    {"refId": "A", "datasource": {"type": "prometheus", "uid": "prom-main"}},
                                    {"refId": "B", "datasource": {"type": "loki", "uid": "logs-main"}},
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                },
                import_dir / "Infra" / "Mixed_Main__mixed-main.json",
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--table"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            output = stdout.getvalue()
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

    def test_inspect_export_table_can_omit_header(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
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
                {
                    "dashboard": {"id": None, "uid": "cpu-main", "title": "CPU Main", "panels": []},
                    "meta": {},
                },
                import_dir / "General" / "CPU_Main__cpu-main.json",
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--table", "--no-header"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("# Summary", output)
            self.assertNotIn("METRIC", output)

    def test_inspect_export_rejects_no_header_without_table(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--no-header"]
        )

        with self.assertRaisesRegex(exporter.GrafanaError, "--no-header is only supported with --table"):
            exporter.inspect_export(args)

    def test_inspect_export_rejects_table_with_json(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--table", "--json"]
        )

        with self.assertRaisesRegex(exporter.GrafanaError, "--table and --json are mutually exclusive"):
            exporter.inspect_export(args)

    def test_render_dashboard_summary_table_uses_headers_and_defaults(self):
        lines = exporter.render_dashboard_summary_table(
            [
                {
                    "uid": "abc",
                    "folderTitle": "Infra",
                    "folderUid": "infra",
                    "folderPath": "Platform / Infra",
                    "title": "CPU",
                    "orgName": "Main Org.",
                    "orgId": "1",
                },
                {"uid": "xyz", "title": "Overview", "orgName": "Main Org.", "orgId": "1"},
            ]
        )

        self.assertEqual(lines[0], "UID  NAME      FOLDER   FOLDER_UID  FOLDER_PATH       ORG        ORG_ID")
        self.assertEqual(lines[2], "abc  CPU       Infra    infra       Platform / Infra  Main Org.  1     ")
        self.assertEqual(lines[3], "xyz  Overview  General  general     General           Main Org.  1     ")

    def test_render_dashboard_summary_table_can_omit_header(self):
        lines = exporter.render_dashboard_summary_table(
            [
                {
                    "uid": "abc",
                    "folderTitle": "Infra",
                    "folderUid": "infra",
                    "folderPath": "Platform / Infra",
                    "title": "CPU",
                    "orgName": "Main Org.",
                    "orgId": "1",
                }
            ],
            include_header=False,
        )

        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("abc  CPU   Infra"))

    def test_render_dashboard_summary_table_includes_sources_column(self):
        lines = exporter.render_dashboard_summary_table(
            [
                {
                    "uid": "abc",
                    "folderTitle": "Infra",
                    "folderUid": "infra",
                    "folderPath": "Platform / Infra",
                    "title": "CPU",
                    "sources": ["Loki Logs", "Prometheus Main"],
                    "orgName": "Main Org.",
                    "orgId": "1",
                }
            ]
        )

        self.assertIn("SOURCES", lines[0])
        self.assertIn("Loki Logs,Prometheus Main", lines[2])
        self.assertTrue(lines[2].startswith("abc  CPU   Infra   infra"))

    def test_render_dashboard_summary_json_uses_expected_fields(self):
        document = exporter.render_dashboard_summary_json(
            [
                {
                    "uid": "abc",
                    "folderTitle": "Infra",
                    "folderUid": "infra",
                    "folderPath": "Platform / Infra",
                    "title": "CPU",
                    "orgName": "Main Org.",
                    "orgId": "1",
                }
            ]
        )

        self.assertEqual(
            json.loads(document),
            [
                {
                    "uid": "abc",
                    "name": "CPU",
                    "folder": "Infra",
                    "folderUid": "infra",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                }
            ],
        )

    def test_render_dashboard_summary_json_includes_sources_when_present(self):
        document = exporter.render_dashboard_summary_json(
            [
                {
                    "uid": "abc",
                    "folderTitle": "Infra",
                    "folderUid": "infra",
                    "folderPath": "Platform / Infra",
                    "title": "CPU",
                    "sources": ["Loki Logs", "Prometheus Main"],
                    "sourceUids": ["loki_uid", "prom_uid"],
                    "orgName": "Main Org.",
                    "orgId": "1",
                }
            ]
        )

        self.assertEqual(
            json.loads(document),
            [
                {
                    "uid": "abc",
                    "name": "CPU",
                    "folder": "Infra",
                    "folderUid": "infra",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                    "sources": ["Loki Logs", "Prometheus Main"],
                    "sourceUids": ["loki_uid", "prom_uid"],
                }
            ],
        )

    def test_render_dashboard_summary_csv_includes_sources_column(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exporter.render_dashboard_summary_csv(
                [
                    {
                        "uid": "abc",
                        "folderTitle": "Infra",
                        "folderUid": "infra",
                        "folderPath": "Platform / Infra",
                        "title": "CPU",
                        "sources": ["Loki Logs", "Prometheus Main"],
                        "sourceUids": ["loki_uid", "prom_uid"],
                        "orgName": "Main Org.",
                        "orgId": "1",
                    }
                ]
            )

        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "uid,name,folder,folderUid,path,org,orgId,sources,sourceUids",
                "abc,CPU,Infra,infra,Platform / Infra,Main Org.,1,\"Loki Logs,Prometheus Main\",\"loki_uid,prom_uid\"",
            ],
        )

    def test_attach_dashboard_sources_resolves_datasource_names(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}},
                            {"datasource": "loki_uid"},
                            {"targets": [{"datasource": "Prometheus Main"}]},
                            {"datasource": "-- Grafana --"},
                        ],
                    }
                }
            },
            datasources=[
                {"uid": "prom_uid", "name": "Prometheus Main", "type": "prometheus"},
                {"uid": "loki_uid", "name": "Loki Logs", "type": "loki"},
            ],
        )

        summaries = exporter.attach_dashboard_sources(
            client,
            [{"uid": "abc", "title": "CPU"}],
        )

        self.assertEqual(summaries[0]["sources"], ["Loki Logs", "Prometheus Main"])
        self.assertEqual(summaries[0]["sourceUids"], ["loki_uid", "prom_uid"])

    def test_list_dashboards_prints_table_by_default(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
            no_header=False,
        )
        client = FakeDashboardWorkflowClient(
            summaries=[
                {"uid": "abc", "folderTitle": "Infra", "folderUid": "infra", "title": "CPU"},
                {"uid": "xyz", "title": "Overview"},
            ],
            folders={
                "infra": {"title": "Infra", "parents": [{"title": "Platform"}]},
            },
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "UID  NAME      FOLDER   FOLDER_UID  FOLDER_PATH       ORG        ORG_ID",
                "---  --------  -------  ----------  ----------------  ---------  ------",
                "abc  CPU       Infra    infra       Platform / Infra  Main Org.  1     ",
                "xyz  Overview  General  general     General           Main Org.  1     ",
                "",
                "Listed 2 dashboard summaries from http://127.0.0.1:3000",
            ],
        )

    def test_list_dashboards_no_header_hides_table_header(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
            no_header=True,
        )
        client = FakeDashboardWorkflowClient(
            summaries=[
                {"uid": "abc", "folderTitle": "Infra", "folderUid": "infra", "title": "CPU"},
                {"uid": "xyz", "title": "Overview"},
            ],
            folders={
                "infra": {"title": "Infra", "parents": [{"title": "Platform"}]},
            },
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "abc  CPU       Infra    infra       Platform / Infra  Main Org.  1     ",
                "xyz  Overview  General  general     General           Main Org.  1     ",
                "",
                "Listed 2 dashboard summaries from http://127.0.0.1:3000",
            ],
        )

    def test_list_dashboards_prints_csv_when_requested(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=True,
            json=False,
            no_header=False,
        )
        client = FakeDashboardWorkflowClient(
            summaries=[
                {"uid": "abc", "folderTitle": "Infra", "folderUid": "infra", "title": "CPU"},
                {"uid": "xyz", "title": "Overview"},
            ],
            folders={
                "infra": {"title": "Infra", "parents": [{"title": "Platform"}]},
            },
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "uid,name,folder,folderUid,path,org,orgId",
                "abc,CPU,Infra,infra,Platform / Infra,Main Org.,1",
                "xyz,Overview,General,general,General,Main Org.,1",
            ],
        )

    def test_list_dashboards_prints_json_when_requested(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=False,
            json=True,
            no_header=False,
        )
        client = FakeDashboardWorkflowClient(
            summaries=[
                {"uid": "abc", "folderTitle": "Infra", "folderUid": "infra", "title": "CPU"},
                {"uid": "xyz", "title": "Overview"},
            ],
            folders={
                "infra": {"title": "Infra", "parents": [{"title": "Platform"}]},
            },
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            json.loads(stdout.getvalue()),
            [
                {
                    "uid": "abc",
                    "name": "CPU",
                    "folder": "Infra",
                    "folderUid": "infra",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "xyz",
                    "name": "Overview",
                    "folder": "General",
                    "folderUid": "general",
                    "path": "General",
                    "org": "Main Org.",
                    "orgId": "1",
                },
            ],
        )

    def test_list_dashboards_with_sources_includes_resolved_datasource_names(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=False,
            with_sources=True,
            table=False,
            csv=False,
            json=False,
            no_header=False,
        )
        client = FakeDashboardWorkflowClient(
            summaries=[
                {"uid": "abc", "folderTitle": "Infra", "folderUid": "infra", "title": "CPU"},
            ],
            dashboards={
                "abc": {
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}},
                            {"datasource": "Loki Logs"},
                        ],
                    }
                }
            },
            datasources=[
                {"uid": "prom_uid", "name": "Prometheus Main", "type": "prometheus"},
                {"uid": "loki_uid", "name": "Loki Logs", "type": "loki"},
            ],
            folders={
                "infra": {"title": "Infra", "parents": [{"title": "Platform"}]},
            },
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "UID  NAME  FOLDER  FOLDER_UID  FOLDER_PATH       ORG        ORG_ID  SOURCES                  ",
                "---  ----  ------  ----------  ----------------  ---------  ------  -------------------------",
                "abc  CPU   Infra   infra       Platform / Infra  Main Org.  1       Loki Logs,Prometheus Main",
                "",
                "Listed 1 dashboard summaries from http://127.0.0.1:3000",
            ],
        )

    def test_list_dashboards_with_org_id_uses_scoped_client(self):
        args = argparse.Namespace(
            command="list-dashboard",
            url="http://127.0.0.1:3000",
            api_token=None,
            username="admin",
            password="admin",
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id="2",
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
        )
        org_two_client = FakeDashboardWorkflowClient(
            summaries=[{"uid": "org2", "title": "Org Two Dashboard"}],
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            org_clients={"2": org_two_client},
            headers={"Authorization": "Basic test"},
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "UID   NAME               FOLDER   FOLDER_UID  FOLDER_PATH  ORG      ORG_ID",
                "----  -----------------  -------  ----------  -----------  -------  ------",
                "org2  Org Two Dashboard  General  general     General      Org Two  2     ",
                "",
                "Listed 1 dashboard summaries from http://127.0.0.1:3000",
            ],
        )

    def test_list_dashboards_with_all_orgs_aggregates_results(self):
        args = argparse.Namespace(
            command="list-dashboard",
            url="http://127.0.0.1:3000",
            api_token=None,
            username="admin",
            password="admin",
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id=None,
            all_orgs=True,
            with_sources=False,
            table=False,
            csv=True,
            json=False,
        )
        org_one_client = FakeDashboardWorkflowClient(
            summaries=[{"uid": "org1", "title": "Org One Dashboard"}],
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        org_two_client = FakeDashboardWorkflowClient(
            summaries=[{"uid": "org2", "title": "Org Two Dashboard"}],
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}, {"id": 2, "name": "Org Two"}],
            org_clients={"1": org_one_client, "2": org_two_client},
            headers={"Authorization": "Basic test"},
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_dashboards(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "uid,name,folder,folderUid,path,org,orgId",
                "org1,Org One Dashboard,General,general,General,Main Org.,1",
                "org2,Org Two Dashboard,General,general,General,Org Two,2",
            ],
        )

    def test_list_dashboards_rejects_all_orgs_with_org_id(self):
        args = argparse.Namespace(
            command="list-dashboard",
            url="http://127.0.0.1:3000",
            api_token=None,
            username="admin",
            password="admin",
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id="2",
            all_orgs=True,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
        )

        with self.assertRaises(exporter.GrafanaError):
            exporter.list_dashboards(args)

    def test_list_dashboards_rejects_org_switch_with_token_auth(self):
        args = argparse.Namespace(
            command="list-dashboard",
            url="http://127.0.0.1:3000",
            api_token="token",
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            org_id="2",
            all_orgs=False,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
        )
        client = FakeDashboardWorkflowClient(headers={"Authorization": "Bearer token"})

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaises(exporter.GrafanaError):
                exporter.list_dashboards(args)

    def test_list_data_sources_prints_table_by_default(self):
        args = argparse.Namespace(
            command="list-data-sources",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            table=False,
            csv=False,
            json=False,
            no_header=False,
        )
        client = FakeDashboardWorkflowClient(
            datasources=[
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": True,
                },
                {
                    "uid": "loki_uid",
                    "name": "Loki Logs",
                    "type": "loki",
                    "url": "http://loki:3100",
                    "isDefault": False,
                },
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_data_sources(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "UID       NAME             TYPE        URL                     IS_DEFAULT",
                "--------  ---------------  ----------  ----------------------  ----------",
                "prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      ",
                "loki_uid  Loki Logs        loki        http://loki:3100        false     ",
                "",
                "Listed 2 data source(s) from http://127.0.0.1:3000",
            ],
        )

    def test_list_data_sources_no_header_hides_table_header(self):
        args = argparse.Namespace(
            command="list-data-sources",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            table=False,
            csv=False,
            json=False,
            no_header=True,
        )
        client = FakeDashboardWorkflowClient(
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

        with mock.patch.object(exporter, "build_client", return_value=client):
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.list_data_sources(args)

        self.assertEqual(result, 0)
        self.assertEqual(
            stdout.getvalue().splitlines(),
            [
                "prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      ",
                "",
                "Listed 1 data source(s) from http://127.0.0.1:3000",
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

    def test_discover_dashboard_files_ignores_export_metadata(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / exporter.EXPORT_METADATA_FILENAME).write_text("{}", encoding="utf-8")
            dashboard_path = root / "team" / "dash.json"
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)
            dashboard_path.write_text('{"dashboard": {"uid": "x"}}', encoding="utf-8")

            files = exporter.discover_dashboard_files(root)

            self.assertEqual(files, [dashboard_path])

    def test_discover_dashboard_files_ignores_folder_inventory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / exporter.FOLDER_INVENTORY_FILENAME).write_text("[]", encoding="utf-8")
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
        args = exporter.parse_args(
            ["export-dashboard", "--without-dashboard-raw", "--without-dashboard-prompt"]
        )

        with self.assertRaises(exporter.GrafanaError):
            exporter.export_dashboards(args)

    def test_validate_export_metadata_rejects_unsupported_schema_version(self):
        metadata = exporter.build_export_metadata(
            variant=exporter.RAW_EXPORT_SUBDIR,
            dashboard_count=1,
        )
        metadata["schemaVersion"] = exporter.TOOL_SCHEMA_VERSION + 1

        with self.assertRaises(exporter.GrafanaError):
            exporter.validate_export_metadata(
                metadata,
                metadata_path=Path("/tmp/export-metadata.json"),
                expected_variant=exporter.RAW_EXPORT_SUBDIR,
            )

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

    def test_collect_folder_inventory_includes_parent_chain(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "child": {
                    "uid": "child",
                    "title": "Infra",
                    "parents": [{"uid": "parent", "title": "Platform"}],
                },
                "parent": {
                    "uid": "parent",
                    "title": "Platform",
                    "parents": [],
                },
            }
        )

        records = exporter.collect_folder_inventory(
            client,
            {"id": 1, "name": "Main Org."},
            [{"uid": "abc", "folderUid": "child", "folderTitle": "Infra"}],
        )

        self.assertEqual(
            records,
            [
                {
                    "uid": "parent",
                    "title": "Platform",
                    "parentUid": "",
                    "path": "Platform",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "child",
                    "title": "Infra",
                    "parentUid": "parent",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                },
            ],
        )

    def test_load_folder_inventory_reads_exported_manifest(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                [
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "parent",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )

            records = exporter.load_folder_inventory(import_dir)

        self.assertEqual(records[0]["uid"], "child")
        self.assertEqual(records[0]["parentUid"], "parent")

    def test_load_datasource_inventory_reads_exported_manifest(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                [
                    {
                        "uid": "prom",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": "true",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.DATASOURCE_INVENTORY_FILENAME,
            )

            records = exporter.load_datasource_inventory(import_dir)

        self.assertEqual(records[0]["uid"], "prom")
        self.assertEqual(records[0]["access"], "proxy")

    def test_ensure_folder_inventory_creates_missing_folders_in_order(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "existing": {"uid": "existing", "title": "Existing", "parents": []},
            }
        )

        created = exporter.ensure_folder_inventory(
            client,
            [
                {
                    "uid": "child",
                    "title": "Infra",
                    "parentUid": "parent",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "parent",
                    "title": "Platform",
                    "parentUid": "",
                    "path": "Platform",
                    "org": "Main Org.",
                    "orgId": "1",
                },
            ],
        )

        self.assertEqual(created, 2)
        self.assertIn("parent", client.folders)
        self.assertIn("child", client.folders)

    def test_inspect_folder_inventory_reports_missing_and_mismatch(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "parent": {"uid": "parent", "title": "Platform", "parents": []},
                "child": {
                    "uid": "child",
                    "title": "Legacy Infra",
                    "parents": [{"uid": "parent", "title": "Platform"}],
                },
            }
        )

        records = exporter.inspect_folder_inventory(
            client,
            [
                {
                    "uid": "parent",
                    "title": "Platform",
                    "parentUid": "",
                    "path": "Platform",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "child",
                    "title": "Infra",
                    "parentUid": "parent",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                },
                {
                    "uid": "missing",
                    "title": "Missing",
                    "parentUid": "",
                    "path": "Missing",
                    "org": "Main Org.",
                    "orgId": "1",
                },
            ],
        )

        records_by_uid = dict((record["uid"], record) for record in records)
        self.assertEqual(records_by_uid["parent"]["status"], "match")
        self.assertEqual(records_by_uid["child"]["status"], "mismatch")
        self.assertEqual(records_by_uid["child"]["reason"], "title,path")
        self.assertEqual(records_by_uid["missing"]["status"], "missing")

    def test_resolve_folder_inventory_record_for_dashboard_uses_relative_path_without_meta(self):
        folder_lookup = exporter.build_folder_inventory_lookup(
            [
                {
                    "uid": "child",
                    "title": "Infra",
                    "parentUid": "parent",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                }
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_file = import_dir / "Platform" / "Infra" / "CPU__abc.json"
            dashboard_file.parent.mkdir(parents=True, exist_ok=True)
            dashboard_file.write_text("{}", encoding="utf-8")

            record = exporter.resolve_folder_inventory_record_for_dashboard(
                {},
                dashboard_file,
                import_dir,
                folder_lookup,
            )

        self.assertEqual(record["uid"], "child")
        self.assertEqual(record["path"], "Platform / Infra")

    def test_resolve_folder_inventory_record_for_dashboard_marks_general_as_builtin(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_file = import_dir / "General" / "CPU__abc.json"
            dashboard_file.parent.mkdir(parents=True, exist_ok=True)
            dashboard_file.write_text("{}", encoding="utf-8")

            record = exporter.resolve_folder_inventory_record_for_dashboard(
                {},
                dashboard_file,
                import_dir,
                {},
            )

        self.assertEqual(record["uid"], "general")
        self.assertEqual(record["path"], "General")
        self.assertEqual(record["builtin"], "true")

    def test_resolve_folder_inventory_record_for_dashboard_uses_unique_folder_title_fallback(self):
        folder_lookup = exporter.build_folder_inventory_lookup(
            [
                {
                    "uid": "infra",
                    "title": "Infra",
                    "parentUid": "platform",
                    "path": "Platform / Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                }
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_file = import_dir / "Infra" / "CPU__abc.json"
            dashboard_file.parent.mkdir(parents=True, exist_ok=True)
            dashboard_file.write_text("{}", encoding="utf-8")

            record = exporter.resolve_folder_inventory_record_for_dashboard(
                {},
                dashboard_file,
                import_dir,
                folder_lookup,
            )

        self.assertEqual(record["uid"], "infra")
        self.assertEqual(record["path"], "Platform / Infra")

    def test_render_folder_inventory_dry_run_table_renders_rows(self):
        lines = exporter.render_folder_inventory_dry_run_table(
            [
                {
                    "uid": "child",
                    "destination": "exists",
                    "status": "mismatch",
                    "reason": "path",
                    "expected_path": "Platform / Infra",
                    "actual_path": "Legacy / Infra",
                }
            ]
        )

        self.assertIn("UID", lines[0])
        self.assertIn("EXPECTED_PATH", lines[0])
        self.assertIn("child", lines[2])
        self.assertIn("Legacy / Infra", lines[2])

    def test_export_dashboards_writes_versioned_manifest_files(self):
        summary = {
            "uid": "abc",
            "title": "CPU",
            "folderTitle": "Infra",
            "folderUid": "infra",
        }
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
            datasources=[
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "access": "proxy",
                    "isDefault": True,
                }
            ],
            folders={
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                },
                "platform": {
                    "uid": "platform",
                    "title": "Platform",
                    "parents": [],
                },
            },
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            root_metadata = json.loads(
                (Path(tmpdir) / exporter.EXPORT_METADATA_FILENAME).read_text(
                    encoding="utf-8"
                )
            )
            raw_metadata = json.loads(
                (
                    Path(tmpdir)
                    / exporter.RAW_EXPORT_SUBDIR
                    / exporter.EXPORT_METADATA_FILENAME
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(root_metadata["schemaVersion"], exporter.TOOL_SCHEMA_VERSION)
            self.assertEqual(root_metadata["variant"], "root")
            self.assertEqual(raw_metadata["variant"], exporter.RAW_EXPORT_SUBDIR)
            self.assertEqual(raw_metadata["dashboardCount"], 1)
            self.assertEqual(raw_metadata["foldersFile"], exporter.FOLDER_INVENTORY_FILENAME)
            self.assertEqual(
                raw_metadata["datasourcesFile"], exporter.DATASOURCE_INVENTORY_FILENAME
            )
            folder_inventory = json.loads(
                (
                    Path(tmpdir)
                    / exporter.RAW_EXPORT_SUBDIR
                    / exporter.FOLDER_INVENTORY_FILENAME
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(folder_inventory[0]["uid"], "platform")
            self.assertEqual(folder_inventory[1]["uid"], "infra")
            datasource_inventory = json.loads(
                (
                    Path(tmpdir)
                    / exporter.RAW_EXPORT_SUBDIR
                    / exporter.DATASOURCE_INVENTORY_FILENAME
                ).read_text(encoding="utf-8")
            )
            self.assertEqual(datasource_inventory[0]["uid"], "prom-main")
            self.assertEqual(datasource_inventory[0]["type"], "prometheus")

    def test_export_dashboards_progress_is_opt_in(self):
        summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--progress",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Exporting dashboard 1/1: abc",
                    "Exported 1 dashboards. Raw index: %s Raw manifest: %s Raw datasources: %s Root index: %s Root manifest: %s"
                    % (
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / "index.json",
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.EXPORT_METADATA_FILENAME,
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.DATASOURCE_INVENTORY_FILENAME,
                        Path(tmpdir) / "index.json",
                        Path(tmpdir) / exporter.EXPORT_METADATA_FILENAME,
                    ),
                ],
            )

    def test_export_dashboards_verbose_prints_paths(self):
        summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Exported raw    abc -> %s"
                    % (Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / "Infra" / "CPU__abc.json"),
                    "Exported 1 dashboards. Raw index: %s Raw manifest: %s Raw datasources: %s Root index: %s Root manifest: %s"
                    % (
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / "index.json",
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.EXPORT_METADATA_FILENAME,
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.DATASOURCE_INVENTORY_FILENAME,
                        Path(tmpdir) / "index.json",
                        Path(tmpdir) / exporter.EXPORT_METADATA_FILENAME,
                    ),
                ],
            )

    def test_export_dashboards_verbose_supersedes_progress_output(self):
        summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--progress",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Exported raw    abc -> %s"
                    % (Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / "Infra" / "CPU__abc.json"),
                    "Exported 1 dashboards. Raw index: %s Raw manifest: %s Raw datasources: %s Root index: %s Root manifest: %s"
                    % (
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / "index.json",
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.EXPORT_METADATA_FILENAME,
                        Path(tmpdir) / exporter.RAW_EXPORT_SUBDIR / exporter.DATASOURCE_INVENTORY_FILENAME,
                        Path(tmpdir) / "index.json",
                        Path(tmpdir) / exporter.EXPORT_METADATA_FILENAME,
                    ),
                ],
            )

    def test_export_dashboards_dry_run_keeps_directory_empty(self):
        summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(list(Path(tmpdir).rglob("*.json")), [])

    def test_export_dashboards_with_org_id_uses_scoped_client(self):
        summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        scoped_client = FakeDashboardWorkflowClient(
            summaries=[summary],
            dashboards={"abc": dashboard},
            org={"id": 2, "name": "Org Two"},
        )
        client = FakeDashboardWorkflowClient(org_clients={"2": scoped_client})

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--org-id",
                    "2",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertTrue((Path(tmpdir) / "raw/Infra/CPU__abc.json").is_file())
            root_index = json.loads((Path(tmpdir) / "index.json").read_text(encoding="utf-8"))
            self.assertEqual(root_index["items"][0]["org"], "Org Two")
            self.assertEqual(root_index["items"][0]["orgId"], "2")

    def test_export_dashboards_with_all_orgs_uses_org_prefix_dirs(self):
        org_one_summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        org_two_summary = {"uid": "abc", "title": "CPU", "folderTitle": "Infra"}
        org_one_dashboard = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        org_two_dashboard = {
            "dashboard": {"id": 8, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        org_one_client = FakeDashboardWorkflowClient(
            summaries=[org_one_summary],
            dashboards={"abc": org_one_dashboard},
            org={"id": 1, "name": "Main Org."},
        )
        org_two_client = FakeDashboardWorkflowClient(
            summaries=[org_two_summary],
            dashboards={"abc": org_two_dashboard},
            org={"id": 2, "name": "Org Two"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}, {"id": 2, "name": "Org Two"}],
            org_clients={"1": org_one_client, "2": org_two_client},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args = exporter.parse_args(
                [
                    "export-dashboard",
                    "--export-dir",
                    tmpdir,
                    "--without-dashboard-prompt",
                    "--all-orgs",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.export_dashboards(args)

            self.assertEqual(result, 0)
            self.assertTrue(
                (Path(tmpdir) / "org_1_Main_Org/raw/Infra/CPU__abc.json").is_file()
            )
            self.assertTrue(
                (Path(tmpdir) / "org_2_Org_Two/raw/Infra/CPU__abc.json").is_file()
            )
            self.assertTrue((Path(tmpdir) / "raw/index.json").is_file())
            root_index = json.loads((Path(tmpdir) / "index.json").read_text(encoding="utf-8"))
            self.assertEqual(len(root_index["items"]), 2)
            self.assertEqual(
                sorted(item["orgId"] for item in root_index["items"]),
                ["1", "2"],
            )
            self.assertTrue(str(root_index["variants"]["raw"]).endswith("/raw/index.json"))

    def test_export_dashboards_rejects_all_orgs_with_org_id(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(
            [
                "export-dashboard",
                "--without-dashboard-prompt",
                "--org-id",
                "2",
                "--all-orgs",
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaises(exporter.GrafanaError):
                exporter.export_dashboards(args)

    def test_export_dashboards_rejects_org_switch_with_token_auth(self):
        client = FakeDashboardWorkflowClient(headers={"Authorization": "Bearer token"})
        args = exporter.parse_args(
            [
                "export-dashboard",
                "--without-dashboard-prompt",
                "--org-id",
                "2",
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "Basic auth"):
                exporter.export_dashboards(args)

    def test_import_dashboards_dry_run_skips_api_write(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.imported_payloads, [])

    def test_import_dashboards_dry_run_verbose_reports_destination_state(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--verbose"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Dry-run import uid=abc dest=exists action=blocked-existing folderPath=General file=%s"
                    % (import_dir / "cpu__abc.json"),
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_dry_run_progress_reports_destination_state(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--progress"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Dry-run dashboard 1/1: abc dest=missing action=create folderPath=General",
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_dry_run_ensure_folders_verbose_reports_folder_status(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "parent": {"uid": "parent", "title": "Platform", "parents": []},
                "child": {
                    "uid": "child",
                    "title": "Legacy Infra",
                    "parents": [{"uid": "parent", "title": "Platform"}],
                },
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "parent",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "parent",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--dry-run",
                    "--ensure-folders",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.created_folders, [])
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Dry-run folder uid=parent dest=exists status=match reason=- expected=Platform actual=Platform",
                    "Dry-run folder uid=child dest=exists status=mismatch reason=title,path expected=Platform / Infra actual=Platform / Legacy Infra",
                    "Dry-run checked 2 folder(s) from %s; 0 missing, 1 mismatched"
                    % (import_dir / exporter.FOLDER_INVENTORY_FILENAME),
                    "Dry-run import uid=abc dest=missing action=create folderPath=Platform / Legacy Infra file=%s"
                    % (import_dir / "cpu__abc.json"),
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_dry_run_table_ensure_folders_includes_folder_status(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "",
                        "path": "Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--dry-run",
                    "--ensure-folders",
                    "--table",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            lines = stdout.getvalue().splitlines()
            self.assertTrue(any("FOLDER_PATH" in line for line in lines))
            self.assertTrue(
                any(
                    "cpu__abc.json" in line
                    and "missing" in line
                    and "Infra" in line
                    for line in lines
                )
            )

    def test_import_dashboards_dry_run_table_renders_rows(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--table"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            lines = stdout.getvalue().splitlines()
            self.assertEqual(lines[0], "Import mode: create-only")
            self.assertEqual(lines[-1], "Dry-run checked 2 dashboard files from %s" % import_dir)
            self.assertIn("UID", lines[1])
            self.assertIn("DESTINATION", lines[1])
            self.assertIn("ACTION", lines[1])
            self.assertIn("FOLDER_PATH", lines[1])
            self.assertIn("FILE", lines[1])
            self.assertIn("abc", lines[3])
            self.assertIn("exists", lines[3])
            self.assertIn("blocked-existing", lines[3])
            self.assertIn("General", lines[3])
            self.assertIn(str(import_dir / "cpu__abc.json"), lines[3])
            self.assertIn("xyz", lines[4])
            self.assertIn("missing", lines[4])
            self.assertIn("create", lines[4])
            self.assertIn("General", lines[4])
            self.assertIn(str(import_dir / "memory__xyz.json"), lines[4])

    def test_import_dashboards_dry_run_table_can_omit_header(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--table", "--no-header"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "xyz  missing      create  General      %s" % (import_dir / "memory__xyz.json"),
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_dry_run_table_marks_missing_dashboards_as_skipped_when_update_existing_only(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--dry-run",
                    "--table",
                    "--update-existing-only",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            lines = stdout.getvalue().splitlines()
            self.assertEqual(lines[0], "Import mode: update-or-skip-missing")
            self.assertIn("abc", lines[3])
            self.assertIn("update", lines[3])
            self.assertIn("infra", lines[3])
            self.assertIn("xyz", lines[4])
            self.assertIn("skip-missing", lines[4])
            self.assertIn("General", lines[4])
            self.assertEqual(
                lines[-1],
                "Dry-run checked 2 dashboard files from %s; would skip 1 missing dashboards"
                % import_dir,
            )

    def test_import_dashboards_update_existing_only_skips_missing_live_dashboards(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--update-existing-only",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(len(client.imported_payloads), 1)
            self.assertEqual(client.imported_payloads[0]["dashboard"]["uid"], "abc")
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: update-or-skip-missing",
                    "Imported %s -> uid=abc status=success" % (import_dir / "cpu__abc.json"),
                    "Skipped import uid=xyz dest=missing action=skip-missing file=%s"
                    % (import_dir / "memory__xyz.json"),
                    "Imported 1 dashboard files from %s; skipped 1 missing dashboards" % import_dir,
                ],
            )

    def test_import_dashboards_update_existing_only_progress_shows_skips(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--update-existing-only",
                    "--progress",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: update-or-skip-missing",
                    "Importing dashboard 1/2: abc",
                    "Skipping dashboard 2/2: xyz dest=missing action=skip-missing",
                    "Imported 1 dashboard files from %s; skipped 1 missing dashboards" % import_dir,
                ],
            )

    def test_import_dashboards_replace_existing_preserves_destination_folder(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "dest-folder"},
                }
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "source-folder"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--replace-existing"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(len(client.imported_payloads), 1)
            self.assertEqual(client.imported_payloads[0]["folderUid"], "dest-folder")
            self.assertTrue(client.imported_payloads[0]["overwrite"])

    def test_import_dashboards_dry_run_table_uses_destination_folder_path_for_updates(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "dest-folder"},
                }
            },
            folders={
                "dest-folder": {
                    "uid": "dest-folder",
                    "title": "Ops",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                }
            },
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "source-folder"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--dry-run",
                    "--replace-existing",
                    "--table",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Platform / Ops", output)

    def test_import_dashboards_rejects_table_without_dry_run(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--table"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--table is only supported with --dry-run"):
                exporter.import_dashboards(args)

    def test_import_dashboards_rejects_json_without_dry_run(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--json"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--json is only supported with --dry-run"):
                exporter.import_dashboards(args)

    def test_import_dashboards_rejects_table_with_json(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--table", "--json"]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--table and --json are mutually exclusive"):
                exporter.import_dashboards(args)

    def test_import_dashboards_rejects_no_header_without_table(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--no-header"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--no-header is only supported with --dry-run --table"):
                exporter.import_dashboards(args)

    def test_import_dashboards_ensure_folders_creates_missing_folders_from_inventory(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "parent",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "parent",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--ensure-folders"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertIn("Ensured 2 folder(s)", stdout.getvalue())
            self.assertIn("parent", client.folders)
            self.assertIn("child", client.folders)

    def test_import_dashboards_ensure_folders_requires_inventory_manifest(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--ensure-folders"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    exporter.GrafanaError,
                    "Folder inventory file not found for --ensure-folders",
                ):
                    exporter.import_dashboards(args)

    def test_import_dashboards_dry_run_ensure_folders_reports_folder_status(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "parent": {"uid": "parent", "title": "Platform", "parents": []},
                "child": {
                    "uid": "child",
                    "title": "Legacy Infra",
                    "parents": [{"uid": "parent", "title": "Platform"}],
                },
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "parent",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "parent",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "missing",
                        "title": "Missing",
                        "parentUid": "",
                        "path": "Missing",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--ensure-folders"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("Dry-run folder uid=parent dest=exists status=match", output)
            self.assertIn("Dry-run folder uid=child dest=exists status=mismatch", output)
            self.assertIn("actual=Platform / Legacy Infra", output)
            self.assertIn("Dry-run folder uid=missing dest=missing status=missing", output)
            self.assertIn("Dry-run checked 3 folder(s)", output)

    def test_import_dashboards_dry_run_ensure_folders_table_renders_folder_table(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "parent": {"uid": "parent", "title": "Platform", "parents": []},
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "parent",
                        "title": "Platform",
                        "parentUid": "",
                        "path": "Platform",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "child",
                        "title": "Infra",
                        "parentUid": "parent",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "child"},
                },
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--ensure-folders", "--table"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("EXPECTED_PATH", output)
            self.assertIn("ACTUAL_PATH", output)
            self.assertIn("FOLDER_PATH", output)
            self.assertIn("Platform / Infra", output)
            self.assertIn("UID", output)
            self.assertIn("DESTINATION", output)

    def test_import_dashboards_dry_run_table_marks_general_folder_as_default(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "",
                        "path": "Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {
                    "dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {},
                },
                import_dir / "General" / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--dry-run", "--ensure-folders", "--table"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("General", output)

    def test_import_dashboards_dry_run_json_renders_structured_output(self):
        client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            },
            folders={
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                }
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
                        "uid": "infra",
                        "title": "Infra",
                        "parentUid": "platform",
                        "path": "Platform / Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "xyz", "title": "Memory", "panels": []}},
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--dry-run",
                    "--replace-existing",
                    "--ensure-folders",
                    "--json",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["mode"], "create-or-update")
            self.assertEqual(payload["summary"]["folderCount"], 1)
            self.assertEqual(payload["summary"]["dashboardCount"], 2)
            self.assertEqual(payload["summary"]["missingDashboards"], 1)
            self.assertEqual(payload["dashboards"][0]["uid"], "abc")
            self.assertEqual(payload["dashboards"][0]["action"], "update")
            self.assertEqual(payload["dashboards"][0]["folderPath"], "Platform / Infra")
            self.assertEqual(payload["dashboards"][1]["uid"], "xyz")
            self.assertEqual(payload["dashboards"][1]["action"], "create")

    def test_import_dashboards_progress_is_opt_in(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--progress"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Importing dashboard 1/1: abc",
                    "Imported 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_verbose_prints_paths(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["import-dashboard", "--import-dir", str(import_dir), "--verbose"]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Imported %s -> uid=abc status=success" % (import_dir / "cpu__abc.json"),
                    "Imported 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_verbose_supersedes_progress_output(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--progress",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Imported %s -> uid=abc status=success" % (import_dir / "cpu__abc.json"),
                    "Imported 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_import_dashboards_rejects_unsupported_manifest_schema(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                {
                    "schemaVersion": exporter.TOOL_SCHEMA_VERSION + 1,
                    "kind": exporter.ROOT_INDEX_KIND,
                    "variant": exporter.RAW_EXPORT_SUBDIR,
                    "dashboardCount": 1,
                    "indexFile": "index.json",
                },
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(["import-dashboard", "--import-dir", str(import_dir)])

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaises(exporter.GrafanaError):
                    exporter.import_dashboards(args)

    def test_diff_dashboards_returns_zero_when_dashboard_matches(self):
        remote_payload = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(dashboards={"abc": remote_payload})

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(["diff", "--import-dir", str(import_dir)])

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.diff_dashboards(args)

            self.assertEqual(result, 0)

    def test_diff_dashboards_prints_unified_diff_when_dashboard_changes(self):
        remote_payload = {
            "dashboard": {"id": 7, "uid": "abc", "title": "Memory", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(dashboards={"abc": remote_payload})

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                exporter.build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                ["diff", "--import-dir", str(import_dir), "--context-lines", "1"]
            )
            stdout = io.StringIO()

            with mock.patch.object(exporter, "build_client", return_value=client):
                with redirect_stdout(stdout):
                    result = exporter.diff_dashboards(args)

            self.assertEqual(result, 1)
            self.assertIn("--- grafana:abc", stdout.getvalue())
            self.assertIn("+++ ", stdout.getvalue())

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
            "${DS_PROM_MAIN}",
        )
        self.assertEqual(
            document["panels"][0]["targets"][0]["datasource"]["uid"],
            "${DS_PROM_MAIN}",
        )
        self.assertEqual(document["panels"][1]["datasource"], "${DS_LOKI_LOGS}")
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_LOKI_LOGS", "DS_PROM_MAIN"],
        )
        self.assertEqual(
            [item["label"] for item in document["__inputs"]],
            ["Loki Logs", "Prom Main"],
        )
        self.assertEqual(
            [item["pluginName"] for item in document["__inputs"]],
            ["Loki", "Prometheus"],
        )
        self.assertEqual(
            {item["id"] for item in document["__requires"] if item["type"] == "datasource"},
            {"loki", "prometheus"},
        )
        self.assertEqual(document["__elements"], {})

    def test_build_preserved_web_import_document_keeps_mixed_panel_query_datasources(self):
        payload = {
            "dashboard": {
                "id": 16,
                "uid": "mixed-query-smoke",
                "title": "Mixed Query Dashboard",
                "panels": [
                    {
                        "id": 1,
                        "type": "timeseries",
                        "title": "Mixed Panel",
                        "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"type": "prometheus", "uid": "prom_uid"},
                                "expr": "up",
                            },
                            {
                                "refId": "B",
                                "datasource": {"type": "loki", "uid": "loki_uid"},
                                "expr": '{job="grafana"}',
                            },
                        ],
                    }
                ],
            }
        }

        document = exporter.build_preserved_web_import_document(payload)

        self.assertIsNone(document["id"])
        self.assertEqual(document["panels"][0]["datasource"]["uid"], "-- Mixed --")
        self.assertEqual(
            document["panels"][0]["targets"][0]["datasource"],
            {"type": "prometheus", "uid": "prom_uid"},
        )
        self.assertEqual(
            document["panels"][0]["targets"][1]["datasource"],
            {"type": "loki", "uid": "loki_uid"},
        )

    def test_build_external_export_document_rewrites_mixed_panel_query_datasources(self):
        payload = {
            "dashboard": {
                "id": 17,
                "title": "Mixed Query Dashboard",
                "panels": [
                    {
                        "id": 1,
                        "type": "timeseries",
                        "title": "Mixed Panel",
                        "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"type": "prometheus", "uid": "prom_uid"},
                                "expr": "up",
                            },
                            {
                                "refId": "B",
                                "datasource": {"type": "loki", "uid": "loki_uid"},
                                "expr": '{job="grafana"}',
                            },
                        ],
                    }
                ],
            }
        }
        catalog = exporter.build_datasource_catalog(
            [
                {"uid": "prom_uid", "name": "Smoke Prometheus", "type": "prometheus"},
                {"uid": "loki_uid", "name": "Smoke Loki", "type": "loki"},
            ]
        )

        document = exporter.build_external_export_document(payload, catalog)

        self.assertEqual(
            document["panels"][0]["datasource"],
            {"type": "datasource", "uid": "-- Mixed --"},
        )
        self.assertEqual(
            document["panels"][0]["targets"][0]["datasource"],
            {"type": "prometheus", "uid": "${DS_SMOKE_PROMETHEUS}"},
        )
        self.assertEqual(
            document["panels"][0]["targets"][1]["datasource"],
            {"type": "loki", "uid": "${DS_SMOKE_LOKI}"},
        )
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_SMOKE_LOKI", "DS_SMOKE_PROMETHEUS"],
        )
        self.assertEqual(
            [item["label"] for item in document["__inputs"]],
            ["Smoke Loki", "Smoke Prometheus"],
        )
        self.assertEqual(
            {item["id"] for item in document["__requires"] if item["type"] == "datasource"},
            {"loki", "prometheus"},
        )

    def test_build_external_export_document_keeps_distinct_same_type_datasources_separate(self):
        payload = {
            "dashboard": {
                "id": 18,
                "title": "Two Prometheus Query Dashboard",
                "panels": [
                    {
                        "id": 1,
                        "type": "timeseries",
                        "title": "Two Prometheus Panel",
                        "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                        "targets": [
                            {
                                "refId": "A",
                                "datasource": {"type": "prometheus", "uid": "prom_uid_1"},
                                "expr": "up",
                            },
                            {
                                "refId": "B",
                                "datasource": {"type": "prometheus", "uid": "prom_uid_2"},
                                "expr": "up",
                            },
                        ],
                    }
                ],
            }
        }
        catalog = exporter.build_datasource_catalog(
            [
                {"uid": "prom_uid_1", "name": "Smoke Prometheus", "type": "prometheus"},
                {"uid": "prom_uid_2", "name": "Smoke Prometheus 2", "type": "prometheus"},
            ]
        )

        document = exporter.build_external_export_document(payload, catalog)

        self.assertEqual(
            document["panels"][0]["datasource"],
            {"type": "datasource", "uid": "-- Mixed --"},
        )
        self.assertEqual(
            document["panels"][0]["targets"][0]["datasource"],
            {"type": "prometheus", "uid": "${DS_SMOKE_PROMETHEUS}"},
        )
        self.assertEqual(
            document["panels"][0]["targets"][1]["datasource"],
            {"type": "prometheus", "uid": "${DS_SMOKE_PROMETHEUS_2}"},
        )
        self.assertEqual(
            [item["name"] for item in document["__inputs"]],
            ["DS_SMOKE_PROMETHEUS", "DS_SMOKE_PROMETHEUS_2"],
        )
        self.assertNotIn("templating", document)

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
            ["DS_PROD_PROMETHEUS"],
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
            ["DS_PROMETHEUS"],
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
            ["DS_PROMETHEUS"],
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
            ["DS_PROMETHEUS"],
        )
        self.assertEqual(document["templating"]["list"][0]["current"], {})
        self.assertEqual(document["templating"]["list"][0]["query"], "prometheus")
        self.assertEqual(
            document["templating"]["list"][1]["datasource"]["uid"],
            "${DS_PROMETHEUS}",
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
