import ast
import importlib
import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "live_mutation_render.py"
render_utils = importlib.import_module("grafana_utils.datasource.live_mutation_render")


class DatasourceLiveMutationRenderTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_build_live_mutation_dry_run_record_for_add(self):
        record = render_utils.build_live_mutation_dry_run_record(
            "add",
            {"action": "would-create", "match": "missing", "target": None},
            spec={"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
        )

        self.assertEqual(
            record,
            {
                "operation": "add",
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "match": "missing",
                "action": "would-create",
                "targetId": "",
            },
        )

    def test_render_live_mutation_dry_run_table_renders_headers_and_rows(self):
        lines = render_utils.render_live_mutation_dry_run_table(
            [
                {
                    "operation": "delete",
                    "uid": "logs-main",
                    "name": "Loki Logs",
                    "type": "loki",
                    "match": "exists-uid",
                    "action": "would-delete",
                    "targetId": "9",
                }
            ]
        )

        self.assertEqual(lines[0], "OPERATION  UID        NAME       TYPE  MATCH       ACTION        TARGET_ID")
        self.assertIn("delete", lines[2])
        self.assertIn("would-delete", lines[2])

    def test_render_live_mutation_dry_run_table_can_omit_header(self):
        lines = render_utils.render_live_mutation_dry_run_table(
            [
                {
                    "operation": "add",
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "match": "missing",
                    "action": "would-create",
                    "targetId": "",
                }
            ],
            include_header=False,
        )

        self.assertEqual(len(lines), 1)
        self.assertIn("would-create", lines[0])

    def test_render_live_mutation_dry_run_json_summarizes_actions(self):
        document = render_utils.render_live_mutation_dry_run_json(
            [
                {"action": "would-create"},
                {"action": "would-delete"},
                {"action": "would-fail-existing"},
            ]
        )

        payload = json.loads(document)
        self.assertEqual(payload["summary"]["itemCount"], 3)
        self.assertEqual(payload["summary"]["createCount"], 1)
        self.assertEqual(payload["summary"]["deleteCount"], 1)
        self.assertEqual(payload["summary"]["blockedCount"], 1)
