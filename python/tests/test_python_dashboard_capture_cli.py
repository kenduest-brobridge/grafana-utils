import importlib
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_ROOT))

dashboard_cli = importlib.import_module("grafana_utils.dashboard_cli")


class DashboardCaptureCliTests(unittest.TestCase):
    def test_dashboard_capture_parse_args_supports_inspect_vars(self):
        args = dashboard_cli.parse_args(
            [
                "inspect-vars",
                "--dashboard-uid",
                "cpu-main",
                "--vars-query",
                "var-env=prod",
                "--output-format",
                "json",
            ]
        )

        self.assertEqual(args.command, "inspect-vars")
        self.assertEqual(args.dashboard_uid, "cpu-main")
        self.assertEqual(args.vars_query, "var-env=prod")
        self.assertEqual(args.output_format, "json")

    def test_dashboard_capture_parse_args_supports_screenshot(self):
        args = dashboard_cli.parse_args(
            [
                "screenshot",
                "--dashboard-url",
                "https://grafana.example.com/d/cpu-main/cpu-overview?var-env=prod",
                "--panel-id",
                "7",
                "--output",
                "./captures/cpu-main.png",
                "--device-scale-factor",
                "2",
                "--full-page",
                "--full-page-output",
                "manifest",
                "--print-capture-url",
                "--header-title",
                "__auto__",
                "--header-url",
                "__auto__",
                "--header-captured-at",
                "--header-text",
                "nightly review",
            ]
        )

        self.assertEqual(args.command, "screenshot")
        self.assertEqual(
            args.dashboard_url,
            "https://grafana.example.com/d/cpu-main/cpu-overview?var-env=prod",
        )
        self.assertEqual(args.panel_id, "7")
        self.assertEqual(args.output, "./captures/cpu-main.png")
        self.assertEqual(args.device_scale_factor, 2.0)
        self.assertTrue(args.full_page)
        self.assertEqual(args.full_page_output, "manifest")
        self.assertTrue(args.print_capture_url)
        self.assertEqual(args.header_title, "__auto__")
        self.assertEqual(args.header_url, "__auto__")
        self.assertTrue(args.header_captured_at)
        self.assertEqual(args.header_text, "nightly review")

    def test_dashboard_capture_inspect_vars_help_mentions_dashboard_inputs(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit):
                dashboard_cli.parse_args(["inspect-vars", "-h"])

        help_text = stdout.getvalue()
        self.assertIn("--dashboard-uid", help_text)
        self.assertIn("--dashboard-url", help_text)
        self.assertIn("--vars-query", help_text)
        self.assertIn("--output-format", help_text)

    def test_dashboard_capture_screenshot_help_mentions_capture_flags(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit):
                dashboard_cli.parse_args(["screenshot", "-h"])

        help_text = stdout.getvalue()
        self.assertIn("--dashboard-uid", help_text)
        self.assertIn("--dashboard-url", help_text)
        self.assertIn("--output", help_text)
        self.assertIn("--panel-id", help_text)
        self.assertIn("--var", help_text)
        self.assertIn("--device-scale-factor", help_text)
        self.assertIn("--full-page", help_text)
        self.assertIn("--full-page-output", help_text)
        self.assertIn("{single,tiles,manifest}", help_text)
        self.assertIn("--print-capture-url", help_text)
        self.assertIn("--header-title", help_text)
        self.assertIn("--header-url", help_text)
        self.assertIn("--header-captured-at", help_text)
        self.assertIn("--header-text", help_text)

    def test_dashboard_capture_main_dispatches_inspect_vars(self):
        with mock.patch.object(
            dashboard_cli, "inspect_vars", return_value=13
        ) as mocked:
            result = dashboard_cli.main(["inspect-vars", "--dashboard-uid", "cpu-main"])

        self.assertEqual(result, 13)
        mocked.assert_called_once()

    def test_dashboard_capture_main_dispatches_screenshot(self):
        with mock.patch.object(
            dashboard_cli, "screenshot_dashboard", return_value=17
        ) as mocked:
            result = dashboard_cli.main(
                [
                    "screenshot",
                    "--dashboard-uid",
                    "cpu-main",
                    "--output",
                    "./captures/cpu-main.png",
                ]
            )

        self.assertEqual(result, 17)
        mocked.assert_called_once()


if __name__ == "__main__":
    unittest.main()
