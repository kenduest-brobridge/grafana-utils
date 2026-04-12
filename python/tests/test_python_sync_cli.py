import ast
import importlib
import io
import json
import sys
import tempfile
import unittest
import unittest.mock
from contextlib import redirect_stdout, redirect_stderr
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
MODULE_PATH = PYTHON_ROOT / "grafana_utils" / "sync_cli.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))
sync_cli = importlib.import_module("grafana_utils.sync_cli")


class FakeSyncGrafanaClient(object):
    def __init__(
        self,
        folders=None,
        dashboards=None,
        datasources=None,
        plugins=None,
        contact_points=None,
        alert_rules=None,
    ):
        self._folders = list(folders or [])
        self._dashboards = list(dashboards or [])
        self._datasources = list(datasources or [])
        self._plugins = list(plugins or [])
        self._contact_points = list(contact_points or [])
        self._alert_rules = [dict(item) for item in (alert_rules or [])]
        self.calls = []

    def with_org_id(self, org_id):
        self.calls.append({"kind": "with-org-id", "orgId": str(org_id)})
        return self

    def request_json(self, path, params=None, method="GET", payload=None):
        self.calls.append(
            {
                "kind": "request",
                "path": path,
                "params": dict(params or {}),
                "method": method,
                "payload": payload,
            }
        )
        if path == "/api/folders" and method == "GET":
            return list(self._folders)
        if path.startswith("/api/folders/") and method in ("PUT", "DELETE"):
            return {"status": "ok"}
        if path == "/api/datasources" and method == "GET":
            return list(self._datasources)
        if path == "/api/plugins" and method == "GET":
            return list(self._plugins)
        if path == "/api/v1/provisioning/contact-points" and method == "GET":
            return list(self._contact_points)
        if path == "/api/v1/provisioning/alert-rules" and method == "GET":
            return [dict(item) for item in self._alert_rules]
        if path == "/api/v1/provisioning/alert-rules" and method == "POST":
            created = dict(payload or {})
            self._alert_rules.append(created)
            return created
        if path.startswith("/api/v1/provisioning/alert-rules/") and method == "PUT":
            uid = path.rsplit("/", 1)[-1]
            updated = dict(payload or {})
            for index, item in enumerate(self._alert_rules):
                if str(item.get("uid") or "") == uid:
                    self._alert_rules[index] = updated
                    return updated
            raise AssertionError("Unexpected sync alert update target %s" % uid)
        if path.startswith("/api/v1/provisioning/alert-rules/") and method == "DELETE":
            uid = path.rsplit("/", 1)[-1]
            self._alert_rules = [
                item for item in self._alert_rules if str(item.get("uid") or "") != uid
            ]
            return {"status": "deleted"}
        if path == "/api/datasources" and method == "POST":
            return {"status": "created"}
        if path.startswith("/api/datasources/") and method in ("PUT", "DELETE"):
            return {"status": "ok"}
        if path.startswith("/api/dashboards/uid/") and method == "DELETE":
            return {"status": "deleted"}
        raise AssertionError("Unexpected sync request %s %s" % (method, path))

    def iter_dashboard_summaries(self, page_size):
        self.calls.append({"kind": "iter-dashboard-summaries", "pageSize": page_size})
        return [
            {
                "uid": item["uid"],
                "title": item["dashboard"]["title"],
            }
            for item in self._dashboards
        ]

    def fetch_dashboard_if_exists(self, uid):
        for item in self._dashboards:
            if item["uid"] == uid:
                return {"dashboard": dict(item["dashboard"])}
        return None

    def list_datasources(self):
        return list(self._datasources)

    def create_folder(self, uid, title, parent_uid=None):
        self.calls.append(
            {
                "kind": "create-folder",
                "uid": uid,
                "title": title,
                "parentUid": parent_uid,
            }
        )
        return {"uid": uid, "title": title}

    def import_dashboard(self, payload):
        self.calls.append({"kind": "import-dashboard", "payload": payload})
        return {"status": "imported"}


class SyncCliTests(unittest.TestCase):
    def test_sync_cli_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_sync_root_help_includes_examples(self):
        help_text = sync_cli.build_parser().format_help()

        self.assertIn("Examples:", help_text)
        self.assertIn("scan", help_text)
        self.assertIn("test", help_text)
        self.assertIn("preview", help_text)
        self.assertIn("grafana-util sync plan", help_text)
        self.assertIn("grafana-util sync apply", help_text)
        self.assertIn("ci", help_text)
        self.assertIn("package", help_text)

    def test_sync_build_parser_has_ci_and_scan_commands(self):
        parser = sync_cli.build_parser()
        root_subparsers = parser._subparsers._group_actions[0]
        self.assertIn("scan", root_subparsers.choices)
        self.assertIn("test", root_subparsers.choices)
        self.assertIn("preview", root_subparsers.choices)
        self.assertIn("ci", root_subparsers.choices)
        self.assertIn("package", root_subparsers.choices)
        self.assertIn("bundle", root_subparsers.choices)
        ci_parser = root_subparsers.choices["ci"]
        ci_subparsers = ci_parser._subparsers._group_actions[0]
        self.assertIn("summary", ci_subparsers.choices)
        self.assertIn("plan", ci_subparsers.choices)
        self.assertIn("mark-reviewed", ci_subparsers.choices)
        self.assertIn("input-test", ci_subparsers.choices)
        self.assertIn("alert-readiness", ci_subparsers.choices)
        self.assertIn("package-test", ci_subparsers.choices)
        self.assertIn("audit", ci_subparsers.choices)
        self.assertIn("promote-test", ci_subparsers.choices)

    def test_sync_apply_help_groups_controls_and_examples(self):
        help_text = (
            sync_cli.build_parser()
            ._subparsers._group_actions[0]
            .choices["apply"]
            .format_help()
        )

        self.assertIn("Apply Control Options", help_text)
        self.assertIn("Runtime Options", help_text)
        self.assertIn("Output Options", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("--approve", help_text)
        self.assertIn("--execute-live", help_text)
        self.assertIn("--output", help_text)

    def test_sync_review_help_groups_controls_and_examples(self):
        help_text = (
            sync_cli.build_parser()
            ._subparsers._group_actions[0]
            .choices["review"]
            .format_help()
        )

        self.assertIn("Apply Control Options", help_text)
        self.assertIn("Output Options", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn("--output", help_text)

    def test_sync_bundle_preflight_help_includes_secret_placeholder_example(self):
        help_text = (
            sync_cli.build_parser()
            ._subparsers._group_actions[0]
            .choices["bundle-preflight"]
            .format_help()
        )

        self.assertIn("Examples:", help_text)
        self.assertIn("--availability-file", help_text)
        self.assertIn("secretPlaceholderNames", help_text)
        self.assertIn('"providerNames": ["vault"]', help_text)

    def test_sync_summary_renders_text_counts(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
                "sourcePath": "datasources/prom-main.json",
            },
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"datasourceUids": ["prom-main"]},
                "sourcePath": "dashboards/cpu-main.json",
            },
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition", "contactPoints"],
                "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]},
                "sourcePath": "alerts/cpu-high.json",
            },
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(["summary", "--desired-file", str(desired_path)])

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Sync summary", output)
            self.assertIn(
                "4 total, 1 dashboards, 1 datasources, 1 folders, 1 alerts", output
            )

    def test_sync_summary_renders_json_document(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    ["summary", "--desired-file", str(desired_path), "--output", "json"]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["kind"], "grafana-utils-sync-summary")
            self.assertEqual(document["summary"]["resourceCount"], 1)
            self.assertEqual(document["summary"]["folderCount"], 1)
            self.assertEqual(document["resources"][0]["identity"], "ops")

    def test_sync_scan_aliases_to_summary(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(["scan", "--desired-file", str(desired_path)])

            self.assertEqual(result, 0)
            self.assertIn("Sync summary", stdout.getvalue())

    def test_sync_test_aliases_to_preflight(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(["test", "--desired-file", str(desired_path)])

            self.assertEqual(result, 0)
            self.assertIn("Sync preflight", stdout.getvalue())

    def test_sync_preview_aliases_to_plan(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path = Path(tmpdir) / "live.json"
            live_path.write_text(json.dumps(live), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "preview",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                    ]
                )

            self.assertEqual(result, 0)
            self.assertIn("Sync plan", stdout.getvalue())

    def test_sync_ci_summary_aliases_to_summary(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    ["ci", "summary", "--desired-file", str(desired_path)]
                )

            self.assertEqual(result, 0)
            self.assertIn("Sync summary", stdout.getvalue())

    def test_sync_ci_mark_reviewed_aliases_to_review(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path = Path(tmpdir) / "live.json"
            live_path.write_text(json.dumps(live), encoding="utf-8")
            plan_path = Path(tmpdir) / "plan.json"
            with redirect_stdout(io.StringIO()):
                sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--plan-file",
                        str(plan_path),
                    ]
                )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(["ci", "mark-reviewed", "--plan-file", str(plan_path)])

            self.assertEqual(result, 0)
            self.assertIn("Sync plan", stdout.getvalue())

    def test_sync_ci_audit_reports_json_with_baseline_and_lock(self):
        managed = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "managedFields": ["title"],
                "sourcePath": "managed/ops.json",
            }
        ]
        baseline_live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "baseline/ops.json",
            }
        ]
        live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations Drifted",
                "body": {"title": "Operations Drifted"},
                "sourcePath": "live/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            managed_path = Path(tmpdir) / "managed.json"
            managed_path.write_text(json.dumps(managed), encoding="utf-8")
            live_path = Path(tmpdir) / "live.json"
            live_path.write_text(json.dumps(live), encoding="utf-8")
            baseline_lock = sync_cli.build_sync_lock_document(managed, baseline_live)
            lock_path = Path(tmpdir) / "baseline-lock.json"
            lock_path.write_text(
                json.dumps(baseline_lock, sort_keys=True), encoding="utf-8"
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "ci",
                        "audit",
                        "--managed-file",
                        str(managed_path),
                        "--lock-file",
                        str(lock_path),
                        "--live-file",
                        str(live_path),
                        "--output",
                        "json",
                    ]
                )
            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["kind"], "grafana-utils-sync-audit")
            self.assertEqual(document["summary"]["managedCount"], 1)
            self.assertEqual(document["summary"]["driftCount"], 1)
            self.assertEqual(document["summary"]["baselineCount"], 1)

    def test_sync_ci_audit_fail_on_drift_blocks_write_lock(self):
        managed = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "managedFields": ["title"],
            }
        ]
        baseline_live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        live = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations Drifted",
                "body": {"title": "Operations Drifted"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            managed_path = Path(tmpdir) / "managed.json"
            managed_path.write_text(json.dumps(managed), encoding="utf-8")
            live_path = Path(tmpdir) / "live.json"
            live_path.write_text(json.dumps(live), encoding="utf-8")
            baseline_lock = sync_cli.build_sync_lock_document(managed, baseline_live)
            lock_path = Path(tmpdir) / "baseline-lock.json"
            lock_path.write_text(json.dumps(baseline_lock), encoding="utf-8")
            write_lock_path = Path(tmpdir) / "current-lock.json"
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "ci",
                        "audit",
                        "--managed-file",
                        str(managed_path),
                        "--lock-file",
                        str(lock_path),
                        "--live-file",
                        str(live_path),
                        "--fail-on-drift",
                        "--write-lock",
                        str(write_lock_path),
                    ]
                )
            self.assertEqual(result, 1)
            self.assertIn(
                "Sync audit detected 1 drifted resource(s).",
                stderr.getvalue(),
            )
            self.assertFalse(write_lock_path.exists())

            write_lock_ok_path = Path(tmpdir) / "current-lock-ok.json"
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "ci",
                        "audit",
                        "--managed-file",
                        str(managed_path),
                        "--lock-file",
                        str(lock_path),
                        "--live-file",
                        str(live_path),
                        "--write-lock",
                        str(write_lock_ok_path),
                    ]
                )
            self.assertEqual(result, 0)
            self.assertEqual(
                json.loads(write_lock_ok_path.read_text(encoding="utf-8"))["kind"],
                "grafana-utils-sync-lock",
            )

    def test_sync_ci_audit_requires_scope_inputs(self):
        managed = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "managedFields": ["title"],
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            managed_path = Path(tmpdir) / "managed.json"
            managed_path.write_text(json.dumps(managed), encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "ci",
                        "audit",
                        "--managed-file",
                        str(managed_path),
                    ]
                )
            self.assertEqual(result, 1)
            self.assertIn(
                "Sync audit requires --live-file unless --fetch-live is used.",
                stderr.getvalue(),
            )
            self.assertEqual(stdout.getvalue(), "")

    def test_sync_ci_audit_requires_live_or_fetch_live(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            lock = {"kind": "grafana-utils-sync-lock", "schemaVersion": 1, "summary": {}, "resources": []}
            lock_path = Path(tmpdir) / "lock.json"
            lock_path.write_text(json.dumps(lock), encoding="utf-8")

            stdout = io.StringIO()
            stderr = io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "ci",
                        "audit",
                        "--lock-file",
                        str(lock_path),
                    ]
                )
            self.assertEqual(result, 1)
            self.assertIn(
                "Sync audit requires --live-file unless --fetch-live is used.",
                stderr.getvalue(),
            )
            self.assertEqual(stdout.getvalue(), "")

    def test_sync_ci_promote_test_reports_resolved_and_blocking_checks(self):
        source_bundle = {
            "dashboards": [
                {
                    "uid": "db-main",
                    "title": "DB Main",
                    "folderUid": "ops-target",
                    "datasourceUids": ["prom-target"],
                },
                {
                    "uid": "cache-main",
                    "title": "Cache Main",
                    "folderUid": "ops-missing",
                    "datasourceUids": ["prom-missing"],
                },
            ],
            "datasources": [],
            "folders": [],
            "summary": {
                "dashboardCount": 2,
                "datasourceCount": 0,
                "folderCount": 0,
                "alertRuleCount": 0,
                "contactPointCount": 0,
            },
        }
        target_inventory = {
            "dashboards": [],
            "datasources": [
                {
                    "uid": "prom-target",
                    "name": "Prometheus",
                    "type": "prometheus",
                }
            ],
            "folders": [
                {
                    "uid": "ops-target",
                    "title": "Operations",
                }
            ],
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            source_path = Path(tmpdir) / "source.json"
            source_path.write_text(json.dumps(source_bundle), encoding="utf-8")
            target_path = Path(tmpdir) / "target.json"
            target_path.write_text(json.dumps(target_inventory), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "ci",
                        "promote-test",
                        "--source-bundle",
                        str(source_path),
                        "--target-inventory",
                        str(target_path),
                        "--output",
                        "json",
                    ]
                )
            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["kind"], "grafana-utils-sync-promotion-preflight")
            self.assertEqual(
                int(document["checkSummary"]["resolvedCount"]),
                2,
            )
            self.assertEqual(
                int(document["summary"]["missingMappingCount"]),
                2,
            )
            expected_blocking_count = int(
                document["summary"]["blockingCount"]
            )
            bundle_summary = document["bundlePreflight"]["summary"]
            expected_by_source = int(len(document["blockingChecks"])) + int(
                bundle_summary.get("syncBlockingCount", 0)
            ) + int(bundle_summary.get("providerBlockingCount", 0)) + int(
                bundle_summary.get("secretBlockingCount", 0)
            ) + int(
                bundle_summary.get("secretPlaceholderBlockingCount", 0)
            ) + int(bundle_summary.get("alertBlockedCount", 0))
            self.assertEqual(expected_blocking_count, expected_by_source)
            self.assertEqual(int(document["summary"]["resourceCount"]), 2)
            self.assertEqual(len(document["resolvedChecks"]), 2)
            self.assertEqual(len(document["blockingChecks"]), 2)
            self.assertIn("folder-remap", document["blockingChecks"][0]["kind"])

    def test_sync_ci_promote_test_reports_without_mapping_and_availability_inputs(self):
        source_bundle = {
            "dashboards": [],
            "datasources": [],
            "folders": [],
            "alerts": [],
            "summary": {
                "dashboardCount": 0,
                "datasourceCount": 0,
                "folderCount": 0,
                "alertRuleCount": 0,
                "contactPointCount": 0,
            },
        }
        target_inventory = {
            "dashboards": [],
            "datasources": [],
            "folders": [],
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            source_path = Path(tmpdir) / "source.json"
            source_path.write_text(json.dumps(source_bundle), encoding="utf-8")
            target_path = Path(tmpdir) / "target.json"
            target_path.write_text(json.dumps(target_inventory), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "ci",
                        "promote-test",
                        "--source-bundle",
                        str(source_path),
                        "--target-inventory",
                        str(target_path),
                        "--output",
                        "json",
                    ]
                )
            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["kind"], "grafana-utils-sync-promotion-preflight")
            self.assertIn("bundlePreflight", document)
            self.assertEqual(int(document["summary"]["blockingCount"]), 0)

    def test_sync_package_aliases_to_bundle(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            dashboard = Path(tmpdir) / "dashboards"
            src = dashboard / "raw"
            src.mkdir(parents=True)
            (dashboard / "folders.json").write_text("[]", encoding="utf-8")
            (dashboard / "datasources.json").write_text("[]", encoding="utf-8")
            (src / "ops.json").write_text(
                json.dumps(desired[0], ensure_ascii=False),
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "package",
                        "--dashboard-export-dir",
                        str(dashboard),
                    ]
                )
            self.assertEqual(result, 0)
            self.assertIn("Sync source bundle", stdout.getvalue())

    @staticmethod
    def _ensure_review_stage(document, trace_id="sync-trace-test"):
        document["traceId"] = document.get("traceId", trace_id)
        document["stage"] = "review"
        document["stepIndex"] = 2
        document["parentTraceId"] = document["traceId"]
        return document

    @staticmethod
    def _ensure_lineage(document, stage, parent_trace_id=None):
        trace_id = document.get("traceId") or "sync-trace-test"
        document["traceId"] = trace_id
        document["stage"] = stage
        document["stepIndex"] = document.get("stepIndex", 1)
        if parent_trace_id:
            document["parentTraceId"] = parent_trace_id
        else:
            document.pop("parentTraceId", None)
        return document

    def test_sync_plan_builds_review_required_document_and_writes_plan_file(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        live = []
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text(json.dumps(live), encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--plan-file",
                        str(plan_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertTrue(document["dryRun"])
            self.assertTrue(document["reviewRequired"])
            self.assertFalse(document["reviewed"])
            self.assertEqual(document["summary"]["would_create"], 1)
            self.assertEqual(document["summary"]["alert_candidate"], 0)
            self.assertEqual(document["summary"]["alert_plan_only"], 0)
            self.assertEqual(document["summary"]["alert_blocked"], 0)
            self.assertEqual(document["alertAssessment"]["alerts"], [])
            self.assertEqual(
                json.loads(plan_path.read_text(encoding="utf-8")), document
            )

    def test_sync_plan_renders_text_output_by_default(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        live = []
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text(json.dumps(live), encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                    ]
                )

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Sync plan", output)
            self.assertIn("Summary: create=1 update=0 delete=0 noop=0 unmanaged=0", output)

    def test_sync_plan_can_fetch_live_state_from_grafana(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": False,
                },
            },
        ]
        client = FakeSyncGrafanaClient(
            folders=[{"uid": "ops", "title": "Operations"}],
            datasources=[
                {
                    "id": 7,
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": False,
                }
            ],
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "plan",
                            "--desired-file",
                            str(desired_path),
                            "--fetch-live",
                            "--url",
                            "http://127.0.0.1:3000",
                            "--output",
                            "json",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["noop"], 2)

    def test_sync_plan_fetch_live_includes_alert_rules(self):
        desired = [
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition"],
                "body": {"condition": "A"},
            }
        ]
        client = FakeSyncGrafanaClient(
            alert_rules=[
                {
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "folderUID": "ops",
                    "ruleGroup": "cpu",
                    "condition": "A",
                    "data": [],
                }
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "plan",
                            "--desired-file",
                            str(desired_path),
                            "--fetch-live",
                            "--url",
                            "http://127.0.0.1:3000",
                            "--output",
                            "json",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["noop"], 1)
            self.assertEqual(document["summary"]["alert_candidate"], 1)

    def test_sync_plan_includes_alert_assessment_summary(self):
        desired = [
            {
                "kind": "alert",
                "uid": "cpu-high",
                "title": "CPU High",
                "managedFields": ["condition", "contactPoints"],
                "body": {
                    "condition": "A > 90",
                    "contactPoints": ["pagerduty-primary"],
                },
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["alert_plan_only"], 1)
            self.assertEqual(document["summary"]["alert_blocked"], 0)
            self.assertEqual(
                document["alertAssessment"]["alerts"][0]["status"], "plan-only"
            )
            self.assertEqual(
                document["operations"][0]["managedFields"],
                ["condition", "contactPoints"],
            )

    def test_sync_preflight_renders_text_summary(self):
        desired = [
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
            }
        ]
        availability = {"pluginIds": [], "datasourceUids": [], "datasourceNames": []}
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            availability_path = Path(tmpdir) / "availability.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            availability_path.write_text(json.dumps(availability), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "preflight",
                        "--desired-file",
                        str(desired_path),
                        "--availability-file",
                        str(availability_path),
                    ]
                )

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Sync preflight summary", output)
            self.assertIn("plugin identity=prometheus status=missing", output)

    def test_sync_preflight_can_fetch_live_availability(self):
        desired = [
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "body": {"type": "prometheus"},
            },
            {
                "kind": "alert",
                "uid": "cpu-high",
                "managedFields": ["condition", "contactPoints"],
                "body": {
                    "condition": "A > 90",
                    "contactPoints": ["pagerduty-primary"],
                    "receiver": "cp-1",
                },
            },
            {
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"datasourceNames": ["Prometheus Main"]},
            },
        ]
        client = FakeSyncGrafanaClient(
            datasources=[{"uid": "prom-main", "name": "Prometheus Main"}],
            plugins=[{"id": "prometheus"}],
            contact_points=[{"name": "pagerduty-primary", "uid": "cp-1"}],
        )
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "preflight",
                            "--desired-file",
                            str(desired_path),
                            "--fetch-live",
                            "--url",
                            "http://127.0.0.1:3000",
                        ]
                    )

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("datasource identity=prom-main status=ok", output)
            self.assertIn(
                "alert-contact-point identity=cpu-high->pagerduty-primary status=ok",
                output,
            )
            self.assertIn(
                "alert-contact-point identity=cpu-high->cp-1 status=ok", output
            )
            self.assertIn(
                "dashboard-datasource-name identity=cpu-main->Prometheus Main status=ok",
                output,
            )

    def test_sync_assess_alerts_renders_json(self):
        alerts = [
            {
                "kind": "alert",
                "uid": "cpu-high",
                "managedFields": ["condition", "contactPoints"],
                "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            alerts_path = Path(tmpdir) / "alerts.json"
            alerts_path.write_text(json.dumps(alerts), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "assess-alerts",
                        "--alerts-file",
                        str(alerts_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["planOnlyCount"], 1)
            self.assertEqual(document["alerts"][0]["status"], "plan-only")

    def test_sync_bundle_preflight_renders_json(self):
        source_bundle = {
            "environment": "staging",
            "dashboards": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "datasourceUids": ["prom-main"],
                }
            ],
            "datasources": [
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "secureJsonDataPlaceholders": {
                        "basicAuthPassword": "${secret:prom-basic-auth}",
                    },
                    "secureJsonDataProviders": {
                        "httpHeaderValue1": "${provider:vault:secret/data/prom/token}",
                    },
                }
            ],
            "alerts": [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "A > 90",
                        "datasourceUid": "prom-main",
                        "datasourceName": "Prometheus Main",
                        "contactPoints": ["pagerduty-primary"],
                        "notificationSettings": {"receiver": "slack-primary"},
                    },
                }
            ],
        }
        target_inventory = {"environment": "prod", "dashboards": [], "datasources": []}
        availability = {
            "pluginIds": [],
            "datasourceUids": [],
            "datasourceNames": [],
            "contactPoints": [],
            "providerNames": ["vault"],
            "secretPlaceholderNames": ["prom-basic-auth"],
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            source_path = Path(tmpdir) / "source.json"
            target_path = Path(tmpdir) / "target.json"
            availability_path = Path(tmpdir) / "availability.json"
            source_path.write_text(json.dumps(source_bundle), encoding="utf-8")
            target_path.write_text(json.dumps(target_inventory), encoding="utf-8")
            availability_path.write_text(json.dumps(availability), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "bundle-preflight",
                        "--source-bundle",
                        str(source_path),
                        "--target-inventory",
                        str(target_path),
                        "--availability-file",
                        str(availability_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["alertPlanOnlyCount"], 1)
            self.assertIn("syncPreflight", document)
            self.assertEqual(document["summary"]["providerBlockingCount"], 0)
            self.assertEqual(document["summary"]["secretBlockingCount"], 0)
            self.assertEqual(
                document["secretAssessment"]["plans"][0]["providerKind"],
                "inline-placeholder-map",
            )
            self.assertEqual(
                document["secretAssessment"]["plans"][0]["placeholderNames"],
                ["prom-basic-auth"],
            )
            self.assertEqual(
                document["secretAssessment"]["checks"][0]["placeholderName"],
                "prom-basic-auth",
            )
            checks = {
                (item["kind"], item["identity"]): item
                for item in document["syncPreflight"]["checks"]
            }
            self.assertEqual(
                checks[("alert-datasource", "cpu-high->prom-main")]["status"],
                "missing",
            )
            self.assertEqual(
                checks[("alert-datasource-name", "cpu-high->Prometheus Main")][
                    "status"
                ],
                "missing",
            )
            self.assertEqual(
                checks[("alert-contact-point", "cpu-high->slack-primary")]["status"],
                "missing",
            )
            self.assertEqual(
                document["providerAssessment"]["plans"][0]["providers"][0][
                    "providerName"
                ],
                "vault",
            )

    def test_sync_bundle_preflight_can_fetch_live_availability(self):
        source_bundle = {
            "environment": "staging",
            "dashboards": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "datasourceUids": ["prom-main"],
                }
            ],
            "datasources": [
                {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}
            ],
            "alerts": [
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "A > 90",
                        "datasourceUid": "prom-main",
                        "datasourceName": "Prometheus Main",
                        "contactPoints": ["pagerduty-primary"],
                        "notificationSettings": {"receiver": "slack-primary"},
                    },
                }
            ],
        }
        target_inventory = {"environment": "prod", "dashboards": [], "datasources": []}
        client = FakeSyncGrafanaClient(
            datasources=[{"uid": "prom-main", "name": "Prometheus Main"}],
            plugins=[{"id": "prometheus"}],
            contact_points=[{"name": "pagerduty-primary"}, {"uid": "slack-primary"}],
        )
        with tempfile.TemporaryDirectory() as tmpdir:
            source_path = Path(tmpdir) / "source.json"
            target_path = Path(tmpdir) / "target.json"
            source_path.write_text(json.dumps(source_bundle), encoding="utf-8")
            target_path.write_text(json.dumps(target_inventory), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "bundle-preflight",
                            "--source-bundle",
                            str(source_path),
                            "--target-inventory",
                            str(target_path),
                            "--fetch-live",
                            "--output",
                            "json",
                            "--url",
                            "http://127.0.0.1:3000",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["syncBlockingCount"], 1)
            checks = {
                (item["kind"], item["identity"]): item
                for item in document["syncPreflight"]["checks"]
            }
            self.assertEqual(
                checks[("alert-datasource", "cpu-high->prom-main")]["status"],
                "ok",
            )
            self.assertEqual(
                checks[("alert-datasource-name", "cpu-high->Prometheus Main")][
                    "status"
                ],
                "ok",
            )
            self.assertEqual(
                checks[("alert-contact-point", "cpu-high->slack-primary")]["status"],
                "ok",
            )
            self.assertEqual(
                checks[("alert-live-apply", "cpu-high")]["status"],
                "blocked",
            )

    def test_sync_bundle_preflight_flags_missing_provider_and_secret_availability(self):
        source_bundle = {
            "environment": "staging",
            "datasources": [
                {
                    "uid": "loki-main",
                    "name": "Loki Main",
                    "type": "loki",
                    "secureJsonDataPlaceholders": {
                        "basicAuthPassword": "${secret:loki-basic-auth}",
                    },
                    "secureJsonDataProviders": {
                        "httpHeaderValue1": "${provider:aws-sm:prod/loki/token}",
                    },
                }
            ],
        }
        target_inventory = {"environment": "prod", "dashboards": [], "datasources": []}
        availability = {
            "pluginIds": ["loki"],
            "datasourceUids": [],
            "datasourceNames": [],
            "contactPoints": [],
        }
        with tempfile.TemporaryDirectory() as tmpdir:
            source_path = Path(tmpdir) / "source.json"
            target_path = Path(tmpdir) / "target.json"
            availability_path = Path(tmpdir) / "availability.json"
            source_path.write_text(json.dumps(source_bundle), encoding="utf-8")
            target_path.write_text(json.dumps(target_inventory), encoding="utf-8")
            availability_path.write_text(json.dumps(availability), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "bundle-preflight",
                        "--source-bundle",
                        str(source_path),
                        "--target-inventory",
                        str(target_path),
                        "--availability-file",
                        str(availability_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["providerBlockingCount"], 1)
            self.assertEqual(document["summary"]["secretBlockingCount"], 1)
            self.assertEqual(
                document["secretAssessment"]["plans"][0]["providerKind"],
                "inline-placeholder-map",
            )
            self.assertEqual(
                document["secretAssessment"]["checks"][0]["placeholderName"],
                "loki-basic-auth",
            )
            self.assertEqual(
                document["providerAssessment"]["checks"][0]["status"],
                "missing",
            )
            self.assertEqual(
                document["secretAssessment"]["checks"][0]["status"],
                "missing",
            )

    def test_sync_apply_rejects_boolean_bundle_preflight_counts(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }
        bundle_preflight_document = {
            "kind": "grafana-utils-sync-bundle-preflight",
            "traceId": "sync-trace-apply",
            "stage": "bundle-preflight",
            "stepIndex": 4,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "resourceCount": True,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
            },
            "syncPreflight": {
                "summary": {
                    "checkCount": 0,
                    "okCount": 0,
                    "blockingCount": 0,
                }
            },
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            bundle_preflight_path = Path(tmpdir) / "bundle-preflight.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            bundle_preflight_path.write_text(
                json.dumps(bundle_preflight_document), encoding="utf-8"
            )

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "apply",
                        "--plan-file",
                        str(plan_path),
                        "--bundle-preflight-file",
                        str(bundle_preflight_path),
                        "--approve",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("missing resourceCount", stderr.getvalue())

    def test_sync_bundle_packages_dashboard_and_alert_exports(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            dashboard_dir = root / "dashboards" / "raw"
            alert_dir = root / "alerts" / "raw"
            dashboard_dir.mkdir(parents=True)
            (alert_dir / "rules" / "infra" / "cpu").mkdir(parents=True)
            (alert_dir / "contact-points" / "Webhook_Main").mkdir(parents=True)
            (alert_dir / "policies").mkdir(parents=True)
            metadata_path = root / "metadata.json"
            output_path = root / "bundle.json"

            (dashboard_dir / "export-metadata.json").write_text(
                json.dumps({"kind": "grafana-dashboard-export-metadata"}),
                encoding="utf-8",
            )
            (dashboard_dir / "folders.json").write_text(
                json.dumps(
                    [
                        {
                            "uid": "ops",
                            "title": "Operations",
                            "path": "Operations",
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (dashboard_dir / "datasources.json").write_text(
                json.dumps(
                    [
                        {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": False,
                        }
                    ]
                ),
                encoding="utf-8",
            )
            (dashboard_dir / "cpu__cpu-main.json").write_text(
                json.dumps(
                    {
                        "dashboard": {
                            "id": None,
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "panels": [],
                        }
                    }
                ),
                encoding="utf-8",
            )
            (alert_dir / "export-metadata.json").write_text(
                json.dumps({"kind": "grafana-alert-export-metadata"}),
                encoding="utf-8",
            )
            (
                alert_dir / "rules" / "infra" / "cpu" / "CPU_High__rule-uid.json"
            ).write_text(
                json.dumps({"kind": "grafana-alert-rule", "spec": {"uid": "rule-uid"}}),
                encoding="utf-8",
            )
            (
                alert_dir
                / "contact-points"
                / "Webhook_Main"
                / "Webhook_Main__cp-uid.json"
            ).write_text(
                json.dumps(
                    {"kind": "grafana-alert-contact-point", "spec": {"uid": "cp-uid"}}
                ),
                encoding="utf-8",
            )
            (alert_dir / "policies" / "notification-policies.json").write_text(
                json.dumps({"kind": "grafana-alert-notification-policies"}),
                encoding="utf-8",
            )
            metadata_path.write_text(
                json.dumps({"environment": "staging"}),
                encoding="utf-8",
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "bundle",
                        "--dashboard-export-dir",
                        str(dashboard_dir),
                        "--alert-export-dir",
                        str(alert_dir),
                        "--metadata-file",
                        str(metadata_path),
                        "--output-file",
                        str(output_path),
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["kind"], "grafana-utils-sync-source-bundle")
            self.assertEqual(document["summary"]["dashboardCount"], 1)
            self.assertEqual(document["summary"]["datasourceCount"], 1)
            self.assertEqual(document["summary"]["folderCount"], 1)
            self.assertEqual(document["summary"]["alertRuleCount"], 1)
            self.assertEqual(document["summary"]["contactPointCount"], 1)
            self.assertEqual(document["summary"]["policyCount"], 1)
            self.assertEqual(document["metadata"]["environment"], "staging")
            self.assertEqual(
                json.loads(output_path.read_text(encoding="utf-8"))["kind"],
                "grafana-utils-sync-source-bundle",
            )

    def test_sync_bundle_requires_at_least_one_input(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = sync_cli.main(["bundle"])

        self.assertEqual(result, 1)
        self.assertIn("requires at least one export input", stderr.getvalue())

    def test_sync_review_marks_plan_reviewed(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            sync_cli.main(
                [
                    "plan",
                    "--desired-file",
                    str(desired_path),
                    "--live-file",
                    str(live_path),
                    "--plan-file",
                    str(plan_path),
                ]
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    ["review", "--plan-file", str(plan_path), "--output", "json"]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertTrue(document["reviewed"])

    def test_sync_review_accepts_explicit_audit_metadata(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            sync_cli.main(
                [
                    "plan",
                    "--desired-file",
                    str(desired_path),
                    "--live-file",
                    str(live_path),
                    "--plan-file",
                    str(plan_path),
                ]
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "review",
                        "--plan-file",
                        str(plan_path),
                        "--output",
                        "json",
                        "--reviewed-by",
                        "alice",
                        "--reviewed-at",
                        "manual-review",
                        "--review-note",
                        "peer-reviewed",
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["reviewedBy"], "alice")
            self.assertEqual(document["reviewedAt"], "manual-review")
            self.assertEqual(document["reviewNote"], "peer-reviewed")

    def test_sync_review_renders_text_output_when_not_json(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            sync_cli.main(
                [
                    "plan",
                    "--desired-file",
                    str(desired_path),
                    "--live-file",
                    str(live_path),
                    "--plan-file",
                    str(plan_path),
                    "--output",
                    "json",
                ]
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(
                    [
                        "review",
                        "--plan-file",
                        str(plan_path),
                        "--reviewed-by",
                        "alice",
                        "--review-note",
                        "peer-reviewed",
                    ]
                )

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Sync plan", output)
            self.assertIn("Reviewed by: alice", output)
            self.assertIn("Review note: peer-reviewed", output)

    def test_sync_apply_renders_text_output_for_non_live_by_default(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            reviewed_path = Path(tmpdir) / "reviewed-plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--output",
                        "json",
                    ]
                )
            plan_document = json.loads(stdout.getvalue())
            reviewed_document = dict(plan_document)
            reviewed_document["reviewed"] = True
            reviewed_document["dryRun"] = False
            reviewed_document = self._ensure_review_stage(reviewed_document)
            reviewed_path.write_text(json.dumps(reviewed_document), encoding="utf-8")

            apply_stdout = io.StringIO()
            with redirect_stdout(apply_stdout):
                result = sync_cli.main(
                    ["apply", "--plan-file", str(reviewed_path), "--approve"]
                )

            self.assertEqual(result, 0)
            output = apply_stdout.getvalue()
            self.assertIn("Sync apply intent", output)
            self.assertIn("Summary: create=1 update=0 delete=0 executable=1", output)

    def test_sync_review_renders_text_output_by_default(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            sync_cli.main(
                [
                    "plan",
                    "--desired-file",
                    str(desired_path),
                    "--live-file",
                    str(live_path),
                    "--plan-file",
                    str(plan_path),
                ]
            )

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                result = sync_cli.main(["review", "--plan-file", str(plan_path)])

            self.assertEqual(result, 0)
            output = stdout.getvalue()
            self.assertIn("Sync plan", output)
            self.assertIn("Review: required=true reviewed=true", output)

    def test_sync_review_rejects_plan_missing_trace_id(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": True,
            "reviewRequired": True,
            "reviewed": False,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(["review", "--plan-file", str(plan_path)])

            self.assertEqual(result, 1)
            self.assertIn("missing traceId", stderr.getvalue())

    def test_sync_review_rejects_plan_with_wrong_lineage_stage(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": True,
            "traceId": "sync-trace-review",
            "stage": "apply",
            "stepIndex": 3,
            "parentTraceId": "sync-trace-review",
            "reviewRequired": True,
            "reviewed": False,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(["review", "--plan-file", str(plan_path)])

            self.assertEqual(result, 1)
            self.assertIn("unexpected lineage stage", stderr.getvalue())

    def test_sync_review_rejects_boolean_lineage_step_index(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": True,
            "traceId": "sync-trace-review",
            "stage": "plan",
            "stepIndex": True,
            "reviewRequired": True,
            "reviewed": False,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(["review", "--plan-file", str(plan_path)])

            self.assertEqual(result, 1)
            self.assertIn("missing lineage stepIndex metadata", stderr.getvalue())

    def test_sync_apply_rejects_unreviewed_plan_without_live_mutation(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            plan_path = Path(tmpdir) / "plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")
            sync_cli.main(
                [
                    "plan",
                    "--desired-file",
                    str(desired_path),
                    "--live-file",
                    str(live_path),
                    "--plan-file",
                    str(plan_path),
                ]
            )

            with open(plan_path, "r", encoding="utf-8") as handle:
                plan_document = json.load(handle)
            self._ensure_review_stage(plan_document)
            plan_path.write_text(json.dumps(plan_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    ["apply", "--plan-file", str(plan_path), "--approve"]
                )

            self.assertEqual(result, 1)
            self.assertIn("marked reviewed", stderr.getvalue())

    def test_sync_apply_emits_non_live_apply_intent_for_reviewed_plan(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            reviewed_path = Path(tmpdir) / "reviewed-plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--output",
                        "json",
                    ]
                )
            plan_document = json.loads(stdout.getvalue())
            reviewed_document = json.loads(json.dumps(plan_document))
            reviewed_document["reviewed"] = True
            reviewed_document["dryRun"] = False
            reviewed_document = self._ensure_review_stage(reviewed_document)
            reviewed_path.write_text(
                json.dumps(reviewed_document),
                encoding="utf-8",
            )

            apply_stdout = io.StringIO()
            with redirect_stdout(apply_stdout):
                result = sync_cli.main(
                    [
                        "apply",
                        "--plan-file",
                        str(reviewed_path),
                        "--approve",
                        "--output",
                        "json",
                    ]
                )

            self.assertEqual(result, 0)
            intent = json.loads(apply_stdout.getvalue())
            self.assertEqual(intent["mode"], "apply")
            self.assertTrue(intent["reviewed"])
            self.assertEqual(len(intent["operations"]), 1)
            self.assertEqual(intent["operations"][0]["action"], "would-create")
            self.assertNotIn("reviewedBy", intent)
            self.assertNotIn("appliedBy", intent)

    def test_sync_apply_renders_text_output_by_default(self):
        desired = [
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
            }
        ]
        with tempfile.TemporaryDirectory() as tmpdir:
            desired_path = Path(tmpdir) / "desired.json"
            live_path = Path(tmpdir) / "live.json"
            reviewed_path = Path(tmpdir) / "reviewed-plan.json"
            desired_path.write_text(json.dumps(desired), encoding="utf-8")
            live_path.write_text("[]", encoding="utf-8")

            stdout = io.StringIO()
            with redirect_stdout(stdout):
                sync_cli.main(
                    [
                        "plan",
                        "--desired-file",
                        str(desired_path),
                        "--live-file",
                        str(live_path),
                        "--output",
                        "json",
                    ]
                )
            plan_document = json.loads(stdout.getvalue())
            reviewed_document = json.loads(json.dumps(plan_document))
            reviewed_document["reviewed"] = True
            reviewed_document["dryRun"] = False
            reviewed_document = self._ensure_review_stage(reviewed_document)
            reviewed_path.write_text(
                json.dumps(reviewed_document),
                encoding="utf-8",
            )

            apply_stdout = io.StringIO()
            with redirect_stdout(apply_stdout):
                result = sync_cli.main(
                    ["apply", "--plan-file", str(reviewed_path), "--approve"]
                )

            self.assertEqual(result, 0)
            output = apply_stdout.getvalue()
            self.assertIn("Sync apply intent", output)
            self.assertIn("Review: required=true reviewed=true approved=true", output)

    def test_sync_apply_execute_live_runs_supported_operations(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "traceId": "sync-trace-live-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-live-apply",
            "summary": {
                "would_create": 2,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                },
                {
                    "kind": "datasource",
                    "identity": "prom-main",
                    "title": "Prometheus Main",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["name", "type", "url"],
                    "desired": {
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "url": "http://prometheus:9090",
                    },
                    "live": None,
                    "sourcePath": "datasources/prom-main.json",
                },
            ],
        }
        client = FakeSyncGrafanaClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "reviewed-plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "apply",
                            "--plan-file",
                            str(plan_path),
                            "--approve",
                            "--execute-live",
                            "--url",
                            "http://127.0.0.1:3000",
                            "--output",
                            "json",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["mode"], "live-apply")
            self.assertEqual(document["appliedCount"], 2)
            self.assertIn(
                {
                    "kind": "create-folder",
                    "uid": "ops",
                    "title": "Operations",
                    "parentUid": None,
                },
                client.calls,
            )
            self.assertTrue(
                any(
                    item["kind"] == "request"
                    and item["path"] == "/api/datasources"
                    and item["method"] == "POST"
                    for item in client.calls
                )
            )

    def test_sync_apply_execute_live_creates_alert_rule(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "traceId": "sync-trace-live-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-live-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "alert",
                    "identity": "cpu-high",
                    "title": "CPU High",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": [
                        "uid",
                        "title",
                        "folderUID",
                        "ruleGroup",
                        "condition",
                        "data",
                    ],
                    "managedFields": ["condition"],
                    "desired": {
                        "uid": "cpu-high",
                        "title": "CPU High",
                        "folderUID": "ops",
                        "ruleGroup": "cpu",
                        "condition": "A",
                        "data": [],
                    },
                    "live": None,
                    "sourcePath": "alerts/cpu-high.json",
                }
            ],
        }
        client = FakeSyncGrafanaClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "reviewed-plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "apply",
                            "--plan-file",
                            str(plan_path),
                            "--approve",
                            "--execute-live",
                            "--url",
                            "http://127.0.0.1:3000",
                            "--output",
                            "json",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["mode"], "live-apply")
            self.assertEqual(document["appliedCount"], 1)
            self.assertTrue(
                any(
                    item["kind"] == "request"
                    and item["path"] == "/api/v1/provisioning/alert-rules"
                    and item["method"] == "POST"
                    for item in client.calls
                )
            )

    def test_sync_apply_execute_live_rejects_partial_alert_spec(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "traceId": "sync-trace-live-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-live-apply",
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "alert",
                    "identity": "cpu-high",
                    "title": "CPU High",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["condition"],
                    "managedFields": ["condition"],
                    "desired": {"condition": "A"},
                    "live": None,
                    "sourcePath": "alerts/cpu-high.json",
                }
            ],
        }
        client = FakeSyncGrafanaClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "reviewed-plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            stdout = io.StringIO()
            stderr = io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                with unittest.mock.patch.object(
                    sync_cli, "build_client", return_value=client
                ):
                    result = sync_cli.main(
                        [
                            "apply",
                            "--plan-file",
                            str(plan_path),
                            "--approve",
                            "--execute-live",
                            "--url",
                            "http://127.0.0.1:3000",
                        ]
                    )

            self.assertEqual(result, 1)
            self.assertIn(
                "Alert-rule import document is missing required fields",
                stderr.getvalue(),
            )

    def test_sync_apply_rejects_wrong_review_lineage_parent(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "other-trace",
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(["apply", "--plan-file", str(plan_path), "--approve"])

            self.assertEqual(result, 1)
            self.assertIn("unexpected lineage parentTraceId", stderr.getvalue())

    def test_sync_apply_rejects_blocking_preflight_file(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }
        preflight_document = {
            "kind": "grafana-utils-sync-preflight",
            "traceId": "sync-trace-apply",
            "stage": "preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "checkCount": 3,
                "okCount": 1,
                "blockingCount": 2,
            },
            "checks": [],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            preflight_path = Path(tmpdir) / "preflight.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            preflight_path.write_text(json.dumps(preflight_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "apply",
                        "--plan-file",
                        str(plan_path),
                        "--preflight-file",
                        str(preflight_path),
                        "--approve",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("blocking checks", stderr.getvalue())

    def test_sync_apply_rejects_preflight_trace_mismatch(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }
        preflight_document = {
            "kind": "grafana-utils-sync-preflight",
            "traceId": "other-trace",
            "stage": "preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "checkCount": 3,
                "okCount": 3,
                "blockingCount": 0,
            },
            "checks": [],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            preflight_path = Path(tmpdir) / "preflight.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            preflight_path.write_text(json.dumps(preflight_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "apply",
                        "--plan-file",
                        str(plan_path),
                        "--preflight-file",
                        str(preflight_path),
                        "--approve",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("does not match sync plan traceId", stderr.getvalue())

    def test_sync_apply_rejects_boolean_preflight_counts(self):
        reviewed_document = {
            "kind": "grafana-utils-sync-plan",
            "schemaVersion": 1,
            "dryRun": False,
            "traceId": "sync-trace-apply",
            "stage": "review",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 1,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
            },
            "alertAssessment": {
                "summary": {
                    "alertCount": 0,
                    "candidateCount": 0,
                    "planOnlyCount": 0,
                    "blockedCount": 0,
                },
                "alerts": [],
            },
            "operations": [
                {
                    "kind": "folder",
                    "identity": "ops",
                    "title": "Operations",
                    "action": "would-create",
                    "reason": "missing-live",
                    "changedFields": ["title"],
                    "managedFields": [],
                    "desired": {"title": "Operations"},
                    "live": None,
                    "sourcePath": "folders/ops.json",
                }
            ],
        }
        preflight_document = {
            "kind": "grafana-utils-sync-preflight",
            "traceId": "sync-trace-apply",
            "stage": "preflight",
            "stepIndex": 2,
            "parentTraceId": "sync-trace-apply",
            "summary": {
                "checkCount": True,
                "okCount": 1,
                "blockingCount": 0,
            },
            "checks": [],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            plan_path = Path(tmpdir) / "plan.json"
            preflight_path = Path(tmpdir) / "preflight.json"
            plan_path.write_text(json.dumps(reviewed_document), encoding="utf-8")
            preflight_path.write_text(json.dumps(preflight_document), encoding="utf-8")

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    [
                        "apply",
                        "--plan-file",
                        str(plan_path),
                        "--preflight-file",
                        str(preflight_path),
                        "--approve",
                    ]
                )

            self.assertEqual(result, 1)
            self.assertIn("missing checkCount", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
