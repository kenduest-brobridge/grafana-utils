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

unified_cli = importlib.import_module("grafana_utils.unified_cli")


class UnifiedDashboardCaptureCliTests(unittest.TestCase):
    def test_unified_cli_dashboard_capture_dashboard_group_help_mentions_new_commands(
        self,
    ):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit):
                unified_cli.parse_args(["dashboard"])

        help_text = stdout.getvalue()
        self.assertIn("inspect-vars", help_text)
        self.assertIn("screenshot", help_text)

    def test_unified_cli_dashboard_capture_parse_args_supports_dashboard_inspect_vars_namespace(
        self,
    ):
        args = unified_cli.parse_args(
            ["dashboard", "inspect-vars", "--dashboard-uid", "cpu-main"]
        )

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            ["inspect-vars", "--dashboard-uid", "cpu-main"],
        )

    def test_unified_cli_dashboard_capture_parse_args_supports_dashboard_screenshot_namespace(
        self,
    ):
        args = unified_cli.parse_args(
            [
                "dashboard",
                "screenshot",
                "--dashboard-uid",
                "cpu-main",
                "--output",
                "./captures/cpu-main.png",
                "--full-page-output",
                "tiles",
                "--header-title",
                "__auto__",
                "--header-captured-at",
            ]
        )

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            [
                "screenshot",
                "--dashboard-uid",
                "cpu-main",
                "--output",
                "./captures/cpu-main.png",
                "--full-page-output",
                "tiles",
                "--header-title",
                "__auto__",
                "--header-captured-at",
            ],
        )

    def test_unified_cli_dashboard_capture_parse_args_supports_dashboard_alias(self):
        args = unified_cli.parse_args(["db", "list", "--json"])

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(args.forwarded_argv, ["list-dashboard", "--json"])

    def test_unified_cli_dashboard_capture_parse_args_supports_sync_alias(self):
        args = unified_cli.parse_args(
            ["sy", "plan", "--desired-file", "./desired.json"]
        )

        self.assertEqual(args.entrypoint, "sync")
        self.assertEqual(
            args.forwarded_argv, ["plan", "--desired-file", "./desired.json"]
        )

    def test_unified_cli_dashboard_capture_main_dispatches_dashboard_screenshot_namespace(
        self,
    ):
        with mock.patch.object(
            unified_cli.dashboard_cli, "main", return_value=19
        ) as mocked:
            result = unified_cli.main(
                [
                    "dashboard",
                    "screenshot",
                    "--dashboard-uid",
                    "cpu-main",
                    "--output",
                    "./captures/cpu-main.png",
                    "--full-page-output",
                    "tiles",
                    "--header-title",
                    "__auto__",
                    "--header-captured-at",
                ]
            )

        self.assertEqual(result, 19)
        mocked.assert_called_once_with(
            [
                "screenshot",
                "--dashboard-uid",
                "cpu-main",
                "--output",
                "./captures/cpu-main.png",
                "--full-page-output",
                "tiles",
                "--header-title",
                "__auto__",
                "--header-captured-at",
            ]
        )


if __name__ == "__main__":
    unittest.main()
