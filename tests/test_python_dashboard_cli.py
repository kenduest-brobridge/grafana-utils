import argparse
import ast
import base64
import io
import importlib
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboard_cli.py"
TRANSPORT_MODULE_PATH = REPO_ROOT / "grafana_utils" / "http_transport.py"
CLIENT_MODULE_PATH = REPO_ROOT / "grafana_utils" / "clients" / "dashboard_client.py"
COMMON_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "common.py"
EXPORT_WORKFLOW_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "export_workflow.py"
)
EXPORT_INVENTORY_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "export_inventory.py"
)
FOLDER_SUPPORT_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "folder_support.py"
)
IMPORT_SUPPORT_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "import_support.py"
)
PROGRESS_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "progress.py"
IMPORT_WORKFLOW_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "import_workflow.py"
)
INSPECTION_WORKFLOW_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_workflow.py"
)
INSPECTION_DISPATCH_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_dispatch.py"
)
INSPECTION_ANALYZERS_PACKAGE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "__init__.py"
)
INSPECTION_ANALYZER_CONTRACT_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "contract.py"
)
INSPECTION_ANALYZER_DISPATCHER_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "dispatcher.py"
)
INSPECTION_ANALYZER_PROMETHEUS_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "prometheus.py"
)
INSPECTION_ANALYZER_FLUX_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "flux.py"
)
INSPECTION_ANALYZER_SQL_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "sql.py"
)
INSPECTION_ANALYZER_GENERIC_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_analyzers" / "generic.py"
)
INSPECTION_REPORT_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_report.py"
)
INSPECTION_RENDER_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_render.py"
)
INSPECTION_SUMMARY_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "inspection_summary.py"
)
GOVERNANCE_GATE_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboard_governance_gate.py"
LISTING_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "listing.py"
OUTPUT_SUPPORT_MODULE_PATH = (
    REPO_ROOT / "grafana_utils" / "dashboards" / "output_support.py"
)
PROGRESS_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "progress.py"
TRANSFORMER_MODULE_PATH = REPO_ROOT / "grafana_utils" / "dashboards" / "transformer.py"
MODULE_ENTRYPOINT_PATH = REPO_ROOT / "grafana_utils" / "__main__.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
transport_module = importlib.import_module("grafana_utils.http_transport")
exporter = importlib.import_module("grafana_utils.dashboard_cli")
export_inventory = importlib.import_module("grafana_utils.dashboards.export_inventory")
folder_support = importlib.import_module("grafana_utils.dashboards.folder_support")
listing_module = importlib.import_module("grafana_utils.dashboards.listing")
output_support = importlib.import_module("grafana_utils.dashboards.output_support")
inspection_dispatcher = importlib.import_module(
    "grafana_utils.dashboards.inspection_analyzers.dispatcher"
)
inspection_output_dispatch = importlib.import_module(
    "grafana_utils.dashboards.inspection_dispatch"
)


def build_export_metadata(
    variant, dashboard_count, format_name=None, folders_file=None, datasources_file=None
):
    return output_support.build_export_metadata(
        variant,
        dashboard_count,
        tool_schema_version=exporter.TOOL_SCHEMA_VERSION,
        root_index_kind=exporter.ROOT_INDEX_KIND,
        format_name=format_name,
        folders_file=folders_file,
        datasources_file=datasources_file,
    )


def build_output_path(output_dir, summary, flat):
    return output_support.build_output_path(
        output_dir,
        summary,
        flat,
        default_folder_title=exporter.DEFAULT_FOLDER_TITLE,
        default_dashboard_title=exporter.DEFAULT_DASHBOARD_TITLE,
        default_unknown_uid=exporter.DEFAULT_UNKNOWN_UID,
    )


def build_all_orgs_output_dir(output_dir, org):
    return output_support.build_all_orgs_output_dir(
        output_dir,
        org,
        default_unknown_uid=exporter.DEFAULT_UNKNOWN_UID,
    )


def build_export_variant_dirs(output_dir):
    return output_support.build_export_variant_dirs(
        output_dir,
        raw_export_subdir=exporter.RAW_EXPORT_SUBDIR,
        prompt_export_subdir=exporter.PROMPT_EXPORT_SUBDIR,
    )


def write_dashboard(payload, output_path, overwrite):
    return output_support.write_dashboard(
        payload,
        output_path,
        overwrite,
        error_cls=exporter.GrafanaError,
    )


def discover_dashboard_files(import_dir):
    return export_inventory.discover_dashboard_files(
        import_dir,
        exporter.RAW_EXPORT_SUBDIR,
        exporter.PROMPT_EXPORT_SUBDIR,
        exporter.EXPORT_METADATA_FILENAME,
        exporter.FOLDER_INVENTORY_FILENAME,
        exporter.DATASOURCE_INVENTORY_FILENAME,
    )


def validate_export_metadata(metadata, metadata_path, expected_variant=None):
    return export_inventory.validate_export_metadata(
        metadata,
        metadata_path,
        root_index_kind=exporter.ROOT_INDEX_KIND,
        tool_schema_version=exporter.TOOL_SCHEMA_VERSION,
        expected_variant=expected_variant,
    )


def load_folder_inventory(import_dir, metadata=None):
    return folder_support.load_folder_inventory(
        import_dir,
        exporter.FOLDER_INVENTORY_FILENAME,
        metadata=metadata,
    )


def load_datasource_inventory(import_dir, metadata=None):
    return folder_support.load_datasource_inventory(
        import_dir,
        exporter.DATASOURCE_INVENTORY_FILENAME,
        metadata=metadata,
    )


def attach_dashboard_sources(client, summaries):
    return listing_module.attach_dashboard_sources(
        client,
        summaries,
        extract_dashboard_object=exporter.extract_dashboard_object,
        datasource_error=exporter.GrafanaError,
    )


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
        self.created_orgs = []

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

    def request_json(self, path, params=None, method="GET", payload=None):
        if path == "/api/orgs" and method == "POST":
            next_org_id = 1
            existing_ids = []
            for item in self.orgs:
                item_id = str(item.get("id") or item.get("orgId") or "").strip()
                if item_id.isdigit():
                    existing_ids.append(int(item_id))
            if existing_ids:
                next_org_id = max(existing_ids) + 1
            name = str((payload or {}).get("name") or "").strip()
            created = {"id": next_org_id, "orgId": next_org_id, "name": name}
            self.created_orgs.append(dict(created))
            self.orgs.append({"id": next_org_id, "name": name})
            self.org_clients[str(next_org_id)] = FakeDashboardWorkflowClient(
                org={"id": next_org_id, "name": name},
                headers=dict(self.headers),
            )
            return created
        raise AssertionError("Unexpected request %s %s" % (method, path))

    def create_organization(self, payload):
        return self.request_json("/api/orgs", method="POST", payload=payload)

    def import_dashboard(self, payload):
        self.imported_payloads.append(payload)
        return {"status": "success", "uid": payload["dashboard"].get("uid")}


class ExporterTests(unittest.TestCase):
    def _write_multi_org_import_root(self, root_dir, org_exports):
        for item in org_exports:
            org_id = str(item["org_id"])
            org_name = str(item["org_name"])
            raw_dir = root_dir / ("org_%s_%s" % (org_id, org_name.replace(" ", "_"))) / "raw"
            raw_dir.mkdir(parents=True, exist_ok=True)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=len(item["dashboards"]),
                    format_name="grafana-web-import-preserve-uid",
                ),
                raw_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": dashboard["uid"],
                        "title": dashboard["title"],
                        "folder": "General",
                        "org": org_name,
                        "orgId": org_id,
                        "path": "%s__%s.json" % (dashboard["title"], dashboard["uid"]),
                        "format": "grafana-web-import-preserve-uid",
                    }
                    for dashboard in item["dashboards"]
                ],
                raw_dir / "index.json",
            )
            for dashboard in item["dashboards"]:
                exporter.write_json_document(
                    {
                        "dashboard": {
                            "id": None,
                            "uid": dashboard["uid"],
                            "title": dashboard["title"],
                            "panels": [],
                        }
                    },
                    raw_dir / ("%s__%s.json" % (dashboard["title"], dashboard["uid"])),
                )
    def _write_minimal_inspection_export(self, import_dir):
        exporter.write_json_document(
            build_export_metadata(
                variant=exporter.RAW_EXPORT_SUBDIR,
                dashboard_count=1,
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
                    "parentUid": "",
                    "path": "Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                }
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
                    "url": "http://prometheus.local",
                    "isDefault": "false",
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
                            "id": 7,
                            "title": "CPU Usage",
                            "type": "timeseries",
                            "datasource": {"uid": "prom-main", "type": "prometheus"},
                            "targets": [
                                {
                                    "refId": "A",
                                    "expr": "sum(rate(node_cpu_seconds_total[5m]))",
                                    "datasource": {
                                        "uid": "prom-main",
                                        "type": "prometheus",
                                    },
                                }
                            ],
                        }
                    ],
                },
                "meta": {"folderUid": "infra"},
            },
            import_dir / "Infra" / "CPU_Main__cpu-main.json",
        )

    def test_dashboard_script_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_transport_module_parses_as_python39_syntax(self):
        source = TRANSPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(TRANSPORT_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_client_module_parses_as_python39_syntax(self):
        source = CLIENT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(CLIENT_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_common_module_parses_as_python39_syntax(self):
        source = COMMON_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(COMMON_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_export_workflow_module_parses_as_python39_syntax(self):
        source = EXPORT_WORKFLOW_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(EXPORT_WORKFLOW_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_export_inventory_module_parses_as_python39_syntax(self):
        source = EXPORT_INVENTORY_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(EXPORT_INVENTORY_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_folder_support_module_parses_as_python39_syntax(self):
        source = FOLDER_SUPPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(FOLDER_SUPPORT_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_import_support_module_parses_as_python39_syntax(self):
        source = IMPORT_SUPPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(IMPORT_SUPPORT_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_progress_module_parses_as_python39_syntax(self):
        source = PROGRESS_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(PROGRESS_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_import_workflow_module_parses_as_python39_syntax(self):
        source = IMPORT_WORKFLOW_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(IMPORT_WORKFLOW_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_inspection_workflow_module_parses_as_python39_syntax(self):
        source = INSPECTION_WORKFLOW_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_WORKFLOW_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_dispatch_module_parses_as_python39_syntax(self):
        source = INSPECTION_DISPATCH_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_DISPATCH_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzers_package_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZERS_PACKAGE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZERS_PACKAGE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_contract_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_CONTRACT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_CONTRACT_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_dispatcher_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_DISPATCHER_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_DISPATCHER_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_prometheus_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_PROMETHEUS_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_PROMETHEUS_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_flux_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_FLUX_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_FLUX_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_sql_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_SQL_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_SQL_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_analyzer_generic_module_parses_as_python39_syntax(self):
        source = INSPECTION_ANALYZER_GENERIC_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_ANALYZER_GENERIC_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_report_module_parses_as_python39_syntax(self):
        source = INSPECTION_REPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_REPORT_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_render_module_parses_as_python39_syntax(self):
        source = INSPECTION_RENDER_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_RENDER_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_inspection_summary_module_parses_as_python39_syntax(self):
        source = INSPECTION_SUMMARY_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(
            source,
            filename=str(INSPECTION_SUMMARY_MODULE_PATH),
            feature_version=(3, 9),
        )

    def test_dashboard_governance_gate_module_parses_as_python39_syntax(self):
        source = GOVERNANCE_GATE_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(GOVERNANCE_GATE_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_listing_module_parses_as_python39_syntax(self):
        source = LISTING_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(LISTING_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_output_support_module_parses_as_python39_syntax(self):
        source = OUTPUT_SUPPORT_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(OUTPUT_SUPPORT_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_progress_module_parses_as_python39_syntax(self):
        source = PROGRESS_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(PROGRESS_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_transformer_module_parses_as_python39_syntax(self):
        source = TRANSFORMER_MODULE_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(TRANSFORMER_MODULE_PATH), feature_version=(3, 9))

    def test_dashboard_module_entrypoint_parses_as_python39_syntax(self):
        source = MODULE_ENTRYPOINT_PATH.read_text(encoding="utf-8")

        ast.parse(source, filename=str(MODULE_ENTRYPOINT_PATH), feature_version=(3, 9))

    def test_dashboard_parse_args_requires_subcommand(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args([])

    def test_dashboard_top_level_help_includes_basic_and_token_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["-h"])

        help_text = stream.getvalue()
        self.assertIn("Export dashboards from local Grafana with Basic auth", help_text)
        self.assertIn("Export dashboards with an API token", help_text)
        self.assertIn("http://localhost:3000", help_text)

    def test_dashboard_export_help_includes_basic_and_token_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["export-dashboard", "-h"])

        help_text = stream.getvalue()
        self.assertIn("Export dashboards from local Grafana with Basic auth", help_text)
        self.assertIn("Export dashboards with an API token", help_text)
        self.assertIn("--basic-user admin --basic-password admin", help_text)

    def test_dashboard_import_help_explains_common_operator_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["import-dashboard", "-h"])

        help_text = stream.getvalue()
        self.assertIn("combined", help_text)
        self.assertIn("export root", help_text)
        self.assertIn("--org-id", help_text)
        self.assertIn("--use-export-org", help_text)
        self.assertIn("--only-org-id", help_text)
        self.assertIn("--create-missing-orgs", help_text)
        self.assertIn("API token auth is not supported here", help_text)
        self.assertIn("--require-matching-export-org", help_text)
        self.assertIn("missing/match/mismatch", help_text)
        self.assertIn("skipped/blocked", help_text)
        self.assertIn("table form", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("--approve", help_text)
        self.assertIn("Connection Options", help_text)
        self.assertIn("Auth Options", help_text)
        self.assertIn("Target Options", help_text)
        self.assertIn("Mutation Options", help_text)
        self.assertIn("Safety Options", help_text)
        self.assertIn("Output Options", help_text)

    def test_dashboard_list_help_includes_examples_and_grouped_sections(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["list-dashboard", "-h"])

        help_text = stream.getvalue()
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util dashboard list-dashboard", help_text)
        self.assertIn("Input Options", help_text)
        self.assertIn("Target Options", help_text)
        self.assertIn("Output Options", help_text)

    def test_dashboard_screenshot_help_includes_examples_and_grouped_sections(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["screenshot", "-h"])

        help_text = stream.getvalue()
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util dashboard screenshot", help_text)
        self.assertIn("Target Options", help_text)
        self.assertIn("State Options", help_text)
        self.assertIn("Rendering Options", help_text)
        self.assertIn("Output Options", help_text)
        self.assertIn("Header Options", help_text)

    def test_dashboard_inspect_export_help_mentions_raw_export_directory(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["inspect-export", "-h"])

        help_text = stream.getvalue()
        self.assertIn("raw/ export directory explicitly", help_text)
        self.assertIn("--output-format", help_text)
        self.assertIn("report-tree-table", help_text)
        self.assertIn("dependency", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util dashboard inspect-export", help_text)
        self.assertIn("--help-full", help_text)
        self.assertIn("datasourceType", help_text)
        self.assertIn("datasourceFamily", help_text)
        self.assertIn("datasource label, uid, type,", help_text)
        self.assertIn("or family exactly matches this value", help_text)
        self.assertNotIn("\n  --json", help_text)
        self.assertNotIn("\n  --table", help_text)
        self.assertNotIn("\n  --report ", help_text)
        self.assertNotIn("Extended examples:", help_text)

    def test_dashboard_inspect_export_help_full_includes_extended_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["inspect-export", "--help-full"])

        help_text = stream.getvalue()
        self.assertIn("raw/ export directory explicitly", help_text)
        self.assertIn("Extended examples:", help_text)
        self.assertIn("grafana-util dashboard inspect-export", help_text)
        self.assertIn("--report tree-table", help_text)
        self.assertIn("--report-filter-datasource prom-main", help_text)
        self.assertIn("--report-filter-panel-id 7", help_text)
        self.assertIn(
            "--report-columns dashboard_uid,datasource_uid,datasource_family,query,file",
            help_text,
        )
        self.assertNotIn("grafana-utils inspect-export", help_text)

    def test_dashboard_inspect_live_help_mentions_live_report_flags(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["inspect-live", "-h"])

        help_text = stream.getvalue()
        self.assertIn("--url", help_text)
        self.assertIn("--page-size", help_text)
        self.assertIn("--output-format", help_text)
        self.assertIn("tree-table", help_text)
        self.assertIn("tree", help_text)
        self.assertIn("dependency", help_text)
        self.assertIn("--report-filter-panel-id", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("grafana-util dashboard inspect-live", help_text)
        self.assertIn("--help-full", help_text)
        self.assertIn("datasourceType", help_text)
        self.assertIn("datasourceFamily", help_text)
        self.assertIn("datasource label, uid, type,", help_text)
        self.assertIn("or family exactly matches this value", help_text)
        self.assertNotIn("\n  --report ", help_text)
        self.assertNotIn("\n  --json", help_text)
        self.assertNotIn("\n  --table", help_text)
        self.assertNotIn("Extended examples:", help_text)

    def test_dashboard_inspect_live_help_full_includes_extended_examples(self):
        stream = io.StringIO()

        with redirect_stdout(stream):
            with self.assertRaises(SystemExit):
                exporter.parse_args(["inspect-live", "--help-full"])

        help_text = stream.getvalue()
        self.assertIn("--url", help_text)
        self.assertIn("Extended examples:", help_text)
        self.assertIn("grafana-util dashboard inspect-live", help_text)
        self.assertIn("--token \"$GRAFANA_API_TOKEN\"", help_text)
        self.assertIn("--report tree-table", help_text)
        self.assertIn("--report-filter-panel-id 7", help_text)
        self.assertIn(
            "--report-columns dashboard_uid,datasource_uid,datasource_family,query,file",
            help_text,
        )
        self.assertNotIn("grafana-utils inspect-live", help_text)

    def test_dashboard_parse_args_supports_import_mode(self):
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards"])

        self.assertEqual(args.import_dir, "dashboards")
        self.assertEqual(args.command, "import-dashboard")

    def test_dashboard_parse_args_supports_import_org_id(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--org-id", "2"]
        )

        self.assertEqual(args.org_id, "2")

    def test_dashboard_main_requires_approve_for_live_import(self):
        stream = io.StringIO()

        with redirect_stderr(stream):
            result = exporter.main(["import-dashboard", "--import-dir", "dashboards/raw"])

        self.assertEqual(result, 1)
        self.assertIn("requires --approve", stream.getvalue())

    def test_dashboard_parse_args_supports_require_matching_export_org(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--require-matching-export-org",
            ]
        )

        self.assertTrue(args.require_matching_export_org)

    def test_dashboard_parse_args_supports_import_by_export_org_flags(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards",
                "--use-export-org",
                "--only-org-id",
                "2",
                "--only-org-id",
                "5",
                "--create-missing-orgs",
            ]
        )

        self.assertTrue(args.use_export_org)
        self.assertEqual(args.only_org_id, ["2", "5"])
        self.assertTrue(args.create_missing_orgs)

    def test_dashboard_parse_args_rejects_only_org_id_without_use_export_org(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                ["import-dashboard", "--import-dir", "dashboards", "--only-org-id", "2"]
            )

    def test_dashboard_parse_args_rejects_create_missing_orgs_without_use_export_org(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    "dashboards",
                    "--create-missing-orgs",
                ]
            )

    def test_dashboard_parse_args_rejects_use_export_org_with_org_id(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    "dashboards",
                    "--use-export-org",
                    "--org-id",
                    "2",
                ]
            )

    def test_dashboard_parse_args_rejects_use_export_org_with_require_matching_export_org(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    "dashboards",
                    "--use-export-org",
                    "--require-matching-export-org",
                ]
            )

    def test_dashboard_parse_args_supports_create_missing_orgs_with_dry_run(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards",
                "--use-export-org",
                "--create-missing-orgs",
                "--dry-run",
            ]
        )

        self.assertTrue(args.use_export_org)
        self.assertTrue(args.create_missing_orgs)
        self.assertTrue(args.dry_run)

    def test_dashboard_parse_args_supports_use_export_org_with_json_output(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards",
                "--use-export-org",
                "--dry-run",
                "--json",
            ]
        )

        self.assertTrue(args.use_export_org)
        self.assertTrue(args.dry_run)
        self.assertTrue(args.json)

    def test_dashboard_parse_args_supports_preferred_auth_aliases(self):
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

    def test_dashboard_parse_args_rejects_legacy_basic_auth_aliases(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["export-dashboard", "--username", "user", "--basic-password", "pass"])
        with self.assertRaises(SystemExit):
            exporter.parse_args(["export-dashboard", "--basic-user", "user", "--password", "pass"])

    def test_dashboard_parse_args_supports_prompt_password(self):
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

    def test_dashboard_parse_args_supports_prompt_token(self):
        args = exporter.parse_args(["export-dashboard", "--prompt-token"])

        self.assertTrue(args.prompt_token)
        self.assertIsNone(args.api_token)

    def test_dashboard_parse_args_supports_list_mode(self):
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

    def test_dashboard_parse_args_supports_list_org_selection(self):
        org_args = exporter.parse_args(["list-dashboard", "--org-id", "2"])
        all_args = exporter.parse_args(["list-dashboard", "--all-orgs"])

        self.assertEqual(org_args.org_id, "2")
        self.assertFalse(org_args.all_orgs)
        self.assertTrue(all_args.all_orgs)
        self.assertIsNone(all_args.org_id)

    def test_dashboard_parse_args_supports_list_data_sources_mode(self):
        args = exporter.parse_args(["list-data-sources"])

        self.assertEqual(args.command, "list-data-sources")
        self.assertFalse(args.table)
        self.assertFalse(args.csv)
        self.assertFalse(args.json)
        self.assertFalse(args.no_header)

    def test_dashboard_parse_args_supports_list_output_format(self):
        list_args = exporter.parse_args(["list-dashboard", "--output-format", "csv"])
        data_source_args = exporter.parse_args(
            ["list-data-sources", "--output-format", "json"]
        )

        self.assertEqual(list_args.output_format, "csv")
        self.assertTrue(list_args.csv)
        self.assertFalse(list_args.table)
        self.assertTrue(data_source_args.json)
        self.assertFalse(data_source_args.csv)

    def test_dashboard_parse_args_supports_list_csv_and_json_modes(self):
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

    def test_dashboard_parse_args_supports_export_and_import_progress(self):
        export_args = exporter.parse_args(["export-dashboard", "--progress"])
        import_args = exporter.parse_args(["import-dashboard", "--import-dir", "./dashboards/raw", "--progress"])
        verbose_export_args = exporter.parse_args(["export-dashboard", "--verbose"])
        verbose_import_args = exporter.parse_args(["import-dashboard", "--import-dir", "./dashboards/raw", "--verbose"])

        self.assertTrue(export_args.progress)
        self.assertTrue(import_args.progress)
        self.assertTrue(verbose_export_args.verbose)
        self.assertTrue(verbose_import_args.verbose)

    def test_dashboard_parse_args_rejects_multiple_list_output_modes(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--csv", "--json"])

    def test_dashboard_parse_args_rejects_multiple_list_data_sources_output_modes(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--table", "--csv"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--csv", "--json"])

    def test_dashboard_parse_args_rejects_list_output_format_with_legacy_flags(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-dashboard", "--output-format", "table", "--json"])

        with self.assertRaises(SystemExit):
            exporter.parse_args(["list-data-sources", "--output-format", "csv", "--table"])

    def test_dashboard_parse_args_supports_diff_mode(self):
        args = exporter.parse_args(["diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(args.import_dir, "dashboards/raw")
        self.assertEqual(args.command, "diff")
        self.assertEqual(args.context_lines, 3)

    def test_dashboard_parse_args_defaults_export_dir_to_dashboards(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.export_dir, "dashboards")
        self.assertEqual(args.command, "export-dashboard")
        self.assertIsNone(args.org_id)
        self.assertFalse(args.all_orgs)

    def test_dashboard_parse_args_supports_export_org_selection(self):
        org_args = exporter.parse_args(["export-dashboard", "--org-id", "2"])
        all_args = exporter.parse_args(["export-dashboard", "--all-orgs"])

        self.assertEqual(org_args.org_id, "2")
        self.assertFalse(org_args.all_orgs)
        self.assertTrue(all_args.all_orgs)
        self.assertIsNone(all_args.org_id)

    def test_dashboard_parse_args_defaults_url_to_local_grafana(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertEqual(args.url, "http://localhost:3000")

    def test_dashboard_parse_args_supports_variant_switches(self):
        args = exporter.parse_args(
            ["export-dashboard", "--without-dashboard-raw", "--without-dashboard-prompt"]
        )

        self.assertTrue(args.without_dashboard_raw)
        self.assertTrue(args.without_dashboard_prompt)

    def test_dashboard_parse_args_supports_export_dry_run(self):
        args = exporter.parse_args(["export-dashboard", "--dry-run"])

        self.assertTrue(args.dry_run)

    def test_dashboard_parse_args_supports_import_dry_run(self):
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run"])

        self.assertTrue(args.dry_run)

    def test_dashboard_parse_args_supports_import_dry_run_table_flags(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--table", "--no-header"]
        )

        self.assertTrue(args.dry_run)
        self.assertTrue(args.table)
        self.assertTrue(args.no_header)

    def test_dashboard_parse_args_supports_import_dry_run_json(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--json"]
        )

        self.assertTrue(args.dry_run)
        self.assertTrue(args.json)

    def test_dashboard_parse_args_supports_import_dry_run_output_format(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--dry-run",
                "--output-format",
                "table",
            ]
        )

        self.assertEqual(args.output_format, "table")
        self.assertTrue(args.table)
        self.assertFalse(args.json)

    def test_dashboard_parse_args_supports_import_dry_run_output_columns(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--dry-run",
                "--table",
                "--output-columns",
                "uid,action,source_folder_path,destination_folder_path,reason",
            ]
        )

        self.assertEqual(
            args.output_columns,
            [
                "uid",
                "action",
                "sourceFolderPath",
                "destinationFolderPath",
                "reason",
            ],
        )

    def test_dashboard_parse_args_rejects_import_output_format_with_legacy_flags(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    "dashboards/raw",
                    "--output-format",
                    "json",
                    "--table",
                ]
            )

    def test_dashboard_parse_args_rejects_import_output_columns_without_table_output(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    "dashboards/raw",
                    "--dry-run",
                    "--output-columns",
                    "uid,action",
                ]
            )

    def test_dashboard_parse_args_supports_update_existing_only(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--update-existing-only"]
        )

        self.assertTrue(args.update_existing_only)

    def test_dashboard_parse_args_supports_inspect_export_json(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--json"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertTrue(args.json)

    def test_dashboard_parse_args_supports_inspect_export_table(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--table", "--no-header"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertTrue(args.table)
        self.assertTrue(args.no_header)

    def test_dashboard_parse_args_supports_inspect_export_output_format(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--output-format",
                "report-tree-table",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.output_format, "report-tree-table")
        self.assertIsNone(args.report)
        self.assertFalse(args.json)
        self.assertFalse(args.table)

    def test_dashboard_parse_args_supports_inspect_export_output_format_dependency(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--output-format",
                "report-dependency",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.output_format, "report-dependency")
        self.assertIsNone(args.report)
        self.assertFalse(args.json)
        self.assertFalse(args.table)

    def test_dashboard_parse_args_supports_inspect_live_report_json(self):
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--report",
                "json",
                "--report-filter-datasource",
                "prom-main",
                "--report-filter-panel-id",
                "7",
            ]
        )

        self.assertEqual(args.command, "inspect-live")
        self.assertEqual(args.url, "http://localhost:3000")
        self.assertEqual(args.report, "json")
        self.assertEqual(args.report_filter_datasource, "prom-main")
        self.assertEqual(args.report_filter_panel_id, "7")

    def test_dashboard_parse_args_supports_inspect_live_report_tree_table(self):
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--report",
                "tree-table",
            ]
        )

        self.assertEqual(args.command, "inspect-live")
        self.assertEqual(args.url, "http://localhost:3000")
        self.assertEqual(args.report, "tree-table")

    def test_dashboard_parse_args_supports_inspect_live_report_dependency(self):
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--report",
                "dependency",
            ]
        )

        self.assertEqual(args.command, "inspect-live")
        self.assertEqual(args.url, "http://localhost:3000")
        self.assertEqual(args.report, "dependency")

    def test_dashboard_parse_args_supports_inspect_live_output_format(self):
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--output-format",
                "governance-json",
            ]
        )

        self.assertEqual(args.command, "inspect-live")
        self.assertEqual(args.output_format, "governance-json")
        self.assertIsNone(args.report)

    def test_dashboard_parse_args_supports_inspect_live_output_format_dependency(self):
        args = exporter.parse_args(
            [
                "inspect-live",
                "--url",
                "http://localhost:3000",
                "--output-format",
                "report-dependency-json",
            ]
        )

        self.assertEqual(args.command, "inspect-live")
        self.assertEqual(args.output_format, "report-dependency-json")
        self.assertIsNone(args.report)

    def test_dashboard_parse_args_supports_inspect_export_report_table(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "table")

    def test_dashboard_parse_args_supports_inspect_export_report_json(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report", "json"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "json")

    def test_dashboard_parse_args_supports_inspect_export_report_csv(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report", "csv"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "csv")

    def test_dashboard_parse_args_supports_inspect_export_report_tree(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report", "tree"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "tree")

    def test_dashboard_parse_args_supports_inspect_export_report_tree_table(self):
        args = exporter.parse_args(
            ["inspect-export", "--import-dir", "dashboards/raw", "--report", "tree-table"]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "tree-table")

    def test_dashboard_parse_args_supports_inspect_export_report_dependency(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--report",
                "dependency",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "dependency")

    def test_dashboard_parse_args_supports_inspect_export_report_dependency_json(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--report",
                "dependency-json",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report, "dependency-json")

    def test_dashboard_parse_args_supports_inspect_export_report_columns_and_filter(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--report",
                "--report-columns",
                "dashboardUid,datasource,metrics",
                "--report-filter-datasource",
                "prom-main",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report_columns, "dashboardUid,datasource,metrics")
        self.assertEqual(args.report_filter_datasource, "prom-main")

    def test_dashboard_parse_args_supports_inspect_export_report_panel_filter(self):
        args = exporter.parse_args(
            [
                "inspect-export",
                "--import-dir",
                "dashboards/raw",
                "--report",
                "--report-filter-panel-id",
                "7",
            ]
        )

        self.assertEqual(args.command, "inspect-export")
        self.assertEqual(args.report_filter_panel_id, "7")

    def test_dashboard_parse_report_columns_accepts_snake_case_aliases(self):
        self.assertEqual(
            exporter.parse_report_columns(
                "dashboard_uid,panel_title,query_field,datasource_uid,datasource_type,datasource_family"
            ),
            [
                "dashboardUid",
                "panelTitle",
                "queryField",
                "datasourceUid",
                "datasourceType",
                "datasourceFamily",
            ],
        )

    def test_dashboard_dispatch_query_analysis_uses_prometheus_analyzer(self):
        analysis = inspection_dispatcher.dispatch_query_analysis(
            panel={"datasource": {"type": "prometheus", "uid": "prom-main"}},
            target={"datasource": {"type": "prometheus", "uid": "prom-main"}},
            query_field="expr",
            query_text='sum(rate(node_cpu_seconds_total{job="node"}[5m]))',
        )

        self.assertEqual(analysis["metrics"], ["node_cpu_seconds_total"])
        self.assertEqual(analysis["measurements"], [])
        self.assertEqual(analysis["buckets"], [])

    def test_dashboard_dispatch_query_analysis_uses_flux_analyzer(self):
        analysis = inspection_dispatcher.dispatch_query_analysis(
            panel={"datasource": {"type": "influxdb", "uid": "flux-main"}},
            target={"datasource": {"type": "influxdb", "uid": "flux-main"}},
            query_field="query",
            query_text='from(bucket: "ops") |> range(start: -1h) |> filter(fn: (r) => r._measurement == "cpu")',
        )

        self.assertEqual(analysis["metrics"], ["from", "range", "filter"])
        self.assertEqual(analysis["measurements"], ["cpu"])
        self.assertEqual(analysis["buckets"], ["ops"])

    def test_dashboard_dispatch_query_analysis_uses_sql_analyzer(self):
        analysis = inspection_dispatcher.dispatch_query_analysis(
            panel={"datasource": {"type": "postgres", "uid": "pg-main"}},
            target={"datasource": {"type": "postgres", "uid": "pg-main"}},
            query_field="rawSql",
            query_text="select count(*) from public.cpu_metrics where host = 'web-01'",
        )

        self.assertEqual(analysis["metrics"], ["select", "where"])
        self.assertEqual(analysis["measurements"], ["public.cpu_metrics"])
        self.assertEqual(analysis["buckets"], [])

    def test_dashboard_dispatch_query_analysis_uses_loki_analyzer_boundary(self):
        analysis = inspection_dispatcher.dispatch_query_analysis(
            panel={"datasource": {"type": "loki", "uid": "loki-main"}},
            target={"datasource": {"type": "loki", "uid": "loki-main"}},
            query_field="logql",
            query_text='sum by (job) (count_over_time({job="varlogs",app=~"api|web"} |= "error" | json [5m]))',
        )

        self.assertEqual(
            analysis["metrics"],
            ["sum", "count_over_time", "filter_eq", "json"],
        )
        self.assertEqual(
            analysis["measurements"],
            ['job="varlogs"', 'app=~"api|web"'],
        )
        self.assertEqual(analysis["buckets"], [])

    def test_dashboard_dispatch_query_analysis_uses_generic_fallback_analyzer(self):
        analysis = inspection_dispatcher.dispatch_query_analysis(
            panel={"datasource": {"type": "custom-plugin", "uid": "custom-main"}},
            target={"datasource": {"type": "custom-plugin", "uid": "custom-main"}},
            query_field="query",
            query_text='cpu_total{host="web-01"}',
        )

        self.assertEqual(analysis["metrics"], ["cpu_total"])
        self.assertEqual(analysis["measurements"], [])
        self.assertEqual(analysis["buckets"], [])

    def test_dashboard_render_export_inspection_tree_tables_uses_grouped_document(self):
        grouped_document = exporter.build_grouped_export_inspection_report_document(
            {
                "queries": [
                    {
                        "dashboardUid": "cpu-main",
                        "dashboardTitle": "CPU Main",
                        "folderPath": "Platform / Infra",
                        "panelId": "7",
                        "panelTitle": "CPU Usage",
                        "panelType": "timeseries",
                        "refId": "A",
                        "datasource": "prom-main",
                        "queryField": "expr",
                        "metrics": ["node_cpu_seconds_total"],
                        "measurements": [],
                        "buckets": [],
                        "query": "sum(rate(node_cpu_seconds_total[5m]))",
                        "file": "Infra/CPU_Main__cpu-main.json",
                    }
                ]
            }
        )

        lines = exporter.render_export_inspection_tree_tables(
            grouped_document,
            Path("dashboards/raw"),
            selected_columns=["dashboardUid", "panelTitle", "datasource", "query"],
        )

        output = "\n".join(lines)
        self.assertIn("Export inspection tree-table report: dashboards/raw", output)
        self.assertIn("# Dashboard sections", output)
        self.assertIn("[1] Dashboard cpu-main", output)
        self.assertIn(
            "DASHBOARD_UID  PANEL_TITLE  DATASOURCE  QUERY",
            output,
        )
        self.assertIn("cpu-main", output)
        self.assertIn("CPU Usage", output)
        self.assertIn("prom-main", output)

    def test_dashboard_parse_args_supports_ensure_folders(self):
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--ensure-folders"]
        )

        self.assertTrue(args.ensure_folders)

    def test_dashboard_parse_args_supports_require_matching_folder_path(self):
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--require-matching-folder-path",
            ]
        )

        self.assertTrue(args.require_matching_folder_path)

    def test_dashboard_describe_dashboard_import_mode(self):
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

    def test_dashboard_parse_args_disables_ssl_verification_by_default(self):
        args = exporter.parse_args(["export-dashboard"])

        self.assertFalse(args.verify_ssl)

    def test_dashboard_parse_args_can_enable_ssl_verification(self):
        args = exporter.parse_args(["export-dashboard", "--verify-ssl"])

        self.assertTrue(args.verify_ssl)

    def test_dashboard_parse_args_rejects_old_list_subcommand_name(self):
        with self.assertRaises(SystemExit):
            exporter.parse_args(["list", "--json"])

    def test_dashboard_build_json_http_transport_defaults_to_requests(self):
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

    def test_dashboard_build_json_http_transport_supports_httpx(self):
        if not transport_module.httpx_is_available():
            self.skipTest("httpx is not installed")
        transport = exporter.build_json_http_transport(
            base_url="http://127.0.0.1:3000",
            headers={},
            timeout=30,
            verify_ssl=False,
            transport_name="httpx",
        )

        self.assertEqual(type(transport).__name__, "HttpxJsonHttpTransport")

    def test_dashboard_http2_capability_helper_returns_boolean(self):
        self.assertIsInstance(transport_module.http2_is_available(), bool)

    def test_dashboard_client_accepts_injected_transport(self):
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

    def test_dashboard_resolve_auth_supports_token_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            prompt_token=False,
            username=None,
            password=None,
        )

        headers = exporter.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer abc123")

    def test_dashboard_resolve_auth_supports_basic_auth(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="user",
            password="pass",
            prompt_password=False,
        )

        headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:pass").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")

    def test_dashboard_resolve_auth_rejects_mixed_token_and_basic_auth(self):
        args = argparse.Namespace(
            api_token="abc123",
            prompt_token=False,
            username="user",
            password="pass",
            prompt_password=False,
        )

        with self.assertRaisesRegex(exporter.GrafanaError, "Choose either token auth"):
            exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_rejects_user_without_password(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="user",
            password=None,
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user and --basic-password or --prompt-password.",
        ):
            exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_rejects_password_without_user(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password="pass",
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Basic auth requires both --basic-user and --basic-password or --prompt-password.",
        ):
            exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_supports_prompt_password(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="user",
            password=None,
            prompt_password=True,
        )

        with mock.patch("grafana_utils.dashboard_cli.getpass.getpass", return_value="secret") as prompt:
            headers = exporter.resolve_auth(args)

        expected = base64.b64encode(b"user:secret").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")
        prompt.assert_called_once_with("Grafana Basic auth password: ")

    def test_dashboard_resolve_auth_supports_prompt_token(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=True,
            username=None,
            password=None,
            prompt_password=False,
        )

        with mock.patch("grafana_utils.dashboard_cli.getpass.getpass", return_value="token-secret") as prompt:
            headers = exporter.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer token-secret")
        prompt.assert_called_once_with("Grafana API token: ")

    def test_dashboard_resolve_auth_supports_env_token_auth(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=False,
        )

        with mock.patch.dict("os.environ", {"GRAFANA_API_TOKEN": "env-token"}, clear=True):
            headers = exporter.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer env-token")

    def test_dashboard_resolve_auth_rejects_partial_basic_auth_env(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=False,
        )

        with mock.patch.dict("os.environ", {"GRAFANA_USERNAME": "env-user"}, clear=True):
            with self.assertRaisesRegex(
                exporter.GrafanaError,
                "Basic auth requires both --basic-user and --basic-password or --prompt-password.",
            ):
                exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_rejects_prompt_without_username(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username=None,
            password=None,
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "--prompt-password requires --basic-user.",
        ):
            exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_rejects_prompt_with_explicit_password(self):
        args = argparse.Namespace(
            api_token=None,
            prompt_token=False,
            username="user",
            password="pass",
            prompt_password=True,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Choose either --basic-password or --prompt-password, not both.",
        ):
            exporter.resolve_auth(args)

    def test_dashboard_resolve_auth_rejects_explicit_and_prompt_token(self):
        args = argparse.Namespace(
            api_token="abc123",
            prompt_token=True,
            username=None,
            password=None,
            prompt_password=False,
        )

        with self.assertRaisesRegex(
            exporter.GrafanaError,
            "Choose either --token / --api-token or --prompt-token, not both.",
        ):
            exporter.resolve_auth(args)

    def test_dashboard_sanitize_path_component(self):
        self.assertEqual(exporter.sanitize_path_component(" Ops / CPU % "), "Ops_CPU")
        self.assertEqual(exporter.sanitize_path_component("..."), "untitled")

    def test_dashboard_build_output_path_keeps_folder_structure(self):
        path = build_output_path(
            Path("out"),
            {"folderTitle": "Infra Team", "title": "Cluster Health", "uid": "abc"},
            flat=False,
        )

        self.assertEqual(path, Path("out/Infra_Team/Cluster_Health__abc.json"))

    def test_dashboard_build_all_orgs_output_dir_uses_org_id_and_name(self):
        path = build_all_orgs_output_dir(
            Path("out"),
            {"id": 2, "name": "Ops Org"},
        )

        self.assertEqual(path, Path("out/org_2_Ops_Org"))

    def test_dashboard_build_export_variant_dirs(self):
        raw_dir, prompt_dir = build_export_variant_dirs(Path("dashboards"))

        self.assertEqual(raw_dir, Path("dashboards/raw"))
        self.assertEqual(prompt_dir, Path("dashboards/prompt"))

    def test_dashboard_iter_dashboard_summaries_paginates_and_deduplicates(self):
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

    def test_dashboard_format_dashboard_summary_line_uses_defaults(self):
        line = exporter.format_dashboard_summary_line({"uid": "abc"})

        self.assertEqual(
            line,
            "uid=abc name=dashboard folder=General folderUid=general path=General org=Main Org. orgId=1",
        )

    def test_dashboard_format_dashboard_summary_line_includes_sources_when_present(self):
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

    def test_dashboard_build_dashboard_summary_record_uses_shared_default_constants(self):
        record = exporter.build_dashboard_summary_record({})

        self.assertEqual(record["uid"], exporter.DEFAULT_UNKNOWN_UID)
        self.assertEqual(record["name"], exporter.DEFAULT_DASHBOARD_TITLE)
        self.assertEqual(record["folder"], exporter.DEFAULT_FOLDER_TITLE)
        self.assertEqual(record["folderUid"], exporter.DEFAULT_FOLDER_UID)
        self.assertEqual(record["org"], exporter.DEFAULT_ORG_NAME)
        self.assertEqual(record["orgId"], exporter.DEFAULT_ORG_ID)

    def test_dashboard_build_folder_path_joins_parents_and_title(self):
        path = exporter.build_folder_path(
            {
                "title": "Child",
                "parents": [{"title": "Root"}, {"title": "Team"}],
            },
            fallback_title="Child",
        )

        self.assertEqual(path, "Root / Team / Child")

    def test_dashboard_attach_dashboard_folder_paths_uses_folder_hierarchy(self):
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

    def test_dashboard_attach_dashboard_org_uses_current_org(self):
        client = FakeDashboardWorkflowClient(org={"id": 7, "name": "Ops Org"})

        summaries = exporter.attach_dashboard_org(
            client,
            [{"uid": "abc", "title": "CPU"}],
        )

        self.assertEqual(summaries[0]["orgName"], "Ops Org")
        self.assertEqual(summaries[0]["orgId"], "7")

    def test_dashboard_format_data_source_line_uses_expected_fields(self):
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

    def test_dashboard_render_data_source_table_uses_headers_and_values(self):
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

    def test_dashboard_render_data_source_table_can_omit_header(self):
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

    def test_dashboard_render_data_source_csv_uses_expected_fields(self):
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

    def test_dashboard_render_data_source_json_uses_expected_fields(self):
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

    def test_dashboard_inspect_export_table_can_omit_header(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
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

    def test_dashboard_inspect_export_json_renders_structured_summary_document(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self._write_minimal_inspection_export(import_dir)

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--json"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["summary"]["dashboardCount"], 1)
            self.assertEqual(payload["summary"]["queryCount"], 1)
            self.assertEqual(payload["summary"]["datasourceInventoryCount"], 1)
            self.assertEqual(payload["dashboards"][0]["uid"], "cpu-main")
            self.assertEqual(payload["dashboards"][0]["folderPath"], "Infra")
            self.assertEqual(payload["datasources"][0]["name"], "prom-main")
            self.assertEqual(payload["datasourceInventory"][0]["uid"], "prom-main")

    def test_dashboard_inspect_export_report_json_renders_per_query_document(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            self._write_minimal_inspection_export(import_dir)

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "json"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["summary"]["dashboardCount"], 1)
            self.assertEqual(payload["summary"]["queryRecordCount"], 1)
            self.assertEqual(len(payload["queries"]), 1)
            self.assertEqual(payload["queries"][0]["dashboardUid"], "cpu-main")
            self.assertEqual(payload["queries"][0]["panelId"], "7")
            self.assertEqual(payload["queries"][0]["queryField"], "expr")
            self.assertEqual(payload["queries"][0]["datasource"], "prom-main")
            self.assertEqual(
                payload["queries"][0]["metrics"],
                ["node_cpu_seconds_total"],
            )

    def test_dashboard_inspect_export_report_json_uses_family_specific_query_analysis(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
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
                        "parentUid": "",
                        "path": "Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
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
                        "url": "http://prometheus.local",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "influx-main",
                        "name": "Influx Main",
                        "type": "influxdb",
                        "access": "proxy",
                        "url": "http://influx.local",
                        "isDefault": "false",
                        "org": "Main Org.",
                        "orgId": "1",
                    },
                    {
                        "uid": "mysql-main",
                        "name": "MySQL Main",
                        "type": "mysql",
                        "access": "proxy",
                        "url": "http://mysql.local",
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
                            {
                                "id": 8_1,
                                "title": "Loki Query",
                                "type": "logs",
                                "datasource": {"type": "loki", "uid": "loki-main"},
                                "targets": [
                                    {
                                        "refId": "B2",
                                        "logql": 'sum by (job) (count_over_time({job="varlogs",app=~"api|web"} |= "error" | json [5m]))',
                                    }
                                ],
                            },
                            {
                                "id": 9,
                                "title": "SQL Query",
                                "type": "table",
                                "datasource": {"type": "mysql", "uid": "mysql-main"},
                                "targets": [
                                    {
                                        "refId": "C",
                                        "rawSql": "select count(*) from metrics.cpu where host = 'node-1'",
                                    }
                                ],
                            },
                        ],
                    },
                    "meta": {"folderUid": "infra"},
                },
                import_dir / "Infra" / "Infra_Main__infra-main.json",
            )

            args = exporter.parse_args(
                ["inspect-export", "--import-dir", str(import_dir), "--report", "json"]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["queries"][0]["metrics"], ["node_cpu_seconds_total"])
            self.assertEqual(payload["queries"][1]["metrics"], ["from", "filter"])
            self.assertEqual(payload["queries"][1]["measurements"], ["cpu"])
            self.assertEqual(payload["queries"][1]["buckets"], ["prod"])
            self.assertEqual(
                payload["queries"][2]["metrics"],
                ["sum", "count_over_time", "filter_eq", "json"],
            )
            self.assertEqual(
                payload["queries"][2]["measurements"],
                ['job="varlogs"', 'app=~"api|web"'],
            )
            self.assertEqual(payload["queries"][2]["buckets"], [])
            self.assertEqual(payload["queries"][3]["metrics"], ["select", "where"])
            self.assertEqual(payload["queries"][3]["measurements"], ["metrics.cpu"])
            self.assertEqual(payload["queries"][3]["buckets"], [])

    def test_dashboard_inspect_export_report_tree_table_renders_loki_analysis_columns(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                    datasources_file=exporter.DATASOURCE_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "logs",
                        "title": "Logs",
                        "parentUid": "",
                        "path": "Logs",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "loki-main",
                        "name": "Loki Main",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki.local",
                        "isDefault": "false",
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
                        "uid": "logs-main",
                        "title": "Logs Main",
                        "panels": [
                            {
                                "id": 11,
                                "title": "Errors",
                                "type": "logs",
                                "datasource": {"type": "loki", "uid": "loki-main"},
                                "targets": [
                                    {
                                        "refId": "A",
                                        "expr": 'sum by (job) (count_over_time({job="varlogs",app=~"api|web"} |= "error" | json [5m]))',
                                        "datasource": {
                                            "type": "loki",
                                            "uid": "loki-main",
                                        },
                                    }
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "logs"},
                },
                import_dir / "Logs" / "Logs_Main__logs-main.json",
            )

            args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "tree-table",
                    "--report-columns",
                    "panel_id,datasource,metrics,measurements,buckets,query",
                ]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("Export inspection tree-table report: %s" % import_dir, output)
            self.assertIn("PANEL_ID  DATASOURCE  METRICS", output)
            self.assertIn("11", output)
            self.assertIn("loki-main", output)
            self.assertIn("sum,count_over_time,filter_eq,json", output)
            self.assertIn('job="varlogs",app=~"api|web"', output)
            self.assertIn("5m", output)
            self.assertIn('{job="varlogs",app=~"api|web"} |= "error" | json [5m]', output)

    def test_dashboard_inspect_export_report_table_renders_loki_analysis_columns(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                    folders_file=exporter.FOLDER_INVENTORY_FILENAME,
                    datasources_file=exporter.DATASOURCE_INVENTORY_FILENAME,
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "logs",
                        "title": "Logs",
                        "parentUid": "",
                        "path": "Logs",
                        "org": "Main Org.",
                        "orgId": "1",
                    }
                ],
                import_dir / exporter.FOLDER_INVENTORY_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "loki-main",
                        "name": "Loki Main",
                        "type": "loki",
                        "access": "proxy",
                        "url": "http://loki.local",
                        "isDefault": "false",
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
                        "uid": "logs-main",
                        "title": "Logs Main",
                        "panels": [
                            {
                                "id": 11,
                                "title": "Errors",
                                "type": "logs",
                                "datasource": {"type": "loki", "uid": "loki-main"},
                                "targets": [
                                    {
                                        "refId": "A",
                                        "expr": 'sum by (job) (count_over_time({job="varlogs",app=~"api|web"} |= "error" | json [5m]))',
                                        "datasource": {
                                            "type": "loki",
                                            "uid": "loki-main",
                                        },
                                    }
                                ],
                            }
                        ],
                    },
                    "meta": {"folderUid": "logs"},
                },
                import_dir / "Logs" / "Logs_Main__logs-main.json",
            )

            args = exporter.parse_args(
                [
                    "inspect-export",
                    "--import-dir",
                    str(import_dir),
                    "--report",
                    "table",
                    "--report-columns",
                    "panel_id,datasource,metrics,measurements,buckets,query",
                ]
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = exporter.inspect_export(args)

            output = stdout.getvalue()
            self.assertEqual(result, 0)
            self.assertIn("Export inspection report: %s" % import_dir, output)
            self.assertIn("PANEL_ID  DATASOURCE  METRICS", output)
            self.assertIn("11", output)
            self.assertIn("loki-main", output)
            self.assertIn("sum,count_over_time,filter_eq,json", output)
            self.assertIn('job="varlogs",app=~"api|web"', output)
            self.assertIn("5m", output)
            self.assertIn('{job="varlogs",app=~"api|web"} |= "error" | json [5m]', output)

    def test_dashboard_render_dashboard_summary_table_uses_headers_and_defaults(self):
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

    def test_dashboard_render_dashboard_summary_table_can_omit_header(self):
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

    def test_dashboard_render_dashboard_summary_table_includes_sources_column(self):
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

    def test_dashboard_render_dashboard_summary_json_uses_expected_fields(self):
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

    def test_dashboard_render_dashboard_summary_json_includes_sources_when_present(self):
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

    def test_dashboard_render_dashboard_summary_csv_includes_sources_column(self):
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

    def test_dashboard_attach_dashboard_sources_resolves_datasource_names(self):
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

        summaries = attach_dashboard_sources(
            client,
            [{"uid": "abc", "title": "CPU"}],
        )

        self.assertEqual(summaries[0]["sources"], ["Loki Logs", "Prometheus Main"])
        self.assertEqual(summaries[0]["sourceUids"], ["loki_uid", "prom_uid"])

    def test_dashboard_list_dashboards_prints_table_by_default(self):
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

    def test_dashboard_list_dashboards_no_header_hides_table_header(self):
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

    def test_dashboard_list_dashboards_prints_csv_when_requested(self):
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

    def test_dashboard_list_dashboards_prints_json_with_sources_by_default(self):
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
                },
                "xyz": {
                    "dashboard": {
                        "uid": "xyz",
                        "title": "Overview",
                        "panels": [],
                    }
                },
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
                    "sources": ["Loki Logs", "Prometheus Main"],
                    "sourceUids": ["loki_uid", "prom_uid"],
                },
                {
                    "uid": "xyz",
                    "name": "Overview",
                    "folder": "General",
                    "folderUid": "general",
                    "path": "General",
                    "org": "Main Org.",
                    "orgId": "1",
                    "sources": [],
                    "sourceUids": [],
                },
            ],
        )

    def test_dashboard_list_dashboards_with_sources_includes_resolved_datasource_names(self):
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

    def test_dashboard_list_dashboards_with_org_id_uses_scoped_client(self):
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

    def test_dashboard_list_dashboards_with_all_orgs_aggregates_results(self):
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

    def test_dashboard_list_dashboards_rejects_all_orgs_with_org_id(self):
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

    def test_dashboard_list_dashboards_rejects_org_switch_with_token_auth(self):
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

    def test_dashboard_list_data_sources_prints_table_by_default(self):
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

    def test_dashboard_list_data_sources_no_header_hides_table_header(self):
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

    def test_dashboard_write_dashboard_obeys_overwrite_flag(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "dash.json"
            write_dashboard({"dashboard": {"uid": "x"}}, path, overwrite=False)
            with self.assertRaises(exporter.GrafanaError):
                write_dashboard({"dashboard": {"uid": "x"}}, path, overwrite=False)

    def test_dashboard_discover_dashboard_files_ignores_index_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "index.json").write_text("[]", encoding="utf-8")
            dashboard_path = root / "team" / "dash.json"
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)
            dashboard_path.write_text('{"dashboard": {"uid": "x"}}', encoding="utf-8")

            files = discover_dashboard_files(root)

            self.assertEqual(files, [dashboard_path])

    def test_dashboard_discover_dashboard_files_ignores_export_metadata(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / exporter.EXPORT_METADATA_FILENAME).write_text("{}", encoding="utf-8")
            dashboard_path = root / "team" / "dash.json"
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)
            dashboard_path.write_text('{"dashboard": {"uid": "x"}}', encoding="utf-8")

            files = discover_dashboard_files(root)

            self.assertEqual(files, [dashboard_path])

    def test_dashboard_discover_dashboard_files_ignores_folder_inventory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / exporter.FOLDER_INVENTORY_FILENAME).write_text("[]", encoding="utf-8")
            dashboard_path = root / "team" / "dash.json"
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)
            dashboard_path.write_text('{"dashboard": {"uid": "x"}}', encoding="utf-8")

            files = discover_dashboard_files(root)

            self.assertEqual(files, [dashboard_path])

    def test_dashboard_discover_dashboard_files_rejects_combined_export_root(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "raw").mkdir()
            (root / "prompt").mkdir()

            with self.assertRaises(exporter.GrafanaError):
                discover_dashboard_files(root)

    def test_dashboard_export_dashboards_rejects_disabling_all_variants(self):
        args = exporter.parse_args(
            ["export-dashboard", "--without-dashboard-raw", "--without-dashboard-prompt"]
        )

        with self.assertRaises(exporter.GrafanaError):
            exporter.export_dashboards(args)

    def test_dashboard_validate_export_metadata_rejects_unsupported_schema_version(self):
        metadata = build_export_metadata(
            variant=exporter.RAW_EXPORT_SUBDIR,
            dashboard_count=1,
        )
        metadata["schemaVersion"] = exporter.TOOL_SCHEMA_VERSION + 1

        with self.assertRaises(exporter.GrafanaError):
            validate_export_metadata(
                metadata,
                metadata_path=Path("/tmp/export-metadata.json"),
                expected_variant=exporter.RAW_EXPORT_SUBDIR,
            )

    def test_dashboard_build_import_payload_uses_export_wrapper_and_override(self):
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

    def test_dashboard_build_import_payload_accepts_top_level_dashboard_document(self):
        payload = exporter.build_import_payload(
            document={"id": 7, "uid": "abc", "title": "CPU"},
            folder_uid_override=None,
            replace_existing=False,
            message="sync dashboards",
        )

        self.assertEqual(payload["dashboard"]["id"], None)
        self.assertEqual(payload["dashboard"]["uid"], "abc")
        self.assertEqual(payload["dashboard"]["title"], "CPU")

    def test_dashboard_collect_folder_inventory_includes_parent_chain(self):
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

    def test_dashboard_load_folder_inventory_reads_exported_manifest(self):
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

            records = load_folder_inventory(import_dir)

        self.assertEqual(records[0]["uid"], "child")
        self.assertEqual(records[0]["parentUid"], "parent")

    def test_dashboard_load_datasource_inventory_reads_exported_manifest(self):
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

            records = load_datasource_inventory(import_dir)

        self.assertEqual(records[0]["uid"], "prom")
        self.assertEqual(records[0]["access"], "proxy")

    def test_dashboard_ensure_folder_inventory_creates_missing_folders_in_order(self):
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

    def test_dashboard_inspect_folder_inventory_reports_missing_and_mismatch(self):
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

    def test_dashboard_resolve_folder_inventory_record_for_dashboard_uses_relative_path_without_meta(self):
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

    def test_dashboard_resolve_folder_inventory_record_for_dashboard_marks_general_as_builtin(self):
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

    def test_dashboard_resolve_folder_inventory_record_for_dashboard_uses_unique_folder_title_fallback(self):
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

    def test_dashboard_render_folder_inventory_dry_run_table_renders_rows(self):
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

    def test_dashboard_export_dashboards_writes_versioned_manifest_files(self):
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

    def test_dashboard_export_dashboards_progress_is_opt_in(self):
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

    def test_dashboard_export_dashboards_verbose_prints_paths(self):
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

    def test_dashboard_export_dashboards_verbose_supersedes_progress_output(self):
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

    def test_dashboard_export_dashboards_dry_run_keeps_directory_empty(self):
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

    def test_dashboard_export_dashboards_with_org_id_uses_scoped_client(self):
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

    def test_dashboard_export_dashboards_with_all_orgs_uses_org_prefix_dirs(self):
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

    def test_dashboard_export_dashboards_rejects_all_orgs_with_org_id(self):
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

    def test_dashboard_export_dashboards_rejects_org_switch_with_token_auth(self):
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
            with self.assertRaisesRegex(exporter.GrafanaError, "does not support API token auth"):
                exporter.export_dashboards(args)

    def test_dashboard_import_dashboards_rejects_org_switch_with_token_auth(self):
        client = FakeDashboardWorkflowClient(headers={"Authorization": "Bearer token"})
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--org-id",
                "2",
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "does not support API token auth"):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_by_export_org_rejects_token_auth(self):
        client = FakeDashboardWorkflowClient(headers={"Authorization": "Bearer token"})
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards",
                "--use-export-org",
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(
                exporter.GrafanaError,
                "does not support API token auth",
            ):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_by_export_org_dry_run_reports_missing_destination_org_without_create(self):
        org_one_client = FakeDashboardWorkflowClient(
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}],
            org_clients={"1": org_one_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "2",
                        "org_name": "Org Two",
                        "dashboards": [{"uid": "abc", "title": "CPU"}],
                    }
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.created_orgs, [])
            self.assertIn("orgAction=missing-org", stdout.getvalue())
            self.assertIn("dashboards=1", stdout.getvalue())
            self.assertEqual(org_one_client.imported_payloads, [])

    def test_dashboard_import_dashboards_by_export_org_dry_run_filters_selected_orgs(self):
        org_one_client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            },
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        org_two_client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}, {"id": 2, "name": "Org Two"}],
            org_clients={"1": org_one_client, "2": org_two_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "1",
                        "org_name": "Main Org.",
                        "dashboards": [{"uid": "abc", "title": "CPU"}],
                    },
                    {
                        "org_id": "2",
                        "org_name": "Org Two",
                        "dashboards": [{"uid": "xyz", "title": "Memory"}],
                    },
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--only-org-id",
                    "2",
                    "--dry-run",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(org_one_client.imported_payloads, [])
            self.assertEqual(org_two_client.imported_payloads, [])
            self.assertIn("Dry-run export orgId=2", stdout.getvalue())
            self.assertIn("Memory__xyz.json", stdout.getvalue())
            self.assertNotIn("CPU__abc.json", stdout.getvalue())

    def test_dashboard_import_dashboards_by_export_org_dry_run_json_reports_orgs_and_dashboards(self):
        org_one_client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            },
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}],
            org_clients={"1": org_one_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "1",
                        "org_name": "Main Org.",
                        "dashboards": [{"uid": "abc", "title": "CPU"}],
                    },
                    {
                        "org_id": "9",
                        "org_name": "Ops Org",
                        "dashboards": [{"uid": "ops", "title": "Ops"}],
                    },
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--create-missing-orgs",
                    "--dry-run",
                    "--json",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            payload = json.loads(stdout.getvalue())
            self.assertEqual(payload["mode"], "routed-import-preview")
            self.assertEqual(len(payload["orgs"]), 2)
            self.assertEqual(payload["orgs"][0]["orgAction"], "exists")
            self.assertEqual(payload["orgs"][1]["orgAction"], "would-create-org")
            self.assertEqual(payload["imports"][0]["dashboards"][0]["uid"], "abc")
            self.assertEqual(payload["imports"][1]["dashboards"], [])

    def test_dashboard_import_dashboards_by_export_org_dry_run_table_prints_org_summary_table(self):
        org_one_client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            },
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}],
            org_clients={"1": org_one_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "1",
                        "org_name": "Main Org.",
                        "dashboards": [{"uid": "abc", "title": "CPU"}],
                    },
                    {
                        "org_id": "9",
                        "org_name": "Ops Org",
                        "dashboards": [{"uid": "ops", "title": "Ops"}],
                    },
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--create-missing-orgs",
                    "--dry-run",
                    "--table",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            text = stdout.getvalue()
            self.assertIn("SOURCE_ORG_ID", text)
            self.assertIn("ORG_ACTION", text)
            self.assertIn("would-create-org", text)
            self.assertIn("Main Org.", text)
            self.assertNotIn("Import mode:", text)
            self.assertNotIn("Dry-run export orgId=", text)

    def test_dashboard_import_dashboards_by_export_org_rejects_unknown_selected_org(self):
        org_two_client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 2, "name": "Org Two"}],
            org_clients={"2": org_two_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "2",
                        "org_name": "Org Two",
                        "dashboards": [{"uid": "xyz", "title": "Memory"}],
                    }
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--only-org-id",
                    "5",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    exporter.GrafanaError,
                    "Selected export orgIds were not found",
                ):
                    exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_by_export_org_creates_missing_org_and_remaps_target(self):
        org_one_client = FakeDashboardWorkflowClient(
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}],
            org_clients={"1": org_one_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "9",
                        "org_name": "Ops Org",
                        "dashboards": [{"uid": "ops", "title": "Ops"}],
                    }
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--create-missing-orgs",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(len(client.created_orgs), 1)
            self.assertEqual(client.created_orgs[0]["name"], "Ops Org")
            self.assertEqual(client.created_orgs[0]["orgId"], 2)
            self.assertIn("targetOrgId=2", stdout.getvalue())
            self.assertEqual(len(client.org_clients["2"].imported_payloads), 1)
            self.assertEqual(
                client.org_clients["2"].imported_payloads[0]["dashboard"]["uid"],
                "ops",
            )

    def test_dashboard_import_dashboards_by_export_org_dry_run_reports_would_create_org(self):
        org_one_client = FakeDashboardWorkflowClient(
            org={"id": 1, "name": "Main Org."},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            orgs=[{"id": 1, "name": "Main Org."}],
            org_clients={"1": org_one_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            root_dir = Path(tmpdir)
            self._write_multi_org_import_root(
                root_dir,
                [
                    {
                        "org_id": "9",
                        "org_name": "Ops Org",
                        "dashboards": [{"uid": "ops", "title": "Ops"}],
                    }
                ],
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(root_dir),
                    "--use-export-org",
                    "--create-missing-orgs",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.created_orgs, [])
            self.assertIn("orgAction=would-create-org", stdout.getvalue())
            self.assertIn("targetOrgId=<new>", stdout.getvalue())
            self.assertIn("dashboards=1", stdout.getvalue())
            self.assertEqual(org_one_client.imported_payloads, [])

    def test_dashboard_import_dashboards_rejects_export_org_mismatch_for_token_scope(self):
        client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Bearer token"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--require-matching-export-org",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    exporter.GrafanaError,
                    "Raw export orgId 1 does not match target Grafana org id 2",
                ):
                    exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_accepts_matching_export_org_for_token_scope(self):
        client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Bearer token"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "abc",
                        "title": "CPU",
                        "folder": "General",
                        "org": "Org Two",
                        "orgId": "2",
                        "path": "cpu__abc.json",
                        "format": "grafana-web-import-preserve-uid",
                    }
                ],
                import_dir / "index.json",
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
                    "--require-matching-export-org",
                    "--dry-run",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertIn("Import mode: create-only", stdout.getvalue())

    def test_dashboard_import_dashboards_rejects_export_org_mismatch_for_org_id_scope(self):
        scoped_client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            org_clients={"2": scoped_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=1,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "abc",
                        "title": "CPU",
                        "folder": "General",
                        "org": "Main Org.",
                        "orgId": "1",
                        "path": "cpu__abc.json",
                        "format": "grafana-web-import-preserve-uid",
                    }
                ],
                import_dir / "index.json",
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
                    "--org-id",
                    "2",
                    "--require-matching-export-org",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    exporter.GrafanaError,
                    "Raw export orgId 1 does not match target Grafana org id 2",
                ):
                    exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_rejects_multi_org_export_metadata_when_guard_is_enabled(self):
        client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Bearer token"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
                    variant=exporter.RAW_EXPORT_SUBDIR,
                    dashboard_count=2,
                    format_name="grafana-web-import-preserve-uid",
                ),
                import_dir / exporter.EXPORT_METADATA_FILENAME,
            )
            exporter.write_json_document(
                [
                    {
                        "uid": "abc",
                        "title": "CPU",
                        "folder": "General",
                        "org": "Main Org.",
                        "orgId": "1",
                        "path": "cpu__abc.json",
                        "format": "grafana-web-import-preserve-uid",
                    },
                    {
                        "uid": "xyz",
                        "title": "Memory",
                        "folder": "General",
                        "org": "Org Two",
                        "orgId": "2",
                        "path": "memory__xyz.json",
                        "format": "grafana-web-import-preserve-uid",
                    },
                ],
                import_dir / "index.json",
            )
            exporter.write_json_document(
                {"dashboard": {"id": None, "uid": "abc", "title": "CPU", "panels": []}},
                import_dir / "cpu__abc.json",
            )
            exporter.write_json_document(
                {
                    "dashboard": {
                        "id": None,
                        "uid": "xyz",
                        "title": "Memory",
                        "panels": [],
                    }
                },
                import_dir / "memory__xyz.json",
            )
            args = exporter.parse_args(
                [
                    "import-dashboard",
                    "--import-dir",
                    str(import_dir),
                    "--require-matching-export-org",
                    "--dry-run",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                with self.assertRaisesRegex(
                    exporter.GrafanaError,
                    "spans multiple orgId values",
                ):
                    exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_dry_run_with_org_id_uses_scoped_client(self):
        scoped_client = FakeDashboardWorkflowClient(
            dashboards={
                "abc": {
                    "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
                    "meta": {"folderUid": "infra"},
                }
            },
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            org_clients={"2": scoped_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                    "--org-id",
                    "2",
                    "--dry-run",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.imported_payloads, [])
            self.assertEqual(scoped_client.imported_payloads, [])
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-only",
                    "Dry-run import uid=abc dest=exists action=blocked-existing folderPath=General file=%s"
                    % (import_dir / "cpu__abc.json"),
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_dashboard_import_dashboards_live_with_org_id_uses_scoped_client(self):
        scoped_client = FakeDashboardWorkflowClient(
            org={"id": 2, "name": "Org Two"},
            headers={"Authorization": "Basic test"},
        )
        client = FakeDashboardWorkflowClient(
            org_clients={"2": scoped_client},
            headers={"Authorization": "Basic test"},
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                    "--org-id",
                    "2",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.imported_payloads, [])
            self.assertEqual(len(scoped_client.imported_payloads), 1)
            self.assertEqual(
                scoped_client.imported_payloads[0]["dashboard"]["uid"],
                "abc",
            )

    def test_dashboard_import_dashboards_dry_run_skips_api_write(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_verbose_reports_destination_state(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_progress_reports_destination_state(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_ensure_folders_verbose_reports_folder_status(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_table_ensure_folders_includes_folder_status(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_table_renders_rows(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_table_can_omit_header(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                    "xyz  missing      create  General      General             %s"
                    % (import_dir / "memory__xyz.json"),
                    "Dry-run checked 1 dashboard files from %s" % import_dir,
                ],
            )

    def test_dashboard_import_dashboards_dry_run_table_marks_missing_dashboards_as_skipped_when_update_existing_only(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_update_existing_only_skips_missing_live_dashboards(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_update_existing_only_progress_shows_skips(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_replace_existing_preserves_destination_folder(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_table_uses_destination_folder_path_for_updates(self):
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
                build_export_metadata(
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
            self.assertIn("DESTINATION_FOLDER_PATH", output)

    def test_dashboard_import_dashboards_dry_run_table_includes_folder_match_reason_and_paths(self):
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
                    "title": "Legacy Infra",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                }
            },
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                        "parentUid": "platform",
                        "path": "Platform / Infra",
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
                    "--replace-existing",
                    "--require-matching-folder-path",
                    "--table",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("SOURCE_FOLDER_PATH", output)
            self.assertIn("DESTINATION_FOLDER_PATH", output)
            self.assertIn("REASON", output)
            self.assertIn("Platform / Infra", output)
            self.assertIn("Platform / Legacy Infra", output)
            self.assertIn("folder-path-mismatch", output)

    def test_dashboard_import_dashboards_dry_run_table_output_columns_limits_rendered_fields(self):
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
                    "title": "Legacy Infra",
                    "parents": [{"uid": "platform", "title": "Platform"}],
                }
            },
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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
                        "parentUid": "platform",
                        "path": "Platform / Infra",
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
                    "--replace-existing",
                    "--require-matching-folder-path",
                    "--table",
                    "--output-columns",
                    "uid,action,reason",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("UID", output)
            self.assertIn("ACTION", output)
            self.assertIn("REASON", output)
            self.assertNotIn("SOURCE_FOLDER_PATH", output)
            self.assertNotIn("DESTINATION_FOLDER_PATH", output)
            self.assertNotIn("FILE", output)

    def test_dashboard_import_dashboards_rejects_matching_folder_path_with_import_folder_uid(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(
            [
                "import-dashboard",
                "--import-dir",
                "dashboards/raw",
                "--require-matching-folder-path",
                "--import-folder-uid",
                "dest-folder",
            ]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(
                exporter.GrafanaError,
                "--require-matching-folder-path cannot be combined with --import-folder-uid",
            ):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_dry_run_matching_folder_path_marks_mismatch_as_skipped(self):
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
                build_export_metadata(
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
                    "--require-matching-folder-path",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("skip-folder-mismatch", output)
            self.assertIn("SOURCE_FOLDER_PATH", output)
            self.assertIn("DESTINATION_FOLDER_PATH", output)
            self.assertIn("General", output)
            self.assertIn("Platform / Ops", output)
            self.assertIn(
                "Dry-run checked 1 dashboard files from %s; would skip 1 folder-mismatched dashboards"
                % import_dir,
                output,
            )

    def test_dashboard_import_dashboards_matching_folder_path_skips_live_update_mismatch(self):
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
                build_export_metadata(
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
                    "--replace-existing",
                    "--require-matching-folder-path",
                    "--verbose",
                ]
            )

            with mock.patch.object(exporter, "build_client", return_value=client):
                stdout = io.StringIO()
                with redirect_stdout(stdout):
                    result = exporter.import_dashboards(args)

            self.assertEqual(result, 0)
            self.assertEqual(client.imported_payloads, [])
            self.assertEqual(
                stdout.getvalue().splitlines(),
                [
                    "Import mode: create-or-update",
                    "Skipped import uid=abc dest=exists action=skip-folder-mismatch sourceFolderPath=General destinationFolderPath=Platform / Ops file=%s"
                    % (import_dir / "cpu__abc.json"),
                    "Imported 0 dashboard files from %s; skipped 1 folder-mismatched dashboards"
                    % import_dir,
                ],
            )

    def test_dashboard_import_dashboards_rejects_table_without_dry_run(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--table"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--table is only supported with --dry-run"):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_rejects_json_without_dry_run(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--json"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--json is only supported with --dry-run"):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_rejects_table_with_json(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(
            ["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--table", "--json"]
        )

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--table and --json are mutually exclusive"):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_rejects_no_header_without_table(self):
        client = FakeDashboardWorkflowClient()
        args = exporter.parse_args(["import-dashboard", "--import-dir", "dashboards/raw", "--dry-run", "--no-header"])

        with mock.patch.object(exporter, "build_client", return_value=client):
            with self.assertRaisesRegex(exporter.GrafanaError, "--no-header is only supported with --dry-run --table"):
                exporter.import_dashboards(args)

    def test_dashboard_import_dashboards_ensure_folders_creates_missing_folders_from_inventory(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_ensure_folders_requires_inventory_manifest(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_ensure_folders_reports_folder_status(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_ensure_folders_table_renders_folder_table(self):
        client = FakeDashboardWorkflowClient(
            folders={
                "parent": {"uid": "parent", "title": "Platform", "parents": []},
            }
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_table_marks_general_folder_as_default(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_dry_run_json_renders_structured_output(self):
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
                build_export_metadata(
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

    def test_dashboard_import_dashboards_progress_is_opt_in(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_verbose_prints_paths(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_verbose_supersedes_progress_output(self):
        client = FakeDashboardWorkflowClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_import_dashboards_rejects_unsupported_manifest_schema(self):
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

    def test_dashboard_diff_dashboards_returns_zero_when_dashboard_matches(self):
        remote_payload = {
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(dashboards={"abc": remote_payload})

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_diff_dashboards_prints_unified_diff_when_dashboard_changes(self):
        remote_payload = {
            "dashboard": {"id": 7, "uid": "abc", "title": "Memory", "panels": []},
            "meta": {"folderUid": "infra"},
        }
        client = FakeDashboardWorkflowClient(dashboards={"abc": remote_payload})

        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            exporter.write_json_document(
                build_export_metadata(
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

    def test_dashboard_build_preserved_web_import_document_keeps_uid_and_title(self):
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

    def test_dashboard_build_import_payload_rejects_web_import_placeholders(self):
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

    def test_dashboard_build_external_export_document_adds_datasource_inputs(self):
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
                {
                    "uid": "prom_uid",
                    "name": "Prom Main",
                    "type": "prometheus",
                    "pluginVersion": "11.0.0",
                },
                {
                    "uid": "loki_uid",
                    "name": "Loki Logs",
                    "type": "loki",
                    "meta": {"info": {"version": "3.1.0"}},
                },
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
        self.assertEqual(
            [
                (item["id"], item["name"], item["version"])
                for item in document["__requires"]
                if item["type"] == "datasource"
            ],
            [("loki", "Loki", "3.1.0"), ("prometheus", "Prometheus", "11.0.0")],
        )
        self.assertEqual(document["__elements"], {})

    def test_dashboard_build_preserved_web_import_document_keeps_mixed_panel_query_datasources(self):
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

    def test_dashboard_build_external_export_document_rewrites_mixed_panel_query_datasources(self):
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
                {
                    "uid": "prom_uid",
                    "name": "Smoke Prometheus",
                    "type": "prometheus",
                    "pluginVersion": "11.0.0",
                },
                {
                    "uid": "loki_uid",
                    "name": "Smoke Loki",
                    "type": "loki",
                    "meta": {"info": {"version": "3.1.0"}},
                },
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
        self.assertEqual(
            [
                (item["id"], item["name"], item["version"])
                for item in document["__requires"]
                if item["type"] == "datasource"
            ],
            [("loki", "Loki", "3.1.0"), ("prometheus", "Prometheus", "11.0.0")],
        )

    def test_dashboard_build_external_export_document_keeps_distinct_same_type_datasources_separate(self):
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
                {
                    "uid": "prom_uid_1",
                    "name": "Smoke Prometheus",
                    "type": "prometheus",
                    "pluginVersion": "11.0.0",
                },
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
        self.assertEqual(
            [
                (item["id"], item["name"], item["version"])
                for item in document["__requires"]
                if item["type"] == "datasource"
            ],
            [("prometheus", "Prometheus", "11.0.0")],
        )
        self.assertNotIn("templating", document)

    def test_dashboard_build_external_export_document_resolves_string_datasource_uid(self):
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

    def test_dashboard_build_external_export_document_resolves_string_datasource_type_alias(self):
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

    def test_dashboard_build_external_export_document_converts_existing_datasource_variable(self):
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

    def test_dashboard_build_external_export_document_preserves_untyped_datasource_variable(self):
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

    def test_dashboard_build_external_export_document_creates_input_from_datasource_template_variable(self):
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

    def test_dashboard_build_external_export_document_keeps_builtin_grafana_datasource_name(self):
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
