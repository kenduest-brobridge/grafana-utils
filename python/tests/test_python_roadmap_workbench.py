import unittest

from grafana_utils import roadmap_workbench


class RoadmapWorkbenchTests(unittest.TestCase):
    def build_summary_document(self):
        return {
            "datasourceInventory": [
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "orgId": "1",
                    "referenceCount": 2,
                },
                {
                    "uid": "logs-main",
                    "name": "Logs Main",
                    "type": "loki",
                    "orgId": "1",
                    "referenceCount": 1,
                },
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                    "type": "tempo",
                    "orgId": "1",
                    "referenceCount": 0,
                },
            ]
        }

    def build_report_document(self):
        return {
            "queries": [
                {
                    "dashboardUid": "cpu-main",
                    "dashboardTitle": "CPU Main",
                    "folderPath": "General",
                    "panelId": "7",
                    "panelTitle": "CPU Usage",
                    "panelType": "timeseries",
                    "datasource": "prom-main",
                    "datasourceUid": "prom-main",
                },
                {
                    "dashboardUid": "cpu-main",
                    "dashboardTitle": "CPU Main",
                    "folderPath": "General",
                    "panelId": "8",
                    "panelTitle": "Logs",
                    "panelType": "logs",
                    "datasource": "logs-main",
                    "datasourceUid": "logs-main",
                },
                {
                    "dashboardUid": "cpu-main",
                    "dashboardTitle": "CPU Main",
                    "folderPath": "General",
                    "panelId": "8",
                    "panelTitle": "Logs",
                    "panelType": "logs",
                    "datasource": "logs-main",
                    "datasourceUid": "logs-main",
                },
            ]
        }

    def build_source_bundle(self):
        return {
            "environment": "staging",
            "dashboards": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "folderPath": "General",
                }
            ],
            "datasources": [
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                }
            ],
        }

    def build_target_inventory(self):
        return {
            "environment": "production",
            "dashboards": [
                {
                    "uid": "prod-cpu-main",
                    "title": "CPU Main Prod",
                }
            ],
            "datasources": [
                {
                    "uid": "prom-prod",
                    "name": "Prometheus Production",
                    "type": "prometheus",
                }
            ],
        }

    def test_roadmap_list_workbench_sections_returns_expected_order(self):
        self.assertEqual(
            roadmap_workbench.list_workbench_sections(),
            [
                roadmap_workbench.INSPECTION_AND_GOVERNANCE,
                roadmap_workbench.PROMOTION_AND_PREFLIGHT,
                roadmap_workbench.DECLARATIVE_SYNC_AND_GITOPS,
                roadmap_workbench.SECRET_HANDLING_AND_REDACTION,
            ],
        )

    def test_roadmap_build_workbench_index_groups_task_names_by_section(self):
        index = roadmap_workbench.build_workbench_index()
        self.assertIn(
            "dashboard-datasource-dependency-summary",
            index[roadmap_workbench.INSPECTION_AND_GOVERNANCE],
        )
        self.assertIn(
            "promote-command-workflow",
            index[roadmap_workbench.PROMOTION_AND_PREFLIGHT],
        )
        self.assertIn(
            "gitops-sync-plan-contract",
            index[roadmap_workbench.DECLARATIVE_SYNC_AND_GITOPS],
        )
        self.assertIn(
            "datasource-secret-placeholder-contract",
            index[roadmap_workbench.SECRET_HANDLING_AND_REDACTION],
        )

    def test_roadmap_iter_candidate_modules_deduplicates_modules(self):
        modules = list(
            roadmap_workbench.iter_candidate_modules(
                roadmap_workbench.INSPECTION_AND_GOVERNANCE
            )
        )
        self.assertEqual(len(modules), len(set(modules)))
        self.assertIn("grafana_utils.dashboards.inspection_report", modules)

    def test_roadmap_iter_candidate_modules_exposes_new_gitops_and_secret_modules(self):
        gitops_modules = list(
            roadmap_workbench.iter_candidate_modules(
                roadmap_workbench.DECLARATIVE_SYNC_AND_GITOPS
            )
        )
        secret_modules = list(
            roadmap_workbench.iter_candidate_modules(
                roadmap_workbench.SECRET_HANDLING_AND_REDACTION
            )
        )
        self.assertIn("grafana_utils.gitops_sync", gitops_modules)
        self.assertIn(
            "grafana_utils.datasource_secret_workbench",
            secret_modules,
        )

    def test_roadmap_build_dependency_graph_document_builds_stable_graph_shape(self):
        document = roadmap_workbench.build_dependency_graph_document(
            self.build_summary_document(),
            self.build_report_document(),
        )
        self.assertEqual(document["kind"], roadmap_workbench.DEPENDENCY_GRAPH_KIND)
        self.assertEqual(document["schemaVersion"], 1)
        self.assertEqual(document["summary"]["dashboardCount"], 1)
        self.assertEqual(document["summary"]["panelCount"], 2)
        self.assertEqual(document["summary"]["datasourceCount"], 3)

        node_ids = {item["id"] for item in document["nodes"]}
        self.assertIn("dashboard:cpu-main", node_ids)
        self.assertIn("panel:cpu-main:7", node_ids)
        self.assertIn("datasource:unused-main", node_ids)

        datasource_nodes = {
            item["id"]: item
            for item in document["nodes"]
            if item["type"] == "datasource"
        }
        self.assertEqual(
            datasource_nodes["datasource:prom-main"]["datasourceType"], "prometheus"
        )
        self.assertEqual(
            datasource_nodes["datasource:unused-main"]["referenceCount"], 0
        )

        edges = {
            (item["source"], item["relation"], item["target"]): item
            for item in document["edges"]
        }
        self.assertIn(
            ("dashboard:cpu-main", "contains-panel", "panel:cpu-main:7"),
            edges,
        )
        self.assertEqual(
            edges[("panel:cpu-main:8", "queries-datasource", "datasource:logs-main")][
                "queryCount"
            ],
            2,
        )

    def test_roadmap_render_dependency_graph_dot_renders_nodes_and_aggregated_edges(
        self,
    ):
        document = roadmap_workbench.build_dependency_graph_document(
            self.build_summary_document(),
            self.build_report_document(),
        )
        dot = roadmap_workbench.render_dependency_graph_dot(document)
        self.assertIn("digraph grafana_dependency_graph {", dot)
        self.assertIn(
            '"dashboard:cpu-main" [label="CPU Main", shape="folder"];',
            dot,
        )
        self.assertIn(
            '"datasource:unused-main" [label="Unused Main", shape="cylinder"];',
            dot,
        )
        self.assertIn(
            '"dashboard:cpu-main" -> "panel:cpu-main:7" [label="contains-panel"];',
            dot,
        )
        self.assertIn(
            '"panel:cpu-main:8" -> "datasource:logs-main" [label="queries-datasource (2)"];',
            dot,
        )

    def test_roadmap_build_dependency_graph_governance_summary_reports_blast_radius_and_orphans(
        self,
    ):
        document = roadmap_workbench.build_dependency_graph_document(
            self.build_summary_document(),
            self.build_report_document(),
        )
        summary = roadmap_workbench.build_dependency_graph_governance_summary(document)

        self.assertEqual(summary["summary"]["dashboardCount"], 1)
        self.assertEqual(summary["summary"]["panelCount"], 2)
        self.assertEqual(summary["summary"]["datasourceCount"], 3)
        self.assertEqual(summary["summary"]["orphanedDatasourceCount"], 1)

        blast_radius = {
            row["datasourceUid"]: row for row in summary["datasourceBlastRadius"]
        }
        self.assertEqual(blast_radius["logs-main"]["dashboardCount"], 1)
        self.assertEqual(blast_radius["logs-main"]["panelCount"], 1)
        self.assertEqual(
            blast_radius["prom-main"]["panelNodeIds"], ["panel:cpu-main:7"]
        )

        orphaned = summary["orphanedDatasources"]
        self.assertEqual(len(orphaned), 1)
        self.assertEqual(orphaned[0]["datasourceUid"], "unused-main")
        self.assertEqual(orphaned[0]["dashboardCount"], 0)

    def test_roadmap_build_promotion_plan_document_tracks_create_update_and_remap_inputs(
        self,
    ):
        document = roadmap_workbench.build_promotion_plan_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            options={
                "dashboardUidMap": {"cpu-main": "prod-cpu-main"},
                "datasourceUidMap": {"prom-main": "prom-prod"},
                "datasourceNameMap": {"Prometheus Main": "Prometheus Production"},
            },
        )
        self.assertEqual(document["kind"], roadmap_workbench.PROMOTION_PLAN_KIND)
        self.assertEqual(document["summary"]["sourceEnvironment"], "staging")
        self.assertEqual(document["summary"]["targetEnvironment"], "production")
        self.assertEqual(document["summary"]["itemCount"], 2)
        self.assertEqual(document["summary"]["updateCount"], 2)
        dashboard_item = [
            item
            for item in document["planItems"]
            if item["resourceType"] == "dashboard"
        ][0]
        self.assertEqual(dashboard_item["targetUid"], "prod-cpu-main")
        datasource_item = [
            item
            for item in document["planItems"]
            if item["resourceType"] == "datasource"
        ][0]
        self.assertEqual(datasource_item["targetUid"], "prom-prod")
        self.assertEqual(
            document["options"]["datasourceNameMap"]["Prometheus Main"],
            "Prometheus Production",
        )

    def test_roadmap_build_preflight_check_document_reports_missing_dependencies(self):
        plan_document = roadmap_workbench.build_promotion_plan_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            options={
                "dashboardUidMap": {"cpu-main": "prod-cpu-main"},
                "datasourceUidMap": {"prom-main": "prom-prod"},
            },
        )
        preflight = roadmap_workbench.build_preflight_check_document(
            plan_document,
            availability={
                "datasourceUids": [],
                "pluginIds": ["grafana-piechart-panel"],
                "requiredPluginIds": [
                    "grafana-piechart-panel",
                    "grafana-clock-panel",
                ],
                "contactPoints": [],
                "requiredContactPoints": ["pagerduty-primary"],
                "libraryPanels": ["libcpu"],
                "requiredLibraryPanels": ["libcpu", "liblogs"],
            },
        )
        self.assertEqual(preflight["kind"], roadmap_workbench.PREFLIGHT_CHECK_KIND)
        self.assertEqual(preflight["summary"]["checkCount"], 6)
        self.assertEqual(preflight["summary"]["missingCount"], 4)
        self.assertEqual(preflight["summary"]["blockingCount"], 4)
        checks = {
            (item["kind"], item["resourceUid"]): item for item in preflight["checks"]
        }
        self.assertEqual(checks[("datasource", "prom-prod")]["status"], "missing")
        self.assertEqual(checks[("plugin", "grafana-piechart-panel")]["status"], "ok")
        self.assertEqual(checks[("plugin", "grafana-clock-panel")]["status"], "missing")
        self.assertEqual(
            checks[("contact-point", "pagerduty-primary")]["status"], "missing"
        )
        self.assertEqual(checks[("library-panel", "libcpu")]["status"], "ok")

    def test_roadmap_render_promotion_plan_text_renders_summary_and_items(self):
        document = roadmap_workbench.build_promotion_plan_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            options={
                "dashboardUidMap": {"cpu-main": "prod-cpu-main"},
                "datasourceUidMap": {"prom-main": "prom-prod"},
            },
        )
        output = "\n".join(roadmap_workbench.render_promotion_plan_text(document))
        self.assertIn("Promotion plan: staging -> production", output)
        self.assertIn(
            "Items: 2 total, 0 create, 2 update, 2 preflight-required", output
        )
        self.assertIn(
            "- dashboard uid=cpu-main target=prod-cpu-main action=update", output
        )
        self.assertIn(
            "- datasource uid=prom-main target=prom-prod action=update", output
        )

    def test_roadmap_render_preflight_check_text_renders_summary_and_status_rows(self):
        plan_document = roadmap_workbench.build_promotion_plan_document(
            self.build_source_bundle(),
            self.build_target_inventory(),
            options={
                "dashboardUidMap": {"cpu-main": "prod-cpu-main"},
                "datasourceUidMap": {"prom-main": "prom-prod"},
            },
        )
        preflight = roadmap_workbench.build_preflight_check_document(
            plan_document,
            availability={
                "datasourceUids": [],
                "pluginIds": ["grafana-piechart-panel"],
                "requiredPluginIds": [
                    "grafana-piechart-panel",
                    "grafana-clock-panel",
                ],
                "contactPoints": [],
                "requiredContactPoints": ["pagerduty-primary"],
                "libraryPanels": ["libcpu"],
                "requiredLibraryPanels": ["libcpu", "liblogs"],
            },
        )
        output = "\n".join(roadmap_workbench.render_preflight_check_text(preflight))
        self.assertIn("Promotion preflight summary", output)
        self.assertIn("Checks: 6 total, 2 ok, 4 missing, 4 blocking", output)
        self.assertIn(
            "- datasource uid=prom-prod status=missing detail=Target datasource is not available in the destination inventory.",
            output,
        )
        self.assertIn(
            "- plugin uid=grafana-piechart-panel status=ok detail=Plugin is available in the destination environment.",
            output,
        )


if __name__ == "__main__":
    unittest.main()
