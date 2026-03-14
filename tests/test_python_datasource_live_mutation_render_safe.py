import ast
import importlib
import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "live_mutation_render_safe.py"
render_safe = importlib.import_module("grafana_utils.datasource.live_mutation_render_safe")
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class DatasourceLiveMutationRenderSafeTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_validate_columns_rejects_unknown_values(self):
        with self.assertRaisesRegex(GrafanaError, "Unsupported live mutation dry-run column"):
            render_safe.validate_columns(["uid", "bad_column"])

    def test_render_table_uses_selected_columns(self):
        lines = render_safe.render_live_mutation_dry_run_table(
            [{"uid": "prom-main", "action": "would-create"}],
            columns=["uid", "action"],
        )

        self.assertEqual(lines[0], "UID        ACTION      ")
        self.assertIn("would-create", lines[2])

    def test_render_json_counts_any_would_fail_action_as_blocked(self):
        document = render_safe.render_live_mutation_dry_run_json(
            [
                {"action": "would-create"},
                {"action": "would-fail-existing-name"},
                {"action": "would-fail-ambiguous-name"},
            ]
        )

        payload = json.loads(document)
        self.assertEqual(payload["summary"]["createCount"], 1)
        self.assertEqual(payload["summary"]["blockedCount"], 2)
