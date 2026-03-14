import ast
import importlib
import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "unified_cli.py"
MODULE_ENTRYPOINT_PATH = REPO_ROOT / "grafana_utils" / "__main__.py"
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
unified_cli = importlib.import_module("grafana_utils.unified_cli")


class UnifiedCliTests(unittest.TestCase):
    def test_unified_script_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_unified_module_entrypoint_parses_as_python39_syntax(self):
        source = MODULE_ENTRYPOINT_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_ENTRYPOINT_PATH), feature_version=(3, 9))

    def test_parse_args_without_command_prints_top_level_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args([])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("dashboard", help_text)
        self.assertIn("export", help_text)
        self.assertIn("alert", help_text)
        self.assertIn("access", help_text)
        self.assertIn("datasource", help_text)

    def test_parse_args_dashboard_without_subcommand_prints_dashboard_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["dashboard"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util dashboard", help_text)
        self.assertIn("list-data-sources", help_text)

    def test_parse_args_alert_without_subcommand_prints_alert_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["alert"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util alert", help_text)
        self.assertIn("export", help_text)
        self.assertIn("import", help_text)
        self.assertIn("diff", help_text)

    def test_parse_args_access_without_subcommand_prints_access_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["access"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util access", help_text)
        self.assertIn("{user,team,service-account}", help_text)

    def test_parse_args_datasource_without_subcommand_prints_datasource_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["datasource"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util datasource", help_text)
        self.assertIn("{list,export,import,diff,add,modify,delete}", help_text)

    def test_parse_args_supports_dashboard_passthrough(self):
        args = unified_cli.parse_args(["diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(args.forwarded_argv, ["diff", "--import-dir", "dashboards/raw"])

    def test_parse_args_supports_dashboard_namespace(self):
        args = unified_cli.parse_args(["dashboard", "export", "--export-dir", "dashboards"])

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            ["export-dashboard", "--export-dir", "dashboards"],
        )

    def test_parse_args_supports_dashboard_inspect_live_namespace(self):
        args = unified_cli.parse_args(
            ["dashboard", "inspect-live", "--url", "http://127.0.0.1:3000", "--report"]
        )

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            ["inspect-live", "--url", "http://127.0.0.1:3000", "--report"],
        )

    def test_parse_args_supports_alert_namespace(self):
        args = unified_cli.parse_args(["alert", "--url", "http://127.0.0.1:3000"])

        self.assertEqual(args.entrypoint, "alert")
        self.assertEqual(args.forwarded_argv, ["--url", "http://127.0.0.1:3000"])

    def test_parse_args_supports_alert_export_namespace(self):
        args = unified_cli.parse_args(
            ["alert", "export", "--output-dir", "./alerts", "--overwrite"]
        )

        self.assertEqual(args.entrypoint, "alert")
        self.assertEqual(
            args.forwarded_argv,
            ["export", "--output-dir", "./alerts", "--overwrite"],
        )

    def test_parse_args_supports_legacy_alert_alias(self):
        args = unified_cli.parse_args(["list-alert-rules", "--json"])

        self.assertEqual(args.entrypoint, "alert")
        self.assertEqual(args.forwarded_argv, ["list-rules", "--json"])

    def test_parse_args_supports_access_namespace(self):
        args = unified_cli.parse_args(
            ["access", "user", "list", "--url", "http://127.0.0.1:3000"]
        )

        self.assertEqual(args.entrypoint, "access")
        self.assertEqual(
            args.forwarded_argv,
            ["user", "list", "--url", "http://127.0.0.1:3000"],
        )

    def test_parse_args_supports_datasource_namespace(self):
        args = unified_cli.parse_args(
            ["datasource", "export", "--export-dir", "./datasources", "--overwrite"]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(
            args.forwarded_argv,
            ["export", "--export-dir", "./datasources", "--overwrite"],
        )

    def test_parse_args_supports_datasource_add_namespace(self):
        args = unified_cli.parse_args(
            ["datasource", "add", "--name", "Prometheus Main", "--type", "prometheus"]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(
            args.forwarded_argv,
            ["add", "--name", "Prometheus Main", "--type", "prometheus"],
        )

    def test_parse_args_supports_datasource_modify_namespace(self):
        args = unified_cli.parse_args(
            ["datasource", "modify", "--uid", "prom-main", "--set-url", "http://prometheus-v2:9090"]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(
            args.forwarded_argv,
            ["modify", "--uid", "prom-main", "--set-url", "http://prometheus-v2:9090"],
        )

    def test_parse_args_supports_datasource_diff_namespace(self):
        args = unified_cli.parse_args(
            ["datasource", "diff", "--diff-dir", "./datasources"]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(args.forwarded_argv, ["diff", "--diff-dir", "./datasources"])

    def test_parse_args_rejects_unknown_top_level_command(self):
        with self.assertRaises(SystemExit):
            unified_cli.parse_args(["unknown-command"])

    def test_main_dispatches_dashboard_passthrough(self):
        with mock.patch.object(unified_cli.dashboard_cli, "main", return_value=7) as mocked:
            result = unified_cli.main(["list-dashboard", "--json"])

        self.assertEqual(result, 7)
        mocked.assert_called_once_with(["list-dashboard", "--json"])

    def test_main_dispatches_alert_passthrough(self):
        with mock.patch.object(unified_cli.alert_cli, "main", return_value=3) as mocked:
            result = unified_cli.main(["alert", "--diff-dir", "./alerts/raw"])

        self.assertEqual(result, 3)
        mocked.assert_called_once_with(["--diff-dir", "./alerts/raw"])

    def test_main_dispatches_access_passthrough(self):
        with mock.patch.object(unified_cli.access_cli, "main", return_value=5) as mocked:
            result = unified_cli.main(["access", "team", "list", "--json"])

        self.assertEqual(result, 5)
        mocked.assert_called_once_with(["team", "list", "--json"])

    def test_main_dispatches_datasource_passthrough(self):
        with mock.patch.object(unified_cli.datasource_cli, "main", return_value=9) as mocked:
            result = unified_cli.main(["datasource", "list", "--json"])

        self.assertEqual(result, 9)
        mocked.assert_called_once_with(["list", "--json"])


if __name__ == "__main__":
    unittest.main()
