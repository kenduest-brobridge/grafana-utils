import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "check_ai_workflow.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("check_ai_workflow", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("check_ai_workflow", module)
    spec.loader.exec_module(module)
    return module


def write_trace_files(root: Path, status_entries: int, change_entries: int) -> None:
    internal = root / "docs" / "internal"
    internal.mkdir(parents=True, exist_ok=True)
    (internal / "ai-status.md").write_text(
        "# ai-status.md\n\n"
        + "\n".join(
            f"## 2026-04-{index:02d} - Status {index}\n- State: Done\n"
            for index in range(status_entries, 0, -1)
        ),
        encoding="utf-8",
    )
    (internal / "ai-changes.md").write_text(
        "# ai-changes.md\n\n"
        + "\n".join(
            f"## 2026-04-{index:02d} - Change {index}\n- Summary: Done\n"
            for index in range(change_entries, 0, -1)
        ),
        encoding="utf-8",
    )


class CheckAiWorkflowTests(unittest.TestCase):
    def test_html_output_requires_source_or_generator_change(self):
        module = load_module()

        errors = module.validate_paths(["docs/html/index.html"], check_trace_size=False)

        self.assertEqual(len(errors), 1)
        self.assertIn("docs/html output", errors[0])

    def test_html_output_allows_command_source_change(self):
        module = load_module()

        errors = module.validate_paths(
            [
                "docs/html/index.html",
                "docs/commands/en/dashboard-export.md",
            ],
            check_trace_size=False,
        )

        self.assertEqual(errors, [])

    def test_man_output_requires_command_source_or_generator_change(self):
        module = load_module()

        errors = module.validate_paths(
            ["docs/man/grafana-util-dashboard.1"],
            check_trace_size=False,
        )

        self.assertEqual(len(errors), 1)
        self.assertIn("docs/man output", errors[0])

    def test_man_output_allows_version_bump(self):
        module = load_module()

        errors = module.validate_paths(
            [
                "VERSION",
                "docs/man/grafana-util-dashboard.1",
            ],
            check_trace_size=False,
        )

        self.assertEqual(errors, [])

    def test_meaningful_internal_doc_requires_both_trace_files(self):
        module = load_module()

        errors = module.validate_paths(
            ["docs/internal/generated-docs-architecture.md"],
            check_trace_size=False,
        )

        self.assertEqual(len(errors), 1)
        self.assertIn("ai-status.md", errors[0])
        self.assertIn("ai-changes.md", errors[0])

    def test_meaningful_internal_doc_allows_trace_updates(self):
        module = load_module()

        errors = module.validate_paths(
            [
                "docs/internal/generated-docs-architecture.md",
                "docs/internal/ai-status.md",
                "docs/internal/ai-changes.md",
            ],
            check_trace_size=False,
        )

        self.assertEqual(errors, [])

    def test_workspace_noise_paths_are_rejected(self):
        module = load_module()

        errors = module.validate_paths(
            ["test-results/alert-export.json", "scratch/note.md"],
            check_trace_size=False,
        )

        self.assertEqual(len(errors), 1)
        self.assertIn("workspace noise paths", errors[0])
        self.assertIn("test-results/alert-export.json", errors[0])
        self.assertIn("scratch/note.md", errors[0])

    def test_workspace_noise_paths_can_still_be_detected_directly(self):
        module = load_module()

        self.assertTrue(module.is_workspace_noise_path("notes/local-review.md"))
        self.assertTrue(module.is_workspace_noise_path(".codex/task-brief.md"))
        self.assertFalse(module.is_workspace_noise_path("docs/internal/ai-workflow-note.md"))

    def test_trace_size_check_rejects_long_trace_files(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            write_trace_files(root, status_entries=7, change_entries=10)

            errors = module.validate_paths(["docs/internal/ai-status.md"], root=root)

            self.assertEqual(len(errors), 1)
            self.assertIn("docs/internal/ai-status.md has 7 entries", errors[0])

    def test_trace_size_check_allows_compact_trace_files(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            write_trace_files(root, status_entries=6, change_entries=10)

            errors = module.validate_paths(["docs/internal/ai-status.md"], root=root)

            self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
