import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "generate_command_html.py"
SCRIPTS_DIR = REPO_ROOT / "scripts"


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_module():
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    spec = importlib.util.spec_from_file_location("generate_command_html", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules.setdefault("generate_command_html", module)
    spec.loader.exec_module(module)
    return module


def load_script_module(name: str):
    if str(SCRIPTS_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPTS_DIR))
    return __import__(name)


class GenerateCommandHtmlTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc_ui = load_json(REPO_ROOT / "scripts" / "contracts" / "doc-ui.json")
        cls.command_taxonomy = load_json(REPO_ROOT / "scripts" / "contracts" / "command-taxonomy.json")
        cls.command_index = load_json(REPO_ROOT / "scripts" / "contracts" / "command-reference-index.json")
        cls.handbook_nav = load_json(REPO_ROOT / "scripts" / "contracts" / "handbook-nav.json")
        cls.handbook_render = load_json(REPO_ROOT / "scripts" / "contracts" / "handbook-render.json")

    def locale_ui(self, locale: str) -> dict:
        return self.doc_ui["locales"][locale]

    def handbook_nav_titles(self, locale: str) -> dict:
        return self.handbook_nav["nav_titles"][locale]

    def handbook_group_labels(self, locale: str) -> list[str]:
        key = "label_zh_tw" if locale == "zh-TW" else "label_en"
        return [group[key] for group in self.handbook_nav["nav_groups"]]

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

    def test_generate_outputs_supports_versioned_lane(self):
        module = load_module()

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            output_prefix="v9.9",
            version="9.9.0",
            version_label="v9.9",
            version_links=(
                module.VersionLink("Version portal", "index.html"),
                module.VersionLink("Latest release", "latest/index.html"),
            ),
            raw_manpage_target_rel="v9.9/man/grafana-util.1",
            include_raw_manpages=True,
        )

        generated = module.generate_outputs(config)

        self.assertIn("v9.9/index.html", generated)
        self.assertIn("v9.9/man/index.html", generated)
        self.assertIn("v9.9/man/grafana-util.1", generated)
        self.assertIn("Current: v9.9", generated["v9.9/commands/en/dashboard.html"])
        self.assertIn("../index.html", generated["v9.9/commands/en/dashboard.html"])

    def test_render_landing_page_uses_structured_sections(self):
        module = load_module()

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
            version_label="v9.9",
            version_links=(module.VersionLink("Latest", "latest/index.html"),),
        )

        rendered = module.render_landing_page(config)

        self.assertIn('class="landing-hero"', rendered)
        self.assertIn('id="landing-hero-links"', rendered)
        self.assertNotIn('id="landing-search-form"', rendered)
        self.assertIn('id="locale-select"', rendered)
        self.assertIn('<option value="auto" selected>Auto</option>', rendered)
        self.assertIn('id="landing-i18n"', rendered)
        self.assertIn("First Run", rendered)
        self.assertIn("Read By Role", rendered)
        self.assertIn("Browse By Command Family", rendered)
        self.assertIn("Complete Reference", rendered)
        self.assertIn("Start with the handbook", rendered)
        self.assertIn("Open command reference", rendered)
        self.assertIn("第一次執行", rendered)
        self.assertIn("從手冊開始", rendered)
        self.assertIn("繁體中文", rendered)
        self.assertIn("Developer guide", rendered)
        self.assertIn("latest/index.html", rendered)
        self.assertIn(">Version<", rendered)
        self.assertIn("Version switching is secondary here.", rendered)

    def test_render_landing_page_labels_local_checkout_when_no_version_switch_exists(self):
        module = load_module()

        rendered = module.render_landing_page(module.HtmlBuildConfig(version="9.9.0"))

        self.assertIn(">Current version<", rendered)
        self.assertIn("This local docs build reflects the current checkout only.", rendered)
        self.assertNotIn("Version switching is secondary here.", rendered)

    def test_generate_outputs_includes_developer_page(self):
        module = load_module()

        generated = module.generate_outputs()

        self.assertIn("developer.html", generated)
        self.assertIn("Developer Guide", generated["developer.html"])
        self.assertIn("Source: docs/DEVELOPER.md", generated["developer.html"])
        self.assertIn("developer.html", generated["index.html"])

    def test_render_manpage_page_renders_structured_html(self):
        module = load_module()

        config = module.HtmlBuildConfig(version="9.9.0")
        roff = "\n".join(
            (
                '.TH TEST 1 "2026-04-04" "grafana-util 9.9.0" "User Commands"',
                ".SH NAME",
                r"grafana\-util\-test \- sample command",
                ".SH DESCRIPTION",
                r"Run \fBgrafana-util test\fR with readable HTML output.",
                r".IP \(bu 2",
                "First bullet",
                ".TP",
                ".B --flag",
                "Flag description",
                ".SH EXAMPLES",
                ".EX",
                "grafana-util test --flag",
                ".EE",
            )
        )

        rendered = module.render_manpage_page("man/test.html", "grafana-util-test.1", roff, config)

        self.assertIn('<div class="manpage-rendered">', rendered)
        self.assertIn("<h2>DESCRIPTION</h2>", rendered)
        self.assertIn("<strong>grafana-util test</strong>", rendered)
        self.assertIn('<ul class="man-bullets">', rendered)
        self.assertIn('<dl class="man-definitions">', rendered)
        self.assertIn('<pre class="man-example"><code>grafana-util test --flag</code></pre>', rendered)
        self.assertNotIn('<pre class="manpage">', rendered)

    def test_strip_leading_h1_removes_duplicate_document_title(self):
        module = load_module()

        stripped = module.strip_leading_h1('<h1 id="title">Title</h1>\n<p>Body</p>')

        self.assertEqual(stripped, "<p>Body</p>")

    def test_split_display_title_separates_english_subtitle_for_mixed_heading(self):
        module = load_module()

        main, secondary = module.split_display_title("📖 維運導引手冊 (Operator Handbook)")

        self.assertEqual(main, "維運導引手冊")
        self.assertEqual(secondary, "Operator Handbook")

    def test_strip_heading_decorations_removes_leading_emoji_from_heading_html(self):
        module = load_module()

        rendered = module.strip_heading_decorations('<h2 id="quick-start">⚡ 30 秒快速上手 (Quick Start)</h2><p>Body</p>')

        self.assertIn(">30 秒快速上手 (Quick Start)<", rendered)
        self.assertNotIn("⚡", rendered)

    def test_strip_heading_decorations_preserves_inline_code_in_heading_html(self):
        module = load_module()

        rendered = module.strip_heading_decorations('<h3 id="status-live">1. <code>status live</code> 入口</h3>')

        self.assertIn("<code>status live</code>", rendered)
        self.assertNotIn("&lt;code&gt;status live&lt;/code&gt;", rendered)

    def test_strip_heading_decorations_preserves_other_inline_markup_in_heading_html(self):
        module = load_module()

        rendered = module.strip_heading_decorations('<h3 id="jq">1. Filtering with <em>jq</em></h3>')

        self.assertIn("<em>jq</em>", rendered)
        self.assertNotIn("&lt;em&gt;jq&lt;/em&gt;", rendered)

    def test_render_toc_adds_hierarchy_classes_and_strips_emoji_labels(self):
        module = load_module()

        headings = (
            module.RenderedHeading(level=2, text="⚡ 30 秒快速上手 (Quick Start)", anchor="quick-start"),
            module.RenderedHeading(level=3, text="安裝 (全域 Binary)", anchor="install"),
        )

        rendered = module.render_toc(headings)

        self.assertIn('class="toc-level-2"', rendered)
        self.assertIn('class="toc-level-3"', rendered)
        self.assertIn('30 秒快速上手 (Quick Start)', rendered)
        self.assertIn('href="#install"', rendered)
        self.assertNotIn("⚡", rendered)

    def test_render_section_index_supports_list_variant_and_min_entries(self):
        module = load_module()

        headings = (
            module.RenderedHeading(level=2, text="適用對象", anchor="audience"),
            module.RenderedHeading(level=2, text="主要目標", anchor="goals"),
            module.RenderedHeading(level=2, text="採用前後對照", anchor="before-after"),
            module.RenderedHeading(level=2, text="成功判準", anchor="success"),
        )

        rendered = module.render_section_index(headings, title="章節索引", variant="list", min_entries=4)

        self.assertIn('class="section-index-list layout-list"', rendered)
        self.assertIn('href="#audience"', rendered)
        self.assertIn(">主要目標<", rendered)

        suppressed = module.render_section_index(headings[:3], title="章節索引", variant="list", min_entries=4)
        self.assertEqual("", suppressed)

    def test_theme_script_copies_full_code_blocks_without_filtering_comments(self):
        module = load_module()

        self.assertIn("navigator.clipboard.writeText(raw)", module.THEME_SCRIPT)
        self.assertNotIn("filter(l => l.trim() && !l.trim().startsWith(\"#\"))", module.THEME_SCRIPT)

    def test_page_style_keeps_unwrapped_code_blocks_inside_content_column(self):
        module = load_module()

        self.assertIn("pre { position: relative; max-width: 100%; box-sizing: border-box;", module.PAGE_STYLE)
        self.assertIn(".article { min-width: 0;", module.PAGE_STYLE)
        self.assertIn(".sidebar { position: sticky; top: 12px; min-width: 0;", module.PAGE_STYLE)

    def test_page_style_uses_fixed_topbar_control_widths(self):
        module = load_module()

        self.assertIn("#locale-select { width: 148px; }", module.PAGE_STYLE)
        self.assertIn("#page-locale-select { width: 172px; }", module.PAGE_STYLE)
        self.assertIn("#jump-select { width: 300px; }", module.PAGE_STYLE)

    def test_render_global_nav_uses_localized_handbook_titles(self):
        module = load_module()

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_global_nav("handbook/zh-TW/index.html", "zh-TW", config)

        for label in self.handbook_group_labels("zh-TW"):
            if label == "第 4 部 · 設計原則":
                continue
            self.assertIn(label, rendered)
        nav_titles = self.handbook_nav_titles("zh-TW")
        self.assertIn(nav_titles["getting-started"], rendered)
        self.assertIn(nav_titles["role-new-user"], rendered)
        self.assertIn(">Dashboard<", rendered)
        self.assertIn('class="nav-group-header"', rendered)
        self.assertIn('class="nav-group-title-text"', rendered)
        self.assertNotIn(">Getting Started<", rendered)
        self.assertNotIn(">Role New User<", rendered)

    def test_render_global_nav_renders_single_command_reference_entry(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_global_nav("commands/zh-TW/datasource.html", "zh-TW", config)

        self.assertIn(f'>{ui_text["open_command_reference_label"]}<', rendered)
        self.assertIn(f'>{ui_text["command_reference_label"]}<', rendered)
        self.assertIn('class="nav-command-entry active"', rendered)
        self.assertEqual(rendered.count('class="nav-command-entry active"'), 1)
        self.assertIn(">Datasource<", rendered)
        self.assertIn(">Dashboard<", rendered)
        self.assertNotIn("Grafana-util-datasource", rendered)

    def test_render_command_map_nav_compacts_dashboard_commands_without_cli_prefix(self):
        module = load_module()

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_map_nav("handbook/zh-TW/dashboard.html", "zh-TW", "dashboard", config)

        self.assertIn(">dashboard<", rendered)
        self.assertIn(">browse<", rendered)
        self.assertIn(">list<", rendered)
        self.assertNotIn(">grafana-util dashboard browse<", rendered)
        self.assertNotIn("grafana-util", rendered)

    def test_render_command_map_nav_uses_alias_map_for_architecture_page(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_map_nav("handbook/zh-TW/architecture.html", "zh-TW", "architecture", config)

        self.assertIn(ui_text["command_relationships_title"], rendered)
        self.assertIn("唯讀檢查", rendered)
        self.assertIn(">status<", rendered)
        self.assertIn(">resource<", rendered)
        self.assertIn(">snapshot<", rendered)

    def test_render_command_page_adds_parent_breadcrumb_and_related_links_for_profile(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_page(
            "zh-TW",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "profile.md",
            "commands/zh-TW/profile.html",
            config,
        )

        self.assertIn('href="config.html">config</a> / grafana-util config profile', rendered)
        self.assertIn(f'{ui_text["parent_command_label"]}: config', rendered)
        self.assertIn("對應手冊章節", rendered)

    def test_render_command_page_adds_child_reference_links_for_config_root(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_page(
            "zh-TW",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "config.md",
            "commands/zh-TW/config.html",
            config,
        )

        self.assertIn(f'{ui_text["child_command_label"]}: config profile', rendered)
        self.assertIn('href="profile.html"', rendered)

    def test_render_command_page_links_command_reference_breadcrumb(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_page(
            "zh-TW",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "profile.md",
            "commands/zh-TW/profile.html",
            config,
        )

        self.assertIn(f'href="index.html">{ui_text["command_reference_label"]}</a>', rendered)

    def test_command_taxonomy_contract_drives_audience_and_handbook_mapping(self):
        module = load_module()
        expected_handbook = self.command_taxonomy["handbook_context_by_page"]["profile"]
        expected_hint = self.command_taxonomy["audience_hints_by_root"]["config"]["zh-TW"]

        self.assertEqual(
            expected_handbook,
            module.command_handbook_stem(REPO_ROOT / "docs" / "commands" / "zh-TW" / "profile.md"),
        )
        self.assertIn(
            expected_hint,
            module.command_audience_hint("zh-TW", REPO_ROOT / "docs" / "commands" / "zh-TW" / "config.md"),
        )

    def test_render_command_reference_index_uses_structured_contract_content(self):
        module = load_module()
        command_index = self.command_index["locales"]["zh-TW"]
        section_titles = [section["title"] for section in command_index["sections"]]

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_page(
            "zh-TW",
            REPO_ROOT / "docs" / "commands" / "zh-TW" / "index.md",
            "commands/zh-TW/index.html",
            config,
        )

        for title in section_titles[:3]:
            self.assertIn(title, rendered)
        self.assertIn('href="dashboard.html"', rendered)

    def test_render_command_reference_index_uses_removed_roots_section_from_contract(self):
        module = load_module()
        command_index = self.command_index["locales"]["en"]
        removed_roots_title = next(section["title"] for section in command_index["sections"] if section["title"] == "Removed roots")
        selector_title = next(section["title"] for section in command_index["sections"] if section["title"] == "Which command should I use?")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_page(
            "en",
            REPO_ROOT / "docs" / "commands" / "en" / "index.md",
            "commands/en/index.html",
            config,
        )

        self.assertIn(removed_roots_title, rendered)
        self.assertIn("grafana-util config profile", rendered)
        self.assertIn(selector_title, rendered)

    def test_render_handbook_page_includes_topbar_language_switch(self):
        module = load_module()
        ui_text = self.locale_ui("zh-TW")
        start_label = self.handbook_group_labels("zh-TW")[0]

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        page = next(p for p in module.build_handbook_pages("zh-TW", handbook_root=config.handbook_root) if p.stem == "index")

        rendered = module.render_handbook_page(page, config)

        self.assertIn('id="page-locale-select"', rendered)
        self.assertIn(f'{ui_text["page_locale_current_prefix"]}繁體中文', rendered)
        self.assertIn(f'{ui_text["page_locale_switch_prefix"]}English', rendered)
        self.assertNotIn('id="locale-select"', rendered)
        self.assertIn('class="sidebar-toggle sidebar-toggle-left"', rendered)
        self.assertIn('class="sidebar-toggle sidebar-toggle-right"', rendered)
        self.assertIn(start_label, rendered)
        self.assertIn("第 1 章 / 共", rendered)

    def test_render_handbook_page_uses_continuous_reading_footer_labels(self):
        module = load_module()
        locale_strings = self.handbook_render["locale_strings"]["zh-TW"]

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        page = next(p for p in module.build_handbook_pages("zh-TW", handbook_root=config.handbook_root) if p.stem == "dashboard")

        rendered = module.render_handbook_page(page, config)

        self.assertIn(f'>{locale_strings["previous_chapter"]}<', rendered)
        self.assertIn(f'>{locale_strings["next_chapter"]}<', rendered)
        self.assertIn("第 7 章", rendered)
        self.assertIn("第 9 章", rendered)

    def test_render_handbook_page_uses_short_nav_title_with_full_title_subtitle(self):
        module = load_module()
        role_paths_label = self.handbook_group_labels("zh-TW")[1]
        nav_title = self.handbook_nav_titles("zh-TW")["role-automation-ci"]

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        page = next(p for p in module.build_handbook_pages("zh-TW", handbook_root=config.handbook_root) if p.stem == "role-automation-ci")

        rendered = module.render_handbook_page(page, config)

        self.assertIn("/ 手冊 /", rendered)
        self.assertIn(f'>{nav_title}<', rendered)
        self.assertIn('class="hero-subtitle">自動化 / CI 角色導讀</p>', rendered)
        self.assertIn(role_paths_label, rendered)

    def test_render_handbook_page_includes_section_index_for_long_chapters(self):
        module = load_module()
        locale_strings = self.handbook_render["locale_strings"]["zh-TW"]

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        page = next(p for p in module.build_handbook_pages("zh-TW", handbook_root=config.handbook_root) if p.stem == "architecture")

        rendered = module.render_handbook_page(page, config)

        self.assertIn('class="section-index"', rendered)
        self.assertIn(f'>{locale_strings["section_index_title"]}<', rendered)
        self.assertIn(locale_strings["section_index_summary"], rendered)
        self.assertIn('class="section-index-list layout-list"', rendered)
        self.assertIn('href="#command-relationships"', rendered)
        self.assertIn('href="#', rendered)

    def test_render_command_map_nav_renders_as_handbook_section(self):
        module = load_module()

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )

        rendered = module.render_command_map_nav("handbook/zh-TW/architecture.html", "zh-TW", "architecture", config, compact=False)

        self.assertIn('class="handbook-command-map"', rendered)
        self.assertIn('id="command-relationships"', rendered)
        self.assertIn('class="handbook-command-map-panel"', rendered)
        self.assertNotIn('<section class="nav-section"><h2>', rendered)

    def test_render_command_page_includes_intro_panel_from_command_doc_metadata(self):
        module = load_module()
        labels = load_script_module("docsite_html_command_pages").command_intro_labels("en")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        source = REPO_ROOT / "docs" / "commands" / "en" / "dashboard-screenshot.md"

        rendered = module.render_command_page("en", source, "commands/en/dashboard-screenshot.html", config)

        self.assertIn(labels["what"], rendered)
        self.assertIn(labels["when"], rendered)
        self.assertIn(labels["who"], rendered)
        self.assertIn("Open one dashboard in a headless browser and capture image or PDF output.", rendered)

    def test_render_command_page_includes_intro_panel_for_zh_tw_command_docs(self):
        module = load_module()
        labels = load_script_module("docsite_html_command_pages").command_intro_labels("zh-TW")

        config = module.HtmlBuildConfig(
            source_root=REPO_ROOT,
            command_docs_root=REPO_ROOT / "docs" / "commands",
            handbook_root=REPO_ROOT / "docs" / "user-guide",
            version="9.9.0",
        )
        source = REPO_ROOT / "docs" / "commands" / "zh-TW" / "dashboard-screenshot.md"

        rendered = module.render_command_page("zh-TW", source, "commands/zh-TW/dashboard-screenshot.html", config)

        self.assertIn(labels["what"], rendered)
        self.assertIn(labels["when"], rendered)
        self.assertIn(labels["who"], rendered)

    def test_render_developer_page_collapses_empty_nav_and_sidebar_sections(self):
        module = load_module()

        rendered = module.render_developer_page(module.HtmlBuildConfig())

        self.assertIn('class="layout layout-no-nav"', rendered)
        self.assertIn(">Guide Map<", rendered)
        self.assertIn("Jump straight to the repo concern", rendered)
        self.assertNotIn("<h2>Related</h2>", rendered)
        self.assertNotIn("<h2>Version</h2>", rendered)
        self.assertNotIn("<h2>Language</h2>", rendered)
        self.assertNotIn('class="sidebar-toggle sidebar-toggle-left"', rendered)
        self.assertIn('class="sidebar-toggle sidebar-toggle-right"', rendered)

    def test_render_landing_page_does_not_include_sidebar_toggles(self):
        module = load_module()

        rendered = module.render_landing_page(module.HtmlBuildConfig())

        self.assertNotIn('class="sidebar-toggle sidebar-toggle-left"', rendered)
        self.assertNotIn('class="sidebar-toggle sidebar-toggle-right"', rendered)

    def test_versioned_handbook_build_skips_missing_newer_chapters(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as tmp_dir:
            root = Path(tmp_dir)
            for locale in ("en", "zh-TW"):
                source_dir = REPO_ROOT / "docs" / "user-guide" / locale
                target_dir = root / locale
                target_dir.mkdir(parents=True, exist_ok=True)
                for source in source_dir.glob("*.md"):
                    if source.name == "what-is-grafana-util.md":
                        continue
                    target_dir.joinpath(source.name).write_text(source.read_text(encoding="utf-8"), encoding="utf-8")

            pages = module.build_handbook_pages("en", handbook_root=root)
            stems = [page.stem for page in pages]

            self.assertNotIn("what-is-grafana-util", stems)
            self.assertIn("getting-started", stems)

            config = module.HtmlBuildConfig(
                source_root=REPO_ROOT,
                command_docs_root=REPO_ROOT / "docs" / "commands",
                handbook_root=root,
                version="0.7.3",
                output_prefix="v0.7",
                version_label="v0.7",
            )
            generated = module.generate_outputs(config)

            self.assertIn("v0.7/index.html", generated)
            self.assertIn("v0.7/handbook/en/getting-started.html", generated)
            self.assertNotIn("v0.7/handbook/en/what-is-grafana-util.html", generated)
            self.assertNotIn("what-is-grafana-util.html", generated["v0.7/index.html"])


if __name__ == "__main__":
    unittest.main()
