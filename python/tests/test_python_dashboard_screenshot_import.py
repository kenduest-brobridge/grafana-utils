import builtins
import importlib
import sys
import unittest
from pathlib import Path
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
if str(PYTHON_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))


class DashboardScreenshotImportTests(unittest.TestCase):
    def test_screenshot_module_import_does_not_require_pillow(self):
        module_name = "grafana_utils.dashboards.screenshot"
        original_import = builtins.__import__

        def guarded_import(name, globals=None, locals=None, fromlist=(), level=0):
            if name == "PIL" or name.startswith("PIL."):
                raise ImportError("blocked PIL import for test")
            return original_import(name, globals, locals, fromlist, level)

        previous_module = sys.modules.pop(module_name, None)
        try:
            with mock.patch("builtins.__import__", side_effect=guarded_import):
                module = importlib.import_module(module_name)
        finally:
            sys.modules.pop(module_name, None)
            if previous_module is not None:
                sys.modules[module_name] = previous_module

        self.assertTrue(hasattr(module, "_pil_modules"))
