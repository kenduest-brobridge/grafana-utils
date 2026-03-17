import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MAKEFILE_PATH = REPO_ROOT / "Makefile"
CI_WORKFLOW_PATH = REPO_ROOT / ".github" / "workflows" / "ci.yml"
PYPROJECT_PATH = REPO_ROOT / "pyproject.toml"
MODULE_ENTRYPOINT_PATH = REPO_ROOT / "grafana_utils" / "__main__.py"
POETRY_LOCK_PATH = REPO_ROOT / "poetry.lock"
SET_VERSION_SCRIPT_PATH = REPO_ROOT / "scripts" / "set-version.sh"
BUILD_RUST_MACOS_ARM64_SCRIPT_PATH = REPO_ROOT / "scripts" / "build-rust-macos-arm64.sh"
VERSION_PATH = REPO_ROOT / "VERSION"


class PackagingTests(unittest.TestCase):
    def test_packaging_pyproject_exists(self):
        self.assertTrue(PYPROJECT_PATH.is_file())

    def test_packaging_repo_commits_makefile_version_sources(self):
        self.assertTrue(MAKEFILE_PATH.is_file())
        self.assertTrue(SET_VERSION_SCRIPT_PATH.is_file())
        self.assertTrue(VERSION_PATH.is_file())

    def test_packaging_ci_python_quality_installs_project_runtime_dependencies(self):
        content = CI_WORKFLOW_PATH.read_text(encoding="utf-8")

        self.assertIn("python3 -m pip install --upgrade pip .", content)

    def test_packaging_pyproject_declares_console_scripts(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertRegex(content, r'(?m)^\[project\.scripts\]$')
        self.assertRegex(content, r'(?m)^grafana-util = "grafana_utils\.unified_cli:main"$')
        self.assertNotRegex(content, r'(?m)^grafana-access-utils = ')

    def test_packaging_pyproject_declares_base_requests_dependency(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requests>=2.27,<3', content)

    def test_packaging_pyproject_requires_python39_or_newer(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('requires-python = ">=3.9"', content)

    def test_packaging_pyproject_finds_package_submodules(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertIn('include = ["grafana_utils", "grafana_utils.*"]', content)

    def test_packaging_pyproject_declares_poetry_dev_group(self):
        content = PYPROJECT_PATH.read_text(encoding="utf-8")

        self.assertRegex(content, r"(?m)^\[tool\.poetry\]$")
        self.assertRegex(content, r'(?m)^requires-poetry = ">=2\.1"$')
        self.assertRegex(content, r"(?m)^\[tool\.poetry\.group\.dev\.dependencies\]$")
        self.assertIn('black = ">=24,<26"', content)
        self.assertIn('build = ">=1.2,<2"', content)
        self.assertIn('mypy = ">=1.10,<2"', content)
        self.assertIn('ruff = ">=0.11,<1"', content)
        self.assertIn('setuptools = ">=59"', content)
        self.assertIn('wheel = ">=0.45,<1"', content)

    def test_packaging_package_declares_module_entrypoint(self):
        self.assertTrue(MODULE_ENTRYPOINT_PATH.is_file())

    def test_packaging_repo_commits_poetry_lock(self):
        self.assertTrue(POETRY_LOCK_PATH.is_file())

    def test_packaging_makefile_declares_version_targets(self):
        content = MAKEFILE_PATH.read_text(encoding="utf-8")

        self.assertIn("print-version:", content)
        self.assertIn("sync-version:", content)
        self.assertIn("set-release-version:", content)
        self.assertIn("set-dev-version:", content)

    def test_packaging_macos_arm64_build_script_resigns_copied_binary(self):
        content = BUILD_RUST_MACOS_ARM64_SCRIPT_PATH.read_text(encoding="utf-8")

        self.assertIn('cp "${RUST_DIR}/target/release/grafana-util" "${OUTPUT_DIR}/grafana-util"', content)
        self.assertIn('codesign --force --sign - "${OUTPUT_DIR}/grafana-util"', content)


if __name__ == "__main__":
    unittest.main()
