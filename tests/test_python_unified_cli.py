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
        self.assertIn("Commands:", help_text)
        self.assertIn("dashboard (db):", help_text)
        self.assertIn("datasource (ds):", help_text)
        self.assertIn("alert (al):", help_text)
        self.assertIn("access (ac):", help_text)
        self.assertIn("sync (sy):", help_text)
        self.assertIn("dashboard", help_text)
        self.assertIn("export", help_text)
        self.assertIn("alert", help_text)
        self.assertIn("access", help_text)
        self.assertIn("datasource", help_text)
        self.assertIn("sync", help_text)
        self.assertIn("grafana-util dashboard", help_text)
        self.assertNotIn("Compatibility", help_text)
        self.assertNotIn("export-alert", help_text)
        self.assertIn("--basic-user admin --basic-password admin --all-orgs", help_text)
        self.assertIn("grafana-util dashboard inspect-export", help_text)
        self.assertIn("grafana-util access org list", help_text)
        self.assertIn("grafana-util access team list", help_text)
        self.assertNotIn("grafana-access-utils", help_text)

    def test_parse_args_dashboard_without_subcommand_prints_dashboard_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["dashboard"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util dashboard", help_text)
        self.assertNotIn("list-data-sources", help_text)
        self.assertIn("Examples:", help_text)
        self.assertIn(
            "grafana-util dashboard list --url http://localhost:3000 --table", help_text
        )

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
        self.assertIn("{user,team,org,service-account}", help_text)

    def test_parse_args_datasource_without_subcommand_prints_datasource_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["datasource"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util datasource", help_text)
        self.assertIn("{list,export,import,diff,add,modify,delete}", help_text)

    def test_parse_args_sync_without_subcommand_prints_sync_help(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            with self.assertRaises(SystemExit) as exc:
                unified_cli.parse_args(["sync"])

        self.assertEqual(exc.exception.code, 0)
        help_text = stdout.getvalue()
        self.assertIn("grafana-util sync", help_text)
        self.assertIn(
            "{plan,review,preflight,assess-alerts,bundle-preflight,apply}", help_text
        )

    def test_parse_args_rejects_top_level_legacy_dashboard_alias(self):
        with self.assertRaises(SystemExit):
            unified_cli.parse_args(["diff", "--import-dir", "dashboards/raw"])

    def test_parse_args_supports_dashboard_namespace(self):
        args = unified_cli.parse_args(
            ["dashboard", "export", "--export-dir", "dashboards"]
        )

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            ["export-dashboard", "--export-dir", "dashboards"],
        )

    def test_parse_args_supports_dashboard_shortcut(self):
        args = unified_cli.parse_args(["db", "diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(args.entrypoint, "dashboard")
        self.assertEqual(
            args.forwarded_argv,
            ["diff", "--import-dir", "dashboards/raw"],
        )

    def test_parse_args_supports_dashboard_list_data_sources_to_datasource_list(self):
        args = unified_cli.parse_args(["dashboard", "list-data-sources", "--table"])

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(args.forwarded_argv, ["list", "--table"])

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

    def test_parse_args_supports_alert_shortcut(self):
        args = unified_cli.parse_args(
            ["al", "export", "--output-dir", "./alerts", "--overwrite"]
        )

        self.assertEqual(args.entrypoint, "alert")
        self.assertEqual(
            args.forwarded_argv,
            ["export", "--output-dir", "./alerts", "--overwrite"],
        )

    def test_parse_args_supports_alert_export_namespace(self):
        args = unified_cli.parse_args(
            ["alert", "export", "--output-dir", "./alerts", "--overwrite"]
        )

        self.assertEqual(args.entrypoint, "alert")
        self.assertEqual(
            args.forwarded_argv,
            ["export", "--output-dir", "./alerts", "--overwrite"],
        )

    def test_parse_args_supports_access_namespace(self):
        args = unified_cli.parse_args(
            ["access", "user", "list", "--url", "http://127.0.0.1:3000"]
        )

        self.assertEqual(args.entrypoint, "access")
        self.assertEqual(
            args.forwarded_argv,
            ["user", "list", "--url", "http://127.0.0.1:3000"],
        )

    def test_parse_args_supports_access_shortcut(self):
        args = unified_cli.parse_args(
            ["ac", "user", "list", "--url", "http://127.0.0.1:3000"]
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

    def test_parse_args_supports_datasource_shortcut(self):
        args = unified_cli.parse_args(["ds", "list", "--json"])

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(args.forwarded_argv, ["list", "--json"])

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
            [
                "datasource",
                "modify",
                "--uid",
                "prom-main",
                "--set-url",
                "http://prometheus-v2:9090",
            ]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(
            args.forwarded_argv,
            ["modify", "--uid", "prom-main", "--set-url", "http://prometheus-v2:9090"],
        )

    def test_main_dispatches_dashboard_namespace(self):
        with mock.patch.object(
            unified_cli.dashboard_cli, "main", return_value=7
        ) as mocked:
            result = unified_cli.main(["dashboard", "list", "--json"])

        self.assertEqual(result, 7)
        mocked.assert_called_once_with(["list-dashboard", "--json"])

    def test_parse_args_supports_datasource_diff_namespace(self):
        args = unified_cli.parse_args(
            ["datasource", "diff", "--diff-dir", "./datasources"]
        )

        self.assertEqual(args.entrypoint, "datasource")
        self.assertEqual(args.forwarded_argv, ["diff", "--diff-dir", "./datasources"])

    def test_parse_args_supports_sync_namespace(self):
        args = unified_cli.parse_args(
            [
                "sync",
                "plan",
                "--desired-file",
                "./desired.json",
                "--live-file",
                "./live.json",
            ]
        )

        self.assertEqual(args.entrypoint, "sync")
        self.assertEqual(
            args.forwarded_argv,
            ["plan", "--desired-file", "./desired.json", "--live-file", "./live.json"],
        )

    def test_parse_args_supports_sync_shortcut(self):
        args = unified_cli.parse_args(
            [
                "sy",
                "plan",
                "--desired-file",
                "./desired.json",
                "--live-file",
                "./live.json",
            ]
        )

        self.assertEqual(args.entrypoint, "sync")
        self.assertEqual(
            args.forwarded_argv,
            ["plan", "--desired-file", "./desired.json", "--live-file", "./live.json"],
        )

    def test_parse_args_supports_sync_preflight_namespace(self):
        args = unified_cli.parse_args(
            ["sync", "preflight", "--desired-file", "./desired.json"]
        )

        self.assertEqual(args.entrypoint, "sync")
        self.assertEqual(
            args.forwarded_argv,
            ["preflight", "--desired-file", "./desired.json"],
        )

    def test_parse_args_rejects_unknown_top_level_command(self):
        with self.assertRaises(SystemExit):
            unified_cli.parse_args(["unknown-command"])

    def test_main_dispatches_dashboard_shortcut(self):
        with mock.patch.object(
            unified_cli.dashboard_cli, "main", return_value=7
        ) as mocked:
            result = unified_cli.main(["db", "diff", "--import-dir", "dashboards/raw"])

        self.assertEqual(result, 7)
        mocked.assert_called_once_with(["diff", "--import-dir", "dashboards/raw"])

    def test_main_dispatches_alert_passthrough(self):
        with mock.patch.object(unified_cli.alert_cli, "main", return_value=3) as mocked:
            result = unified_cli.main(["alert", "--diff-dir", "./alerts/raw"])

        self.assertEqual(result, 3)
        mocked.assert_called_once_with(["--diff-dir", "./alerts/raw"])

    def test_main_dispatches_alert_shortcut(self):
        with mock.patch.object(unified_cli.alert_cli, "main", return_value=3) as mocked:
            result = unified_cli.main(
                ["al", "export", "--output-dir", "./alerts", "--overwrite"]
            )

        self.assertEqual(result, 3)
        mocked.assert_called_once_with(
            ["export", "--output-dir", "./alerts", "--overwrite"]
        )

    def test_main_dispatches_access_passthrough(self):
        with mock.patch.object(
            unified_cli.access_cli, "main", return_value=5
        ) as mocked:
            result = unified_cli.main(["access", "team", "list", "--json"])

        self.assertEqual(result, 5)
        mocked.assert_called_once_with(["team", "list", "--json"])

    def test_main_dispatches_access_shortcut(self):
        with mock.patch.object(
            unified_cli.access_cli, "main", return_value=5
        ) as mocked:
            result = unified_cli.main(["ac", "team", "list", "--json"])

        self.assertEqual(result, 5)
        mocked.assert_called_once_with(["team", "list", "--json"])

    def test_main_dispatches_datasource_passthrough(self):
        with mock.patch.object(
            unified_cli.datasource_cli, "main", return_value=9
        ) as mocked:
            result = unified_cli.main(["datasource", "list", "--json"])

        self.assertEqual(result, 9)
        mocked.assert_called_once_with(["list", "--json"])

    def test_main_dispatches_datasource_shortcut(self):
        with mock.patch.object(
            unified_cli.datasource_cli, "main", return_value=9
        ) as mocked:
            result = unified_cli.main(["ds", "add", "--name", "Prometheus Main"])

        self.assertEqual(result, 9)
        mocked.assert_called_once_with(["add", "--name", "Prometheus Main"])

    def test_main_dispatches_sync_passthrough(self):
        with mock.patch.object(unified_cli.sync_cli, "main", return_value=4) as mocked:
            result = unified_cli.main(
                [
                    "sync",
                    "plan",
                    "--desired-file",
                    "./desired.json",
                    "--live-file",
                    "./live.json",
                ]
            )

        self.assertEqual(result, 4)
        mocked.assert_called_once_with(
            ["plan", "--desired-file", "./desired.json", "--live-file", "./live.json"]
        )

    def test_main_dispatches_sync_shortcut(self):
        with mock.patch.object(unified_cli.sync_cli, "main", return_value=4) as mocked:
            result = unified_cli.main(
                [
                    "sy",
                    "plan",
                    "--desired-file",
                    "./desired.json",
                    "--live-file",
                    "./live.json",
                ]
            )

        self.assertEqual(result, 4)
        mocked.assert_called_once_with(
            ["plan", "--desired-file", "./desired.json", "--live-file", "./live.json"]
        )

    def test_main_rejects_unsupported_entrypoint(self):
        with mock.patch.object(
            unified_cli,
            "parse_args",
            return_value=unified_cli.argparse.Namespace(
                entrypoint="unknown",
                forwarded_argv=["--json"],
            ),
        ):
            with self.assertRaisesRegex(
                RuntimeError,
                "Unsupported unified CLI entrypoint: unknown",
            ):
                unified_cli.main(["unknown", "--json"])


if __name__ == "__main__":
    unittest.main()
