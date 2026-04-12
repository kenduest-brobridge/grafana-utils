import importlib.util
import json
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
    @classmethod
    def setUpClass(cls):
        cls.portal_contract = json.loads(
            (REPO_ROOT / "scripts" / "contracts" / "versioned-docs-portal.json").read_text(encoding="utf-8")
        )

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

    def test_build_version_links_includes_portal_latest_without_dev_by_default(self):
        module = load_module()

        links = module.build_version_links(["v0.7", "v0.6"])

        self.assertEqual(links[0].label, "Version portal")
        self.assertEqual(links[1].target_rel, "latest/index.html")
        self.assertEqual(links[2].target_rel, "v0.7/index.html")

    def test_build_version_links_can_include_dev_for_preview_validation(self):
        module = load_module()

        links = module.build_version_links(["v0.7"], include_dev=True)

        self.assertEqual(links[2].target_rel, "dev/index.html")
        self.assertEqual(links[3].target_rel, "v0.7/index.html")

    def test_render_version_portal_uses_landing_locale_switch_and_dual_language_copy(self):
        module = load_module()
        en = self.portal_contract["locales"]["en"]
        zh = self.portal_contract["locales"]["zh-TW"]

        rendered = module.render_version_portal(
            latest_lane="v0.7",
            version_lanes=["v0.7", "v0.6"],
            has_dev=True,
        )

        self.assertIn('id="locale-select"', rendered)
        self.assertIn('<option value="auto" selected>Auto</option>', rendered)
        self.assertIn('id="landing-i18n"', rendered)
        self.assertIn(en["hero_title"], rendered)
        self.assertIn(zh["hero_title"], rendered)
        self.assertIn(en["jump_prompt"], rendered)
        self.assertIn(zh["jump_prompt"], rendered)
        self.assertIn(en["lane_labels"]["latest_release"].format(latest_lane="v0.7"), rendered)
        self.assertIn(zh["lane_labels"]["dev_preview"], rendered)
        self.assertNotIn('value="v0.7/index.html"', rendered)
        self.assertIn('value="v0.6/index.html"', rendered)

    def test_render_version_portal_omits_dev_preview_when_not_included(self):
        module = load_module()

        rendered = module.render_version_portal(
            latest_lane="v0.7",
            version_lanes=["v0.7", "v0.6"],
            has_dev=False,
        )

        self.assertIn("latest/index.html", rendered)
        self.assertNotIn("v0.7/index.html", rendered)
        self.assertIn("v0.6/index.html", rendered)
        self.assertNotIn("dev/index.html", rendered)
        self.assertNotIn("Dev preview", rendered)
        self.assertNotIn("開發預覽", rendered)

    def test_render_version_portal_deep_links_outputs_by_lane_and_locale(self):
        module = load_module()

        rendered = module.render_version_portal(
            latest_lane="v0.7",
            version_lanes=["v0.7", "v0.6"],
            has_dev=True,
        )

        self.assertIn('href="latest/handbook/en/index.html"', rendered)
        self.assertIn('href="latest/commands/en/index.html"', rendered)
        self.assertIn('href="latest/man/index.html"', rendered)
        self.assertIn('href="dev/handbook/en/index.html"', rendered)
        self.assertIn('href="dev/commands/en/index.html"', rendered)
        self.assertIn('href="dev/man/index.html"', rendered)
        self.assertIn('latest/handbook/zh-TW/index.html', rendered)
        self.assertIn('latest/commands/zh-TW/index.html', rendered)
        self.assertIn('dev/handbook/zh-TW/index.html', rendered)
        self.assertNotIn('href="#outputs"', rendered)
        self.assertNotIn('Open a docs lane first', rendered)
        self.assertNotIn('先開啟任一版本線', rendered)

    def test_add_legacy_html_manpage_redirects_keeps_stale_lane_links_working(self):
        module = load_module()
        outputs = {
            "latest/man/grafana-util.html": "<html>manpage</html>",
            "latest/man/grafana-util-dashboard.html": "<html>dashboard manpage</html>",
        }

        module.add_legacy_html_manpage_redirects(outputs)

        self.assertIn("latest/html/man/grafana-util.html", outputs)
        self.assertIn("latest/html/man/grafana-util-dashboard.html", outputs)
        self.assertIn('url=../../man/grafana-util.html', outputs["latest/html/man/grafana-util.html"])
        self.assertIn('href="../../man/grafana-util.html"', outputs["latest/html/man/grafana-util.html"])
        self.assertEqual("<html>manpage</html>", outputs["latest/man/grafana-util.html"])


if __name__ == "__main__":
    unittest.main()
