import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


class PublicDocsEvidenceTests(unittest.TestCase):
    def test_readmes_include_before_after_section(self):
        expectations = {
            REPO_ROOT / "README.md": "## Before / After",
            REPO_ROOT / "README.zh-TW.md": "## 採用前後對照",
        }
        for path, marker in expectations.items():
            text = path.read_text(encoding="utf-8")
            self.assertIn(marker, text, path.name)

    def test_what_is_pages_include_value_transition_section(self):
        expectations = {
            REPO_ROOT / "docs" / "user-guide" / "en" / "what-is-grafana-util.md": "## Before / After",
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "what-is-grafana-util.md": "## 採用前後對照",
        }
        for path, marker in expectations.items():
            text = path.read_text(encoding="utf-8")
            self.assertIn(marker, text, path.name)

    def test_handbook_pages_include_value_and_failure_sections(self):
        expectations = {
            REPO_ROOT / "docs" / "user-guide" / "en" / "index.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "index.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "getting-started.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "getting-started.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "role-new-user.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "role-new-user.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "role-sre-ops.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "role-sre-ops.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "role-automation-ci.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "role-automation-ci.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "dashboard.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "dashboard.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "datasource.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "datasource.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "alert.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "alert.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "access.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "access.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "change-overview-status.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "change-overview-status.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "scenarios.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "scenarios.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "recipes.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "recipes.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "troubleshooting.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "troubleshooting.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "architecture.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "architecture.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            for marker in markers:
                self.assertIn(marker, text, path.name)

    def test_high_value_command_pages_include_before_after_guidance(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "access-user.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "access-org.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "access-team.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-export.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-import.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "alert-plan.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-analyze-export.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "access-service-account-token.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "access.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "access-service-account.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "profile.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "snapshot.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "alert.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "alert-add-rule.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "alert-apply.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "datasource.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-topology.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-governance-gate.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-impact.md": "## Before / After",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access-user.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access-org.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access-team.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-export.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-import.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-plan.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-analyze-export.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access-service-account-token.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access-service-account.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "profile.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "snapshot.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-add-rule.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-apply.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "datasource.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-topology.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-governance-gate.md": "## 採用前後對照",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-impact.md": "## 採用前後對照",
        }
        for path, marker in expectations.items():
            text = path.read_text(encoding="utf-8")
            self.assertIn(marker, text, path.name)

    def test_command_root_pages_include_workflow_lane_maps(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard.md": "## Workflow lanes",
            REPO_ROOT / "docs" / "commands" / "en" / "alert.md": "## Workflow lanes",
            REPO_ROOT / "docs" / "commands" / "en" / "access.md": "## Workflow lanes",
            REPO_ROOT / "docs" / "commands" / "en" / "datasource.md": "## Workflow lanes",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard.md": "## 這一頁對應的工作流",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert.md": "## 這一頁對應的工作流",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access.md": "## 這一頁對應的工作流",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "datasource.md": "## 這一頁對應的工作流",
        }
        for path, marker in expectations.items():
            text = path.read_text(encoding="utf-8")
            self.assertIn(marker, text, path.name)

    def test_command_root_pages_group_related_commands_by_workflow(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard.md": ["### Browse and Inventory", "### Move", "### Review Before Mutate", "### Capture"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert.md": ["### Inspect", "### Move", "### Review Before Mutate", "### Related Surface"],
            REPO_ROOT / "docs" / "commands" / "en" / "access.md": ["### Inspect", "### Review Before Mutate"],
            REPO_ROOT / "docs" / "commands" / "en" / "datasource.md": ["### Inspect", "### Move", "### Review Before Mutate"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard.md": ["### 盤點", "### 分析與報表", "### 變更前檢查", "### 截圖與素材"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert.md": ["### 盤點", "### 搬移", "### 變更前檢查", "### 規則與路由撰寫"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "access.md": ["### 盤點", "### 服務帳號與 token"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "datasource.md": ["### 盤點", "### 搬移", "### 變更前檢查"],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            for marker in markers:
                self.assertIn(marker, text, path.name)

    def test_dashboard_export_docs_cover_history_artifacts(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-export.md": ["--include-history", "history/"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-export.md": ["--include-history", "history/"],
            REPO_ROOT / "docs" / "user-guide" / "en" / "dashboard.md": ["--include-history", "history/"],
            REPO_ROOT / "docs" / "user-guide" / "zh-TW" / "dashboard.md": ["--include-history", "history/"],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            for marker in markers:
                self.assertIn(marker, text, path.name)

    def test_dashboard_analyze_docs_use_canonical_analyze_and_output_names(self):
        expectations = {
            REPO_ROOT / "README.md": ["dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format dependency"],
            REPO_ROOT / "README.zh-TW.md": ["dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format dependency"],
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-analyze-live.md": ["dashboard analyze --url", "--output-format", "governance"],
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-analyze-export.md": ["dashboard analyze --input-dir", "--output-format", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-governance-gate.md": ["--url", "--input-dir", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-topology.md": ["--url", "--input-dir", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-impact.md": ["--url", "--input-dir", "governance-json"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-analyze-live.md": ["dashboard analyze --url", "--output-format", "governance"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-analyze-export.md": ["dashboard analyze --input-dir", "--output-format", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-governance-gate.md": ["--url", "--input-dir", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-topology.md": ["--url", "--input-dir", "governance-json", "queries-json"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-impact.md": ["--url", "--input-dir", "governance-json"],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("--report", text, path.name)
            for marker in markers:
                self.assertIn(marker, text, path.name)

    def test_dashboard_history_docs_cover_schema_help_and_contract_markers(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "dashboard-history.md": [
                "--help-schema",
                "schemaVersion",
                "grafana-util-dashboard-history-list",
                "grafana-util-dashboard-history-restore",
                "grafana-util-dashboard-history-export",
            ],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-history.md": [
                "--help-schema",
                "schemaVersion",
                "grafana-util-dashboard-history-list",
                "grafana-util-dashboard-history-restore",
                "grafana-util-dashboard-history-export",
            ],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            for marker in markers:
                self.assertIn(marker, text, path.name)

    def test_alert_command_pages_include_before_after_guidance(self):
        expectations = {
            REPO_ROOT / "docs" / "commands" / "en" / "alert-export.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-import.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-delete.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-set-route.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-preview-route.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-add-contact-point.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-new-contact-point.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-list-contact-points.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-list-rules.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-list-templates.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "en" / "alert-list-mute-timings.md": ["## Before / After", "## What success looks like", "## Failure checks"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-export.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-import.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-delete.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-set-route.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-preview-route.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-add-contact-point.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-new-contact-point.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-list-contact-points.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-list-rules.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-list-templates.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "alert-list-mute-timings.md": ["## 採用前後對照", "## 成功判準", "## 失敗時先檢查"],
        }
        for path, markers in expectations.items():
            text = path.read_text(encoding="utf-8")
            for marker in markers:
                self.assertIn(marker, text, path.name)


if __name__ == "__main__":
    unittest.main()
