import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "check_docs_surface.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("check_docs_surface", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("check_docs_surface", module)
    spec.loader.exec_module(module)
    return module


class CheckDocsSurfaceTests(unittest.TestCase):
    def test_validate_command_doc_locale_parity_accepts_mirrored_pages(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            for locale in ("en", "zh-TW"):
                locale_root = root / locale
                locale_root.mkdir(parents=True)
                (locale_root / "index.md").write_text("# Command Reference\n\n- [alert](./alert.md)\n", encoding="utf-8")
                (locale_root / "alert.md").write_text(
                    "\n".join(
                        (
                            "# `grafana-util alert`",
                            "",
                            "See [related](./index.md)",
                            "",
                            "```bash",
                            "grafana-util alert preview-route --desired-dir ./alerts/desired",
                            "```",
                        )
                    )
                    + "\n",
                    encoding="utf-8",
                )

            surface = {"command_doc_locales": ["en", "zh-TW"], "doc_pages": {}}
            with patch.object(module, "COMMAND_DOC_ROOT", root), patch.object(
                module, "resolve_cli_path", side_effect=lambda tokens: tuple(tokens)
            ):
                findings = module.validate_command_doc_locale_parity(surface)

        self.assertEqual(findings, [])

    def test_validate_command_doc_locale_parity_accepts_plain_heading_style(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            en_root = root / "en"
            zh_root = root / "zh-TW"
            en_root.mkdir(parents=True)
            zh_root.mkdir(parents=True)
            (en_root / "index.md").write_text("# Command Reference\n", encoding="utf-8")
            (zh_root / "index.md").write_text("# 指令參考\n", encoding="utf-8")
            (en_root / "alert.md").write_text("# alert\n\n```bash\ngrafana-util alert preview-route\n```\n", encoding="utf-8")
            (zh_root / "alert.md").write_text(
                "# `grafana-util alert`\n\n```bash\ngrafana-util alert preview-route\n```\n",
                encoding="utf-8",
            )

            surface = {"command_doc_locales": ["en", "zh-TW"], "doc_pages": {}}
            with patch.object(module, "COMMAND_DOC_ROOT", root), patch.object(
                module, "resolve_cli_path", side_effect=lambda tokens: tuple(tokens)
            ):
                findings = module.validate_command_doc_locale_parity(surface)

        self.assertEqual(findings, [])

    def test_validate_command_doc_locale_parity_flags_heading_drift(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            en_root = root / "en"
            zh_root = root / "zh-TW"
            en_root.mkdir(parents=True)
            zh_root.mkdir(parents=True)
            (en_root / "index.md").write_text("# Command Reference\n", encoding="utf-8")
            (zh_root / "index.md").write_text("# 指令參考\n", encoding="utf-8")
            (en_root / "alert.md").write_text(
                "# `grafana-util alert`\n\n```bash\ngrafana-util alert preview-route\n```\n",
                encoding="utf-8",
            )
            (zh_root / "alert.md").write_text(
                "# `grafana-util alert preview-route`\n\n```bash\ngrafana-util alert preview-route\n```\n",
                encoding="utf-8",
            )

            surface = {"command_doc_locales": ["en", "zh-TW"], "doc_pages": {}}
            with patch.object(module, "COMMAND_DOC_ROOT", root), patch.object(
                module, "resolve_cli_path", side_effect=lambda tokens: tuple(tokens)
            ):
                findings = module.validate_command_doc_locale_parity(surface)

        self.assertTrue(
            any("heading differs across locales" in finding.message for finding in findings)
        )

    def test_validate_command_doc_locale_parity_flags_example_outside_page_surface(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            en_root = root / "en"
            zh_root = root / "zh-TW"
            en_root.mkdir(parents=True)
            zh_root.mkdir(parents=True)
            (en_root / "index.md").write_text("# Command Reference\n", encoding="utf-8")
            (zh_root / "index.md").write_text("# 指令參考\n", encoding="utf-8")
            (en_root / "alert.md").write_text(
                "# `grafana-util alert`\n\n```bash\ngrafana-util dashboard browse\n```\n",
                encoding="utf-8",
            )
            (zh_root / "alert.md").write_text(
                "# `grafana-util alert`\n\n```bash\ngrafana-util alert preview-route\n```\n",
                encoding="utf-8",
            )

            surface = {"command_doc_locales": ["en", "zh-TW"], "doc_pages": {}}
            with patch.object(module, "COMMAND_DOC_ROOT", root), patch.object(
                module, "resolve_cli_path", side_effect=lambda tokens: tuple(tokens)
            ):
                findings = module.validate_command_doc_locale_parity(surface)

        self.assertTrue(any("escapes page command surface" in finding.message for finding in findings))

    def test_validate_links_accepts_landing_manpage_html_mirror(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            landing = root / "docs" / "landing"
            man_html = root / "docs" / "html" / "man"
            landing.mkdir(parents=True)
            man_html.mkdir(parents=True)
            source = landing / "en.md"
            source.write_text("[grafana-util(1)](../man/grafana-util.html)\n", encoding="utf-8")
            (man_html / "grafana-util.html").write_text("<html></html>\n", encoding="utf-8")

            with patch.object(module, "REPO_ROOT", root):
                findings = module.validate_links(source, source.read_text(encoding="utf-8"))

        self.assertEqual(findings, [])


if __name__ == "__main__":
    unittest.main()
