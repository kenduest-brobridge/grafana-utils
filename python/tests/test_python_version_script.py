import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
SET_VERSION_SCRIPT = REPO_ROOT / "scripts" / "set-version.sh"


class SetVersionScriptTests(unittest.TestCase):
    def _make_fixture(self):
        temp_dir = tempfile.TemporaryDirectory()
        root = Path(temp_dir.name)
        (root / "python").mkdir()
        (root / "rust").mkdir()
        (root / "VERSION").write_text("0.2.17\n", encoding="utf-8")
        (root / "python" / "pyproject.toml").write_text(
            textwrap.dedent("""
                [project]
                version = "0.2.17"
                """).lstrip(),
            encoding="utf-8",
        )
        (root / "rust" / "Cargo.toml").write_text(
            textwrap.dedent("""
                [package]
                name = "grafana-utils-rust"
                version = "0.2.17"
                """).lstrip(),
            encoding="utf-8",
        )
        (root / "rust" / "Cargo.lock").write_text(
            textwrap.dedent("""
                version = 4

                [[package]]
                name = "grafana-utils-rust"
                version = "0.2.17"
                """).lstrip(),
            encoding="utf-8",
        )
        return temp_dir, root

    def _run_script(self, root: Path, *args: str) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.update(
            {
                "REPO_ROOT_OVERRIDE": str(root),
                "VERSION_FILE_OVERRIDE": str(root / "VERSION"),
                "PYPROJECT_TOML_OVERRIDE": str(root / "python" / "pyproject.toml"),
                "CARGO_TOML_OVERRIDE": str(root / "rust" / "Cargo.toml"),
                "CARGO_LOCK_OVERRIDE": str(root / "rust" / "Cargo.lock"),
            }
        )
        return subprocess.run(
            ["bash", str(SET_VERSION_SCRIPT), *args],
            cwd=str(REPO_ROOT),
            text=True,
            capture_output=True,
            env=env,
            check=False,
        )

    def test_version_script_release_version_updates_all_version_files(self):
        temp_dir, root = self._make_fixture()
        self.addCleanup(temp_dir.cleanup)

        result = self._run_script(root, "--version", "0.2.18")

        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn(
            'version = "0.2.18"',
            (root / "python" / "pyproject.toml").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.18"',
            (root / "rust" / "Cargo.toml").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.18"',
            (root / "rust" / "Cargo.lock").read_text(encoding="utf-8"),
        )
        self.assertEqual(
            (root / "VERSION").read_text(encoding="utf-8").strip(), "0.2.18"
        )

    def test_version_script_sync_from_file_accepts_python_dev_notation(self):
        temp_dir, root = self._make_fixture()
        self.addCleanup(temp_dir.cleanup)
        (root / "VERSION").write_text("0.2.19.dev3\n", encoding="utf-8")

        result = self._run_script(root, "--sync-from-file")

        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn(
            'version = "0.2.19.dev3"',
            (root / "python" / "pyproject.toml").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.19-dev.3"',
            (root / "rust" / "Cargo.toml").read_text(encoding="utf-8"),
        )
        self.assertIn(
            'version = "0.2.19-dev.3"',
            (root / "rust" / "Cargo.lock").read_text(encoding="utf-8"),
        )

    def test_version_script_dry_run_leaves_files_unchanged(self):
        temp_dir, root = self._make_fixture()
        self.addCleanup(temp_dir.cleanup)
        before = (root / "python" / "pyproject.toml").read_text(encoding="utf-8")

        result = self._run_script(root, "--version", "0.2.20", "--dry-run")

        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertEqual(
            (root / "python" / "pyproject.toml").read_text(encoding="utf-8"),
            before,
        )
        self.assertEqual(
            (root / "VERSION").read_text(encoding="utf-8").strip(), "0.2.17"
        )


if __name__ == "__main__":
    unittest.main()
