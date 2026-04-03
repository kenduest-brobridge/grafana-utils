import importlib.util
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "generate_command_html.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("generate_command_html", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("generate_command_html", module)
    spec.loader.exec_module(module)
    return module


class GenerateCommandHtmlTests(unittest.TestCase):
    def test_generated_html_matches_checked_in_outputs(self):
        module = load_module()

        generated = module.generate_outputs()
        html_root = REPO_ROOT / "docs" / "html"
        checked_in = {
            path.relative_to(html_root).as_posix(): path.read_text(encoding="utf-8")
            for path in sorted(html_root.rglob("*.html"))
        }
        nojekyll_path = html_root / ".nojekyll"
        if nojekyll_path.exists():
            checked_in[".nojekyll"] = nojekyll_path.read_text(encoding="utf-8")

        self.assertEqual(set(generated), set(checked_in))
        self.assertEqual(generated, checked_in)


if __name__ == "__main__":
    unittest.main()
