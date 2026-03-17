import unittest

from grafana_utils import gitops_sync
from grafana_utils.dashboard_cli import GrafanaError


class GitopsSyncTests(unittest.TestCase):
    def test_gitops_sync_build_sync_source_bundle_document_tracks_portable_sections(
        self,
    ):
        document = gitops_sync.build_sync_source_bundle_document(
            dashboards=[
                {
                    "kind": "dashboard",
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "body": {"uid": "cpu-main", "title": "CPU Main"},
                    "sourcePath": "cpu__cpu-main.json",
                }
            ],
            datasources=[
                {
                    "kind": "datasource",
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "title": "Prometheus Main",
                    "body": {"uid": "prom-main", "name": "Prometheus Main"},
                    "sourcePath": "datasources.json",
                }
            ],
            folders=[
                {
                    "kind": "folder",
                    "uid": "ops",
                    "title": "Operations",
                    "body": {"title": "Operations"},
                    "sourcePath": "folders.json",
                }
            ],
            alerting={
                "summary": {
                    "ruleCount": 1,
                    "contactPointCount": 1,
                    "muteTimingCount": 0,
                    "policyCount": 1,
                    "templateCount": 0,
                },
                "rules": [
                    {
                        "sourcePath": "rules/rule.json",
                        "document": {"kind": "grafana-alert-rule"},
                    }
                ],
            },
            metadata={"dashboardExportDir": "./dashboards/raw"},
        )

        self.assertEqual(document["kind"], gitops_sync.SYNC_SOURCE_BUNDLE_KIND)
        self.assertEqual(document["summary"]["dashboardCount"], 1)
        self.assertEqual(document["summary"]["datasourceCount"], 1)
        self.assertEqual(document["summary"]["folderCount"], 1)
        self.assertEqual(document["summary"]["alertRuleCount"], 1)
        self.assertEqual(document["alerts"], [])
        self.assertEqual(document["metadata"]["dashboardExportDir"], "./dashboards/raw")

    def test_gitops_sync_build_sync_plan_tracks_create_update_noop_and_unmanaged(self):
        desired = [
            {
                "kind": "dashboard",
                "uid": "dash-prod",
                "title": "Prod Overview",
                "body": {"folderUid": "ops", "version": 2},
                "sourcePath": "dashboards/prod-overview.json",
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "prom-main",
                "body": {"type": "prometheus", "url": "http://prometheus-v2:9090"},
                "sourcePath": "datasources/prom-main.json",
            },
            {
                "kind": "folder",
                "uid": "ops",
                "title": "Operations",
                "body": {"title": "Operations"},
                "sourcePath": "folders/ops.json",
            },
            {
                "kind": "alert",
                "uid": "cpu-burn",
                "title": "CPU Burn",
                "managedFields": ["condition", "labels"],
                "body": {
                    "condition": "A > 90",
                    "labels": {"severity": "warning"},
                },
                "sourcePath": "alerts/cpu-burn.json",
            },
        ]
        live = [
            {
                "kind": "dashboard",
                "uid": "dash-prod",
                "title": "Prod Overview",
                "body": {"folderUid": "ops", "version": 2},
            },
            {
                "kind": "datasource",
                "uid": "prom-main",
                "name": "prom-main",
                "body": {"type": "prometheus", "url": "http://prometheus:9090"},
            },
            {
                "kind": "alert",
                "uid": "cpu-burn",
                "title": "CPU Burn",
                "managedFields": ["condition", "labels"],
                "body": {
                    "condition": "A > 90",
                    "labels": {"severity": "warning"},
                    "annotations": {"runbook": "ignored"},
                },
            },
            {
                "kind": "folder",
                "uid": "legacy",
                "title": "Legacy",
                "body": {"title": "Legacy"},
            },
        ]

        plan = gitops_sync.build_sync_plan(
            desired, live, allow_prune=False, dry_run=True
        )
        actions = {
            (operation.kind, operation.identity): operation.action
            for operation in plan.operations
        }

        self.assertEqual(actions[("dashboard", "dash-prod")], "noop")
        self.assertEqual(actions[("datasource", "prom-main")], "would-update")
        self.assertEqual(actions[("folder", "ops")], "would-create")
        self.assertEqual(actions[("alert", "cpu-burn")], "noop")
        self.assertEqual(actions[("folder", "legacy")], "unmanaged")
        self.assertEqual(
            plan.summary,
            {
                "would_create": 1,
                "would_update": 1,
                "would_delete": 0,
                "noop": 2,
                "unmanaged": 1,
            },
        )
        document = gitops_sync.plan_to_document(plan)
        self.assertEqual(document["summary"]["alert_candidate"], 1)
        self.assertEqual(document["summary"]["alert_plan_only"], 0)
        self.assertEqual(document["summary"]["alert_blocked"], 0)
        self.assertEqual(
            document["alertAssessment"]["alerts"][0]["status"], "candidate"
        )
        alert_operation = {
            (operation["kind"], operation["identity"]): operation
            for operation in document["operations"]
        }[("alert", "cpu-burn")]
        self.assertEqual(
            alert_operation["managedFields"],
            ["condition", "labels"],
        )

    def test_gitops_sync_build_sync_plan_rejects_duplicate_desired_identity(self):
        desired = [
            {"kind": "folder", "uid": "ops", "title": "Operations"},
            {"kind": "folder", "uid": "ops", "title": "Operations Copy"},
        ]

        with self.assertRaises(GrafanaError) as error:
            gitops_sync.build_sync_plan(desired, [])

        self.assertIn("Duplicate sync identity", str(error.exception))

    def test_gitops_sync_alert_spec_requires_explicit_managed_fields(self):
        with self.assertRaises(GrafanaError) as error:
            gitops_sync.normalize_resource_spec(
                {
                    "kind": "alert",
                    "uid": "cpu-burn",
                    "title": "CPU Burn",
                    "body": {"condition": "A > 90"},
                }
            )

        self.assertIn("managedFields", str(error.exception))

    def test_gitops_sync_build_apply_intent_blocks_live_apply_until_reviewed_and_approved(
        self,
    ):
        plan = gitops_sync.build_sync_plan(
            desired_specs=[{"kind": "folder", "uid": "ops", "title": "Operations"}],
            live_specs=[],
            dry_run=False,
            review_required=True,
        )

        with self.assertRaises(GrafanaError) as not_reviewed_error:
            gitops_sync.build_apply_intent(plan, approve=True)
        self.assertIn("marked reviewed", str(not_reviewed_error.exception))

        reviewed_plan = gitops_sync.mark_plan_reviewed(plan)

        with self.assertRaises(GrafanaError) as not_approved_error:
            gitops_sync.build_apply_intent(reviewed_plan, approve=False)
        self.assertIn("explicit approval", str(not_approved_error.exception))

        intent = gitops_sync.build_apply_intent(reviewed_plan, approve=True)
        self.assertEqual(intent["mode"], "apply")
        self.assertEqual(len(intent["operations"]), 1)
        self.assertEqual(intent["operations"][0].action, "would-create")

    def test_gitops_sync_mark_plan_reviewed_rejects_unexpected_token(self):
        plan = gitops_sync.build_sync_plan(
            desired_specs=[{"kind": "folder", "uid": "ops", "title": "Operations"}],
            live_specs=[],
            dry_run=False,
            review_required=True,
        )

        with self.assertRaises(GrafanaError) as error:
            gitops_sync.mark_plan_reviewed(plan, review_token="wrong-token")

        self.assertIn("review token rejected", str(error.exception))

    def test_gitops_sync_plan_to_document_marks_plan_only_and_blocked_alerts_in_summary(
        self,
    ):
        plan = gitops_sync.build_sync_plan(
            desired_specs=[
                {
                    "kind": "alert",
                    "uid": "cpu-high",
                    "title": "CPU High",
                    "managedFields": ["condition", "contactPoints"],
                    "body": {
                        "condition": "A > 90",
                        "contactPoints": ["pagerduty-primary"],
                    },
                },
                {
                    "kind": "alert",
                    "uid": "disk-high",
                    "title": "Disk High",
                    "managedFields": ["labels"],
                    "body": {
                        "condition": "A > 80",
                        "labels": {"severity": "warning"},
                    },
                },
            ],
            live_specs=[],
            dry_run=True,
        )

        document = gitops_sync.plan_to_document(plan)

        self.assertEqual(document["summary"]["alert_candidate"], 0)
        self.assertEqual(document["summary"]["alert_plan_only"], 1)
        self.assertEqual(document["summary"]["alert_blocked"], 1)
        self.assertEqual(len(document["alertAssessment"]["alerts"]), 2)

    def test_gitops_sync_render_sync_source_bundle_text_renders_summary(self):
        lines = gitops_sync.render_sync_source_bundle_text(
            {
                "kind": gitops_sync.SYNC_SOURCE_BUNDLE_KIND,
                "summary": {
                    "dashboardCount": 2,
                    "datasourceCount": 1,
                    "folderCount": 1,
                    "alertRuleCount": 3,
                    "contactPointCount": 1,
                    "muteTimingCount": 1,
                    "policyCount": 1,
                    "templateCount": 2,
                },
            }
        )

        self.assertEqual(lines[0], "Sync source bundle")
        self.assertIn("Dashboards: 2", lines[1])
        self.assertIn("Alerting: rules=3", lines[4])


if __name__ == "__main__":
    unittest.main()
