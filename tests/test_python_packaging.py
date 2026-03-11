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
        self.assertRegex(content, r'(?m)^grafana-utils = "grafana_utils\.dashboard_cli:main"$')
        self.assertRegex(content, r'(?m)^grafana-alert-utils = "grafana_utils\.alert_cli:main"$')
        self.assertRegex(content, r'(?m)^grafana-access-utils = "grafana_utils\.access_cli:main"$')

    def test_pyproject_declares_base_requests_dependency(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requests>=2.27,<3', content)


if __name__ == "__main__":
    unittest.main()
