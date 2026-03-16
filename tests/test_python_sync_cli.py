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


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "sync_cli.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
sync_cli = importlib.import_module("grafana_utils.sync_cli")


class FakeSyncGrafanaClient(object):
    def __init__(self, folders=None, dashboards=None, datasources=None, plugins=None, contact_points=None):
        self._folders = list(folders or [])
        self._dashboards = list(dashboards or [])
        self._datasources = list(datasources or [])
        self._plugins = list(plugins or [])
        self._contact_points = list(contact_points or [])
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

    def test_plan_builds_review_required_document_and_writes_plan_file(self):
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
            self.assertEqual(json.loads(plan_path.read_text(encoding="utf-8")), document)

    def test_plan_can_fetch_live_state_from_grafana(self):
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
                with unittest.mock.patch.object(sync_cli, "build_client", return_value=client):
                    result = sync_cli.main(
                        [
                            "plan",
                            "--desired-file",
                            str(desired_path),
                            "--fetch-live",
                            "--url",
                            "http://127.0.0.1:3000",
                        ]
                    )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["noop"], 2)

    def test_plan_includes_alert_assessment_summary(self):
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
                    ]
                )

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["summary"]["alert_plan_only"], 1)
            self.assertEqual(document["summary"]["alert_blocked"], 0)
            self.assertEqual(document["alertAssessment"]["alerts"][0]["status"], "plan-only")
            self.assertEqual(
                document["operations"][0]["managedFields"],
                ["condition", "contactPoints"],
            )

    def test_preflight_renders_text_summary(self):
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

    def test_preflight_can_fetch_live_availability(self):
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
                with unittest.mock.patch.object(sync_cli, "build_client", return_value=client):
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
            self.assertIn("alert-contact-point identity=cpu-high->pagerduty-primary status=ok", output)
            self.assertIn("alert-contact-point identity=cpu-high->cp-1 status=ok", output)
            self.assertIn(
                "dashboard-datasource-name identity=cpu-main->Prometheus Main status=ok",
                output,
            )

    def test_assess_alerts_renders_json(self):
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

    def test_bundle_preflight_renders_json(self):
        source_bundle = {
            "environment": "staging",
            "dashboards": [{"uid": "cpu-main", "title": "CPU Main", "datasourceUids": ["prom-main"]}],
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
            checks = {
                (item["kind"], item["identity"]): item
                for item in document["syncPreflight"]["checks"]
            }
            self.assertEqual(
                checks[("alert-datasource", "cpu-high->prom-main")]["status"],
                "missing",
            )
            self.assertEqual(
                checks[("alert-datasource-name", "cpu-high->Prometheus Main")]["status"],
                "missing",
            )
            self.assertEqual(
                checks[("alert-contact-point", "cpu-high->slack-primary")]["status"],
                "missing",
            )
            self.assertEqual(
                document["providerAssessment"]["plans"][0]["providers"][0]["providerName"],
                "vault",
            )

    def test_bundle_preflight_can_fetch_live_availability(self):
        source_bundle = {
            "environment": "staging",
            "dashboards": [{"uid": "cpu-main", "title": "CPU Main", "datasourceUids": ["prom-main"]}],
            "datasources": [{"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"}],
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
                with unittest.mock.patch.object(sync_cli, "build_client", return_value=client):
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
                checks[("alert-datasource-name", "cpu-high->Prometheus Main")]["status"],
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

    def test_bundle_preflight_flags_missing_provider_and_secret_availability(self):
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
                document["providerAssessment"]["checks"][0]["status"],
                "missing",
            )
            self.assertEqual(
                document["secretAssessment"]["checks"][0]["status"],
                "missing",
            )

    def test_bundle_packages_dashboard_and_alert_exports(self):
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
                json.dumps({"kind": "grafana-alert-contact-point", "spec": {"uid": "cp-uid"}}),
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

    def test_bundle_requires_at_least_one_input(self):
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            result = sync_cli.main(["bundle"])

        self.assertEqual(result, 1)
        self.assertIn("requires at least one export input", stderr.getvalue())

    def test_review_marks_plan_reviewed(self):
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
            document = json.loads(stdout.getvalue())
            self.assertTrue(document["reviewed"])

    def test_apply_rejects_unreviewed_plan_without_live_mutation(self):
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

            stderr = io.StringIO()
            with redirect_stderr(stderr):
                result = sync_cli.main(
                    ["apply", "--plan-file", str(plan_path), "--approve"]
                )

            self.assertEqual(result, 1)
            self.assertIn("marked reviewed", stderr.getvalue())

    def test_apply_emits_non_live_apply_intent_for_reviewed_plan(self):
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
                    ]
                )
            plan_document = json.loads(stdout.getvalue())
            reviewed_document = json.loads(
                json.dumps(plan_document)
            )
            reviewed_document["reviewed"] = True
            reviewed_document["dryRun"] = False
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
            intent = json.loads(apply_stdout.getvalue())
            self.assertEqual(intent["mode"], "apply")
            self.assertTrue(intent["reviewed"])
            self.assertEqual(len(intent["operations"]), 1)
            self.assertEqual(intent["operations"][0]["action"], "would-create")

    def test_apply_execute_live_runs_supported_operations(self):
        reviewed_document = {
            "dryRun": False,
            "reviewRequired": True,
            "reviewed": True,
            "allowPrune": False,
            "summary": {
                "would_create": 2,
                "would_update": 0,
                "would_delete": 0,
                "noop": 0,
                "unmanaged": 0,
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
                with unittest.mock.patch.object(sync_cli, "build_client", return_value=client):
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

            self.assertEqual(result, 0)
            document = json.loads(stdout.getvalue())
            self.assertEqual(document["mode"], "live-apply")
            self.assertEqual(document["appliedCount"], 2)
            self.assertIn(
                {"kind": "create-folder", "uid": "ops", "title": "Operations", "parentUid": None},
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


if __name__ == "__main__":
    unittest.main()
