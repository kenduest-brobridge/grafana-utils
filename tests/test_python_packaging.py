import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
PYPROJECT_PATH = REPO_ROOT / "pyproject.toml"
MODULE_ENTRYPOINT_PATH = REPO_ROOT / "grafana_utils" / "__main__.py"
POETRY_LOCK_PATH = REPO_ROOT / "poetry.lock"


class PackagingTests(unittest.TestCase):
    def test_pyproject_exists(self):
        self.assertTrue(PYPROJECT_PATH.is_file())

    def test_pyproject_declares_console_scripts(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertRegex(content, r"(?m)^\[project\.scripts\]$")
        self.assertRegex(
            content, r'(?m)^grafana-util = "grafana_utils\.unified_cli:main"$'
        )
        self.assertNotRegex(content, r"(?m)^grafana-access-utils = ")

    def test_pyproject_declares_base_requests_dependency(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn("requests>=2.27,<3", content)

    def test_pyproject_requires_python39_or_newer(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requires-python = ">=3.9"', content)

    def test_pyproject_finds_package_submodules(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('include = ["grafana_utils", "grafana_utils.*"]', content)

    def test_pyproject_declares_poetry_dev_group(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertRegex(content, r"(?m)^\[tool\.poetry\]$")
        self.assertRegex(content, r'(?m)^requires-poetry = ">=2\.1"$')
        self.assertRegex(content, r"(?m)^\[tool\.poetry\.group\.dev\.dependencies\]$")
        self.assertIn('black = ">=24,<26"', content)
        self.assertIn('build = ">=1.2,<2"', content)
        self.assertIn('mypy = ">=1.10,<2"', content)
        self.assertIn('ruff = ">=0.11,<1"', content)
        self.assertIn('setuptools = ">=80.9"', content)
        self.assertIn('wheel = ">=0.45,<1"', content)

    def test_package_declares_module_entrypoint(self):
        self.assertTrue(MODULE_ENTRYPOINT_PATH.is_file())

    def test_repo_commits_poetry_lock(self):
        self.assertTrue(POETRY_LOCK_PATH.is_file())


if __name__ == "__main__":
    unittest.main()
