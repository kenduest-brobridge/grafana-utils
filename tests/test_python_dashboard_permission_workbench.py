import unittest

from grafana_utils import dashboard_permission_workbench as permission_workbench


class DashboardPermissionWorkbenchTests(unittest.TestCase):
    def build_permissions(self):
        return [
            {
                "userId": 7,
                "userLogin": "alice",
                "permission": 1,
            },
            {
                "teamId": 9,
                "team": "SRE",
                "permission": 2,
                "inherited": True,
            },
            {
                "serviceAccountId": 11,
                "serviceAccountName": "robot",
                "permissionName": "Admin",
            },
            {
                "roleName": "Viewer",
                "permission": "View",
            },
        ]

    def build_bundle_resources(self):
        return [
            {
                "resourceKind": "folder",
                "resourceUid": "infra",
                "resourceTitle": "Infra",
                "permissions": [
                    {"teamId": 9, "team": "SRE", "permission": 2},
                    {"roleName": "Viewer", "permission": 1},
                ],
            },
            {
                "resourceKind": "dashboard",
                "resourceUid": "cpu-main",
                "resourceTitle": "CPU Main",
                "permissions": [
                    {"userId": 7, "userLogin": "alice", "permission": 1},
                ],
            },
        ]

    def test_dashboard_permission_build_permission_export_document_normalizes_subjects_and_levels(self):
        document = permission_workbench.build_permission_export_document(
            "dashboard",
            "cpu-main",
            "CPU Main",
            self.build_permissions(),
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_EXPORT_KIND)
        self.assertEqual(document["summary"]["permissionCount"], 4)
        self.assertEqual(document["summary"]["userCount"], 1)
        self.assertEqual(document["summary"]["teamCount"], 1)
        self.assertEqual(document["summary"]["serviceAccountCount"], 1)
        self.assertEqual(document["summary"]["roleCount"], 1)
        rows = {row["subjectKey"]: row for row in document["permissions"]}
        self.assertEqual(rows["user:7"]["permissionName"], "view")
        self.assertEqual(rows["team:9"]["permissionName"], "edit")
        self.assertEqual(rows["service-account:11"]["permissionName"], "admin")
        self.assertEqual(rows["role:Viewer"]["permission"], 1)

    def test_dashboard_permission_build_permission_diff_document_reports_missing_changed_and_extra(self):
        expected = permission_workbench.build_permission_export_document(
            "folder",
            "infra",
            "Infra",
            [
                {"userId": 7, "userLogin": "alice", "permission": 1},
                {"teamId": 9, "team": "SRE", "permission": 2},
            ],
        )
        actual = permission_workbench.build_permission_export_document(
            "folder",
            "infra",
            "Infra",
            [
                {"userId": 7, "userLogin": "alice", "permission": 2},
                {"serviceAccountId": 11, "serviceAccountName": "robot", "permission": 4},
            ],
        )
        diff = permission_workbench.build_permission_diff_document(expected, actual)
        self.assertEqual(diff["kind"], permission_workbench.PERMISSION_DIFF_KIND)
        self.assertEqual(diff["summary"]["changedCount"], 1)
        self.assertEqual(diff["summary"]["missingLiveCount"], 1)
        self.assertEqual(diff["summary"]["extraLiveCount"], 1)
        rows = {row["subjectKey"]: row for row in diff["rows"]}
        self.assertEqual(rows["user:7"]["status"], "changed")
        self.assertEqual(rows["team:9"]["status"], "missing-live")
        self.assertEqual(rows["service-account:11"]["status"], "extra-live")

    def test_dashboard_permission_render_permission_export_text_renders_summary_and_rows(self):
        document = permission_workbench.build_permission_export_document(
            "dashboard",
            "cpu-main",
            "CPU Main",
            self.build_permissions(),
        )
        output = "\n".join(permission_workbench.render_permission_export_text(document))
        self.assertIn("Permission export: dashboard uid=cpu-main title=CPU Main", output)
        self.assertIn(
            "Counts: 4 permissions, 1 users, 1 teams, 1 service-accounts, 1 roles",
            output,
        )
        self.assertIn("- user alice permission=view inherited=false", output)
        self.assertIn("- team SRE permission=edit inherited=true", output)

    def test_dashboard_permission_build_permission_preflight_document_reports_missing_subjects(self):
        export_document = permission_workbench.build_permission_export_document(
            "folder",
            "infra",
            "Infra",
            self.build_permissions(),
        )
        document = permission_workbench.build_permission_preflight_document(
            export_document,
            {
                "userIds": [7],
                "teamIds": [],
                "serviceAccountIds": [],
                "roles": ["Viewer"],
            },
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_PREFLIGHT_KIND)
        self.assertEqual(document["summary"]["checkCount"], 4)
        self.assertEqual(document["summary"]["missingCount"], 2)
        checks = {row["subjectKey"]: row for row in document["checks"]}
        self.assertEqual(checks["user:7"]["status"], "ok")
        self.assertEqual(checks["team:9"]["status"], "missing")
        self.assertEqual(checks["service-account:11"]["status"], "missing")
        self.assertEqual(checks["role:Viewer"]["status"], "ok")

    def test_dashboard_permission_build_permission_promotion_document_summarizes_drift(self):
        expected = permission_workbench.build_permission_export_document(
            "dashboard",
            "cpu-main",
            "CPU Main",
            [
                {"userId": 7, "userLogin": "alice", "permission": 1},
                {"teamId": 9, "team": "SRE", "permission": 2},
            ],
        )
        actual = permission_workbench.build_permission_export_document(
            "dashboard",
            "cpu-main",
            "CPU Main",
            [
                {"userId": 7, "userLogin": "alice", "permission": 2},
                {"serviceAccountId": 11, "serviceAccountName": "robot", "permission": 4},
            ],
        )
        document = permission_workbench.build_permission_promotion_document(
            expected, actual
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_PROMOTION_KIND)
        self.assertEqual(document["summary"]["wouldAddCount"], 1)
        self.assertEqual(document["summary"]["wouldChangeCount"], 1)
        self.assertEqual(document["summary"]["wouldLeaveExtraCount"], 1)

    def test_dashboard_permission_render_permission_preflight_text_renders_summary_and_rows(self):
        export_document = permission_workbench.build_permission_export_document(
            "folder",
            "infra",
            "Infra",
            self.build_permissions(),
        )
        document = permission_workbench.build_permission_preflight_document(
            export_document,
            {
                "userIds": [7],
                "teamIds": [],
                "serviceAccountIds": [],
                "roles": ["Viewer"],
            },
        )
        output = "\n".join(permission_workbench.render_permission_preflight_text(document))
        self.assertIn("Permission preflight summary", output)
        self.assertIn("Checks: 4 total, 2 ok, 2 missing, 2 blocking", output)
        self.assertIn("- user alice permission=view status=ok", output)
        self.assertIn("- team SRE permission=edit status=missing", output)

    def test_dashboard_permission_build_permission_bundle_document_summarizes_resources(self):
        document = permission_workbench.build_permission_bundle_document(
            self.build_bundle_resources()
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_BUNDLE_KIND)
        self.assertEqual(document["summary"]["resourceCount"], 2)
        self.assertEqual(document["summary"]["dashboardCount"], 1)
        self.assertEqual(document["summary"]["folderCount"], 1)
        self.assertEqual(document["summary"]["permissionCount"], 3)
        self.assertEqual(document["resources"][0]["resource"]["kind"], "dashboard")
        self.assertEqual(document["resources"][1]["resource"]["kind"], "folder")

    def test_dashboard_permission_build_permission_bundle_diff_document_summarizes_resource_drift(self):
        expected = permission_workbench.build_permission_bundle_document(
            self.build_bundle_resources()
        )
        actual = permission_workbench.build_permission_bundle_document(
            [
                {
                    "resourceKind": "folder",
                    "resourceUid": "infra",
                    "resourceTitle": "Infra",
                    "permissions": [
                        {"teamId": 9, "team": "SRE", "permission": 4},
                    ],
                },
                {
                    "resourceKind": "dashboard",
                    "resourceUid": "extra-main",
                    "resourceTitle": "Extra Main",
                    "permissions": [
                        {"userId": 8, "userLogin": "bob", "permission": 1},
                    ],
                },
            ]
        )
        document = permission_workbench.build_permission_bundle_diff_document(
            expected, actual
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_BUNDLE_DIFF_KIND)
        self.assertEqual(document["summary"]["resourceCount"], 3)
        self.assertEqual(document["summary"]["changedCount"], 1)
        self.assertEqual(document["summary"]["missingLiveCount"], 1)
        self.assertEqual(document["summary"]["extraLiveCount"], 1)

    def test_dashboard_permission_render_permission_bundle_text_renders_summary_and_resources(self):
        document = permission_workbench.build_permission_bundle_document(
            self.build_bundle_resources()
        )
        output = "\n".join(permission_workbench.render_permission_bundle_text(document))
        self.assertIn("Permission bundle summary", output)
        self.assertIn("Counts: 2 resources, 1 dashboards, 1 folders, 3 permissions", output)
        self.assertIn("- dashboard uid=cpu-main title=CPU Main permissions=1", output)
        self.assertIn("- folder uid=infra title=Infra permissions=2", output)

    def test_dashboard_permission_build_permission_remap_document_summarizes_uid_title_and_path_rewrites(self):
        bundle = permission_workbench.build_permission_bundle_document(
            self.build_bundle_resources()
        )
        document = permission_workbench.build_permission_remap_document(
            bundle,
            {
                "uidMap": {"dashboard:cpu-main": "prod-cpu-main"},
                "titleMap": {"folder:infra": "Platform / Infra"},
                "pathMap": {"folder:infra": "Platform / Infra"},
            },
        )
        self.assertEqual(document["kind"], permission_workbench.PERMISSION_REMAP_KIND)
        self.assertEqual(document["summary"]["resourceCount"], 2)
        self.assertEqual(document["summary"]["remappedCount"], 2)
        rows = {row["sourceUid"]: row for row in document["resources"]}
        self.assertEqual(rows["cpu-main"]["targetUid"], "prod-cpu-main")
        self.assertEqual(rows["infra"]["targetTitle"], "Platform / Infra")
        self.assertEqual(rows["infra"]["targetPath"], "Platform / Infra")

    def test_dashboard_permission_render_permission_remap_text_renders_summary_and_rows(self):
        bundle = permission_workbench.build_permission_bundle_document(
            self.build_bundle_resources()
        )
        document = permission_workbench.build_permission_remap_document(
            bundle,
            {
                "uidMap": {"dashboard:cpu-main": "prod-cpu-main"},
            },
        )
        output = "\n".join(permission_workbench.render_permission_remap_text(document))
        self.assertIn("Permission remap summary", output)
        self.assertIn("Counts: 2 resources, 1 remapped, 1 unchanged", output)
        self.assertIn("- dashboard cpu-main -> prod-cpu-main remapped=true", output)
        self.assertIn("- folder infra -> infra remapped=false", output)


if __name__ == "__main__":
    unittest.main()
