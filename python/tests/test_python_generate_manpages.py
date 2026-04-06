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
    def test_subcommand_manpages_are_generated(self):
        module = load_module()

        generated = module.generate_manpages()

        self.assertIn("grafana-util-dashboard-screenshot.1", generated)
        self.assertIn("grafana-util-access-service-account-token.1", generated)
        self.assertIn("grafana-util-profile-add.1", generated)

    def test_subcommand_manpage_contains_command_sections(self):
        module = load_module()

        generated = module.generate_manpages()
        screenshot_manpage = generated["grafana-util-dashboard-screenshot.1"]

        self.assertIn(".SH SYNOPSIS", screenshot_manpage)
        self.assertIn(".B grafana-util dashboard screenshot [\\fIOPTIONS\\fR]", screenshot_manpage)
        self.assertIn(".SH DESCRIPTION", screenshot_manpage)
        self.assertIn("Open one dashboard in a headless browser and capture image or PDF output.", screenshot_manpage)
        self.assertIn(".SH EXAMPLES", screenshot_manpage)

    def test_subcommand_manpage_projects_before_after_success_and_failure_sections(self):
        module = load_module()

        generated = module.generate_manpages()
        topology_manpage = generated["grafana-util-dashboard-topology.1"]

        self.assertIn(".SH BEFORE / AFTER", topology_manpage)
        self.assertIn("one topology run gives you a reproducible graph", topology_manpage)
        self.assertIn(".SH SUCCESS CRITERIA", topology_manpage)
        self.assertIn("the same topology can be reviewed in the terminal", topology_manpage)
        self.assertIn(".SH FAILURE CHECKS", topology_manpage)
        self.assertIn("if the graph looks empty or too small", topology_manpage)

    def test_namespace_manpage_subcommands_include_use_case_summary(self):
        module = load_module()

        generated = module.generate_manpages()
        access_manpage = generated["grafana-util-access.1"]

        self.assertIn(
            "List, browse, create, modify, export, import, diff, or delete Grafana users. Use when:",
            access_manpage,
        )
        self.assertIn(
            "List, create, export, import, diff, or delete Grafana service accounts, and manage their tokens. Use when:",
            access_manpage,
        )

    def test_namespace_manpage_projects_root_workflow_evidence_sections(self):
        module = load_module()

        generated = module.generate_manpages()
        dashboard_manpage = generated["grafana-util-dashboard.1"]

        self.assertIn(".SH WORKFLOW LANES", dashboard_manpage)
        self.assertIn(".SH WORKFLOW LANES", dashboard_manpage)
        self.assertIn("Review Before Mutate: review, governance\\-gate, and impact analysis.", dashboard_manpage)
        self.assertIn("Choose this page when the task is dashboard work", dashboard_manpage)
        self.assertIn(".SH BEFORE / AFTER", dashboard_manpage)
        self.assertIn(".SH SUCCESS CRITERIA", dashboard_manpage)
        self.assertIn(".SH FAILURE CHECKS", dashboard_manpage)

    def test_top_level_manpage_commands_include_use_case_summary(self):
        module = load_module()

        generated = module.generate_manpages()
        top_level_manpage = generated["grafana-util.1"]

        self.assertIn(
            ".B access\nRun the access\\-management command surface for users, orgs, teams, and service accounts. Use when:",
            top_level_manpage,
        )

    def test_top_level_manpage_lists_subcommand_manpages(self):
        module = load_module()

        generated = module.generate_manpages()
        top_level_manpage = generated["grafana-util.1"]

        self.assertIn(".SH SUBCOMMAND MANPAGES", top_level_manpage)
        self.assertIn(".SS dashboard", top_level_manpage)
        self.assertIn(".B grafana\\-util\\-dashboard\\-screenshot(1)", top_level_manpage)
        self.assertIn(".SS access", top_level_manpage)
        self.assertIn(".B grafana\\-util\\-access\\-service\\-account\\-token(1)", top_level_manpage)

    def test_top_level_manpage_points_sync_workflows_to_change_family(self):
        module = load_module()

        generated = module.generate_manpages()
        top_level_manpage = generated["grafana-util.1"]

        self.assertIn(".B change", top_level_manpage)
        self.assertIn("Declarative sync planning and gated apply workflows.", top_level_manpage)
        self.assertIn(
            "public CLI surface and generated manpages live under grafana-util change",
            top_level_manpage,
        )
        self.assertIn("grafana-util-change*(1) pages.", top_level_manpage)
        self.assertNotIn("does not yet carry a generated sync namespace manpage", top_level_manpage)

    def test_namespace_manpage_examples_include_caption_lines(self):
        module = load_module()

        generated = module.generate_manpages()
        access_manpage = generated["grafana-util-access.1"]

        self.assertIn(
            ".PP\nuser: List, browse, create, modify, export, import, diff, or delete Grafana users.",
            access_manpage,
        )
        self.assertIn(
            ".PP\nservice\\-account token: Add or delete tokens for a Grafana service account.",
            access_manpage,
        )

    def test_inline_evidence_headings_do_not_generate_bogus_subcommand_manpages(self):
        module = load_module()

        generated = module.generate_manpages()

        self.assertNotIn("grafana-util-profile-Before-/.1", generated)
        self.assertNotIn("grafana-util-profile-What-success-looks-like.1", generated)
        self.assertNotIn("grafana-util-profile-Failure-checks.1", generated)
        self.assertNotIn("grafana-util-snapshot-Before-/.1", generated)
        self.assertNotIn("grafana-util-snapshot-What-success-looks-like.1", generated)
        self.assertNotIn("grafana-util-snapshot-Failure-checks.1", generated)

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
