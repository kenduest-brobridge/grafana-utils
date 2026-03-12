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
WRAPPER_PATH = REPO_ROOT / "cmd" / "grafana-utils.py"
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
    def __init__(self, summaries=None, dashboards=None, datasources=None, folders=None, org=None):
        self.summaries = summaries or []
        self.dashboards = dashboards or {}
        self.datasources = datasources or []
        self.folders = folders or {}
        self.org = org or {"id": 1, "name": "Main Org."}
        self.imported_payloads = []

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

    def list_datasources(self):
        return list(self.datasources)

    def fetch_current_org(self):
        return dict(self.org)

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

    def test_dashboard_wrapper_script_parses_as_python36_syntax(self):
        source = WRAPPER_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(WRAPPER_PATH), feature_version=(3, 6))

    def test_parse_args_requires_subcommand(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args([])

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

    def test_parse_args_supports_list_mode(self):
        args = exporter.parse_args(["list-dashboard", "--page-size", "25", "--table"])

        self.assertEqual(args.command, "list-dashboard")
        self.assertEqual(args.page_size, 25)
        self.assertTrue(args.table)
        self.assertFalse(args.with_sources)
        self.assertFalse(args.csv)
        self.assertFalse(args.json)

    def test_parse_args_supports_list_csv_and_json_modes(self):
        csv_args = exporter.parse_args(["list-dashboard", "--csv"])
        json_args = exporter.parse_args(["list-dashboard", "--json"])
        source_args = exporter.parse_args(["list-dashboard", "--with-sources"])

        self.assertTrue(csv_args.csv)
        self.assertFalse(csv_args.table)
        self.assertFalse(csv_args.json)
        self.assertTrue(json_args.json)
        self.assertFalse(json_args.table)
        self.assertFalse(json_args.csv)
        self.assertTrue(source_args.with_sources)

    def test_parse_args_rejects_multiple_list_output_modes(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--csv", "--json"])

    def test_parse_args_supports_diff_mode(self):
        args = exporter.parse_args(["diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(args.import_dir, "dashboards/raw")
        self.assertEqual(args.command, "diff")
        self.assertEqual(args.context_lines, 3)

    def test_parse_args_defaults_export_dir_to_dashboards(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.export_dir, "dashboards")
        self.assertEqual(args.command, "export-dashboard")

    def test_parse_args_defaults_url_to_local_grafana(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.url, "http://127.0.0.1:3000")

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
        )

        headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:pass").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")

    def test_resolve_auth_rejects_mixed_token_and_basic_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="user",
            password="pass",
        )

        with self.assertRaisesRegex(exporter.GrafanaError, "Choose either token auth"):
            exporter.resolve_auth(args)

    def test_resolve_auth_rejects_user_without_password(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password=None,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user / --username and --basic-password / --password.",
        ):
            exporter.resolve_auth(args)

    def test_resolve_auth_rejects_password_without_user(self):
        args = argparse.Namespace(
            api_token=None,
            username=None,
            password="pass",
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user / --username and --basic-password / --password.",
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

    def test_list_dashboards_prints_live_summaries(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            with_sources=False,
            table=False,
            csv=False,
            json=False,
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
                "uid=abc name=CPU folder=Infra folderUid=infra path=Platform / Infra org=Main Org. orgId=1",
                "uid=xyz name=Overview folder=General folderUid=general path=General org=Main Org. orgId=1",
                "",
                "Listed 2 dashboard summaries from http://127.0.0.1:3000",
            ],
        )

    def test_list_dashboards_prints_table_when_requested(self):
        args = argparse.Namespace(
            command="list",
            url="http://127.0.0.1:3000",
            api_token=None,
            username=None,
            password=None,
            timeout=30,
            verify_ssl=False,
            page_size=50,
            with_sources=False,
            table=True,
            csv=False,
            json=False,
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
            with_sources=False,
            table=False,
            csv=True,
            json=False,
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
            with_sources=False,
            table=False,
            csv=False,
            json=True,
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
            with_sources=True,
            table=False,
            csv=False,
            json=False,
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
                "uid=abc name=CPU folder=Infra folderUid=infra path=Platform / Infra org=Main Org. orgId=1 sources=Loki Logs,Prometheus Main",
                "",
                "Listed 1 dashboard summaries from http://127.0.0.1:3000",
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

    def test_export_dashboards_writes_versioned_manifest_files(self):
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
