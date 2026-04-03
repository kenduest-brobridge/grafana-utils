import importlib.util
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "generate_manpages.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("generate_manpages", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("generate_manpages", module)
    spec.loader.exec_module(module)
    return module


class GenerateManpagesTests(unittest.TestCase):
    def test_generated_manpages_match_checked_in_outputs(self):
        module = load_module()

        generated = module.generate_manpages()
        checked_in = {
            path.name: path.read_text(encoding="utf-8")
            for path in sorted((REPO_ROOT / "docs" / "man").glob("*.1"))
        }

        self.assertEqual(set(generated), set(checked_in))
        self.assertEqual(generated, checked_in)


if __name__ == "__main__":
    unittest.main()
