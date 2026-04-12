from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "workspace_noise_auditor.py"


def load_module():
    spec = importlib.util.spec_from_file_location("workspace_noise_auditor", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    sys.modules.setdefault("workspace_noise_auditor", module)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class WorkspaceNoiseAuditorTests(unittest.TestCase):
    def test_discover_noise_paths_from_git_status_reports_visible_tmp_file(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            (root / "notes.tmp").write_text("scratch\n", encoding="utf-8")

            items = module.discover_noise_paths_from_git_status(root)

            self.assertEqual(len(items), 1)
            self.assertEqual(items[0].path.resolve(), (root / "notes.tmp").resolve())
            self.assertEqual(items[0].category, "artifact")

    def test_discover_noise_paths_from_git_status_skips_clean_git_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            (root / "README.md").write_text("ok\n", encoding="utf-8")

            items = module.discover_noise_paths_from_git_status(root)

            self.assertEqual(items, [])

    def test_discover_noise_paths_does_not_descend_into_known_build_dirs(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            target_dir = root / "rust" / "target"
            target_dir.mkdir(parents=True)
            (target_dir / "object.tmp").write_text("noise\n", encoding="utf-8")

            items = module.discover_noise_paths(root)

            self.assertEqual(len(items), 1)
            self.assertEqual(items[0].path.resolve(), target_dir.resolve())
            self.assertEqual(items[0].category, "directory")

    def test_discover_noise_paths_allows_html_template_files(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            template_dir = root / "scripts" / "templates"
            template_dir.mkdir(parents=True)
            (template_dir / "base.html.tmpl").write_text("{{ content }}\n", encoding="utf-8")

            items = module.discover_noise_paths(root)

            self.assertEqual(items, [])


if __name__ == "__main__":
    unittest.main()
