import importlib.util
import sys
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "build_pages_site.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("build_pages_site", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("build_pages_site", module)
    spec.loader.exec_module(module)
    return module


class BuildPagesSiteTests(unittest.TestCase):
    def test_parse_semver_tag_accepts_release_tags_only(self):
        module = load_module()

        parsed = module.parse_semver_tag("v1.2.3")

        self.assertIsNotNone(parsed)
        self.assertEqual(parsed.minor_label, "v1.2")
        self.assertIsNone(module.parse_semver_tag("v1.2"))
        self.assertIsNone(module.parse_semver_tag("v1.2.3-rc1"))

    def test_select_latest_tags_per_minor_keeps_highest_patch(self):
        module = load_module()

        selected = module.select_latest_tags_per_minor(
            [
                module.SemverTag(0, 7, 1, "v0.7.1"),
                module.SemverTag(0, 7, 4, "v0.7.4"),
                module.SemverTag(0, 6, 9, "v0.6.9"),
                module.SemverTag(0, 6, 2, "v0.6.2"),
            ]
        )

        self.assertEqual([tag.raw for tag in selected], ["v0.7.4", "v0.6.9"])

    def test_build_version_links_includes_portal_latest_and_dev(self):
        module = load_module()

        links = module.build_version_links(["v0.7", "v0.6"])

        self.assertEqual(links[0].label, "Version portal")
        self.assertEqual(links[1].target_rel, "latest/index.html")
        self.assertEqual(links[2].target_rel, "dev/index.html")
        self.assertEqual(links[3].target_rel, "v0.7/index.html")


if __name__ == "__main__":
    unittest.main()
