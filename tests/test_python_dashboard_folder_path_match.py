import tempfile
import unittest
from pathlib import Path

from grafana_utils.dashboards import folder_path_match


class FakeFolderPathClient:
    def __init__(self, dashboards=None, folders=None):
        self.dashboards = dashboards or {}
        self.folders = folders or {}

    def fetch_dashboard_if_exists(self, uid):
        return self.dashboards.get(uid)

    def fetch_folder_if_exists(self, uid):
        return self.folders.get(uid)


class FolderPathMatchTests(unittest.TestCase):
    def test_resolve_source_dashboard_folder_path_uses_inventory_record(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_dir = import_dir / "ignored"
            dashboard_dir.mkdir(parents=True)
            dashboard_file = dashboard_dir / "cpu.json"
            dashboard_file.write_text("{}", encoding="utf-8")
            document = {"meta": {"folderUid": "infra"}}
            folder_lookup = {
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "path": "Platform / Infra",
                }
            }

            result = folder_path_match.resolve_source_dashboard_folder_path(
                document,
                dashboard_file,
                import_dir,
                folder_lookup,
            )

        self.assertEqual(result, "Platform / Infra")

    def test_resolve_source_dashboard_folder_path_falls_back_to_file_layout(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_dir = import_dir / "Platform" / "Infra"
            dashboard_dir.mkdir(parents=True)
            dashboard_file = dashboard_dir / "cpu.json"
            dashboard_file.write_text("{}", encoding="utf-8")

            result = folder_path_match.resolve_source_dashboard_folder_path(
                {},
                dashboard_file,
                import_dir,
                {},
            )

        self.assertEqual(result, "Platform / Infra")

    def test_resolve_source_dashboard_folder_path_defaults_root_to_general(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            import_dir = Path(tmpdir)
            dashboard_file = import_dir / "cpu.json"
            dashboard_file.write_text("{}", encoding="utf-8")

            result = folder_path_match.resolve_source_dashboard_folder_path(
                {},
                dashboard_file,
                import_dir,
                {},
            )

        self.assertEqual(result, "General")

    def test_resolve_existing_dashboard_folder_path_returns_general_for_builtin(self):
        client = FakeFolderPathClient(
            dashboards={
                "cpu-main": {
                    "dashboard": {"uid": "cpu-main"},
                    "meta": {"folderUid": "general"},
                }
            }
        )

        result = folder_path_match.resolve_existing_dashboard_folder_path(
            client,
            "cpu-main",
        )

        self.assertEqual(result, "General")

    def test_resolve_existing_dashboard_folder_path_builds_nested_live_path(self):
        client = FakeFolderPathClient(
            dashboards={
                "cpu-main": {
                    "dashboard": {"uid": "cpu-main"},
                    "meta": {"folderUid": "infra"},
                }
            },
            folders={
                "infra": {
                    "uid": "infra",
                    "title": "Infra",
                    "parentUid": "platform",
                },
                "platform": {
                    "uid": "platform",
                    "title": "Platform",
                    "parentUid": "",
                },
            },
        )

        result = folder_path_match.resolve_existing_dashboard_folder_path(
            client,
            "cpu-main",
        )

        self.assertEqual(result, "Platform / Infra")

    def test_build_folder_path_match_result_accepts_missing_destination(self):
        result = folder_path_match.build_folder_path_match_result(
            source_folder_path="Platform / Infra",
            destination_folder_path=None,
            destination_exists=False,
            require_matching_folder_path=True,
        )

        self.assertTrue(result["matches"])
        self.assertEqual(result["source_folder_path"], "Platform / Infra")
        self.assertIsNone(result["destination_folder_path"])

    def test_build_folder_path_match_result_reports_exact_match(self):
        result = folder_path_match.build_folder_path_match_result(
            source_folder_path="Platform / Infra",
            destination_folder_path="Platform / Infra",
            destination_exists=True,
            require_matching_folder_path=True,
        )

        self.assertTrue(result["matches"])
        self.assertEqual(result["reason"], "")

    def test_build_folder_path_match_result_reports_mismatch(self):
        result = folder_path_match.build_folder_path_match_result(
            source_folder_path="Platform / Infra",
            destination_folder_path="Legacy / Infra",
            destination_exists=True,
            require_matching_folder_path=True,
        )

        self.assertFalse(result["matches"])
        self.assertEqual(result["reason"], "folder-path-mismatch")

    def test_build_folder_path_match_result_reports_unknown_destination_path(self):
        result = folder_path_match.build_folder_path_match_result(
            source_folder_path="Platform / Infra",
            destination_folder_path=None,
            destination_exists=True,
            require_matching_folder_path=True,
        )

        self.assertFalse(result["matches"])
        self.assertEqual(result["reason"], "folder-path-unknown")

    def test_apply_folder_path_guard_to_action_rewrites_only_updates(self):
        mismatch = {
            "matches": False,
            "reason": "folder-path-mismatch",
            "source_folder_path": "Platform / Infra",
            "destination_folder_path": "Legacy / Infra",
            "destination_exists": True,
        }

        self.assertEqual(
            folder_path_match.apply_folder_path_guard_to_action(
                "would-update",
                mismatch,
            ),
            "would-skip-folder-mismatch",
        )
        self.assertEqual(
            folder_path_match.apply_folder_path_guard_to_action(
                "would-create",
                mismatch,
            ),
            "would-create",
        )


if __name__ == "__main__":
    unittest.main()
