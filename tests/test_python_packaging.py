import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
PYPROJECT_PATH = REPO_ROOT / "pyproject.toml"


class PackagingTests(unittest.TestCase):
    def test_pyproject_exists(self):
        self.assertTrue(PYPROJECT_PATH.is_file())

    def test_pyproject_declares_console_scripts(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertRegex(content, r'(?m)^\[project\.scripts\]$')
        self.assertRegex(content, r'(?m)^grafana-util = "grafana_utils\.unified_cli:main"$')
        self.assertNotRegex(content, r'(?m)^grafana-access-utils = ')

    def test_pyproject_declares_base_requests_dependency(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requests>=2.27,<3', content)

    def test_pyproject_requires_python39_or_newer(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requires-python = ">=3.9"', content)

    def test_pyproject_finds_package_submodules(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('include = ["grafana_utils", "grafana_utils.*"]', content)


if __name__ == "__main__":
    unittest.main()
