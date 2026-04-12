from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "ai_trace.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("ai_trace", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("ai_trace", module)
    spec.loader.exec_module(module)
    return module


def write_trace_files(root: Path, status_entries: int, change_entries: int) -> None:
    internal = root / "docs" / "internal"
    internal.mkdir(parents=True, exist_ok=True)
    (internal / "ai-status.md").write_text(
        "# ai-status.md\n\nCurrent AI-maintained status only.\n\n"
        + "\n".join(
            f"## 2026-04-{day:02d} - Status {day}\n- State: Done\n- Result: ok\n"
            for day in range(status_entries, 0, -1)
        ),
        encoding="utf-8",
    )
    (internal / "ai-changes.md").write_text(
        "# ai-changes.md\n\nCurrent AI change log only.\n\n"
        + "\n".join(
            f"## 2026-04-{day:02d} - Change {day}\n- Summary: ok\n- Tests: ok\n"
            for day in range(change_entries, 0, -1)
        ),
        encoding="utf-8",
    )


class AiTraceTests(unittest.TestCase):
    def test_add_inserts_status_and_change_entries_at_top(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            write_trace_files(root, status_entries=1, change_entries=1)

            result = module.main(
                [
                    "--root",
                    str(root),
                    "add",
                    "--title",
                    "Trace Tool",
                    "--scope",
                    "scripts/ai_trace.py",
                    "--summary",
                    "Added structured trace helper",
                    "--tests",
                    "python3 -m unittest -v python.tests.test_python_ai_trace",
                    "--impact",
                    "Trace maintenance workflow",
                    "--risk",
                    "Internal helper only",
                    "--follow-up",
                    "none",
                    "--date",
                    "2026-04-12",
                ]
            )

            self.assertEqual(result, 0)
            status_text = (root / "docs/internal/ai-status.md").read_text(encoding="utf-8")
            changes_text = (root / "docs/internal/ai-changes.md").read_text(encoding="utf-8")
            self.assertLess(
                status_text.index("## 2026-04-12 - Trace Tool"),
                status_text.index("## 2026-04-01 - Status 1"),
            )
            self.assertIn("- State: Done", status_text)
            self.assertIn("- Current Update: Added structured trace helper", status_text)
            self.assertLess(
                changes_text.index("## 2026-04-12 - Trace Tool"),
                changes_text.index("## 2026-04-01 - Change 1"),
            )
            self.assertIn("- Tests: python3 -m unittest -v python.tests.test_python_ai_trace", changes_text)
            self.assertIn("- Rollback/Risk: Internal helper only", changes_text)

    def test_compact_keeps_recent_entries_and_appends_archive(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            write_trace_files(root, status_entries=3, change_entries=4)
            archive = root / "docs/internal/archive/ai-status-archive-2026-04-12.md"
            archive.parent.mkdir(parents=True, exist_ok=True)
            archive.write_text("# existing archive\n\nExisting content\n", encoding="utf-8")

            result = module.main(
                [
                    "--root",
                    str(root),
                    "compact",
                    "--keep-status",
                    "1",
                    "--keep-changes",
                    "2",
                    "--date",
                    "2026-04-12",
                ]
            )

            self.assertEqual(result, 0)
            status_text = (root / "docs/internal/ai-status.md").read_text(encoding="utf-8")
            changes_text = (root / "docs/internal/ai-changes.md").read_text(encoding="utf-8")
            self.assertIn("## 2026-04-03 - Status 3", status_text)
            self.assertNotIn("## 2026-04-02 - Status 2", status_text)
            self.assertIn("ai-status-archive-2026-04-12.md", status_text)
            self.assertIn("## 2026-04-04 - Change 4", changes_text)
            self.assertIn("- Tests: ok\n\n## ", changes_text)
            self.assertIn("## 2026-04-03 - Change 3", changes_text)
            self.assertNotIn("## 2026-04-02 - Change 2", changes_text)
            archive_text = archive.read_text(encoding="utf-8")
            self.assertIn("Existing content", archive_text)
            self.assertIn("## 2026-04-02 - Status 2", archive_text)
            self.assertIn("- Result: ok\n\n## ", archive_text)
            change_archive = root / "docs/internal/archive/ai-changes-archive-2026-04-12.md"
            self.assertIn("## 2026-04-02 - Change 2", change_archive.read_text(encoding="utf-8"))

    def test_check_size_reports_over_limit_files(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            write_trace_files(root, status_entries=2, change_entries=3)

            errors = module.check_size(root, keep_status=1, keep_changes=5)

            self.assertEqual(len(errors), 1)
            self.assertIn("docs/internal/ai-status.md has 2 entries", errors[0])
            self.assertIn("python3 scripts/ai_trace.py compact", errors[0])


if __name__ == "__main__":
    unittest.main()
