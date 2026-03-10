import argparse
import base64
import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("grafana-alert-utils.py")
SPEC = importlib.util.spec_from_file_location("grafana_alert_utils_script", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"Cannot load module from {MODULE_PATH}")
alert_utils = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(alert_utils)


def sample_rule(**overrides):
    rule = {
        "id": 12,
        "uid": "rule-uid",
        "orgID": 1,
        "folderUID": "infra-folder",
        "ruleGroup": "cpu-alerts",
        "title": "CPU High",
        "condition": "C",
        "data": [
            {
                "refId": "A",
                "relativeTimeRange": {"from": 300, "to": 0},
                "datasourceUid": "__expr__",
                "model": {"type": "math", "expression": "1"},
            }
        ],
        "noDataState": "NoData",
        "execErrState": "Error",
        "for": "5m",
        "annotations": {"summary": "CPU too high"},
        "labels": {"severity": "warning"},
        "updated": "2026-03-10T10:00:00Z",
        "provenance": "api",
        "isPaused": False,
    }
    rule.update(overrides)
    return rule


class FakeAlertClient:
    def __init__(self, rules=None, existing=None):
        self.rules = [dict(rule) for rule in (rules or [])]
        self.existing = {uid: dict(rule) for uid, rule in (existing or {}).items()}
        self.created = []
        self.updated = []
        self.lookups = []

    def list_alert_rules(self):
        return [dict(rule) for rule in self.rules]

    def get_alert_rule(self, uid):
        self.lookups.append(uid)
        if uid not in self.existing:
            raise alert_utils.GrafanaApiError(404, f"https://grafana/{uid}", "not found")
        return dict(self.existing[uid])

    def create_alert_rule(self, payload):
        self.created.append(dict(payload))
        return {"uid": payload.get("uid") or "created-uid"}

    def update_alert_rule(self, uid, payload):
        self.updated.append((uid, dict(payload)))
        return {"uid": uid}


class AlertUtilsTests(unittest.TestCase):
    def test_parse_args_supports_import_mode(self):
        args = alert_utils.parse_args(["--import-dir", "alerts/raw"])

        self.assertEqual(args.import_dir, "alerts/raw")

    def test_parse_args_defaults_output_dir_to_alerts(self):
        args = alert_utils.parse_args([])

        self.assertEqual(args.output_dir, "alerts")

    def test_parse_args_disables_ssl_verification_by_default(self):
        args = alert_utils.parse_args([])

        self.assertFalse(args.verify_ssl)

    def test_parse_args_can_enable_ssl_verification(self):
        args = alert_utils.parse_args(["--verify-ssl"])

        self.assertTrue(args.verify_ssl)

    def test_resolve_auth_prefers_token(self):
        args = argparse.Namespace(
            api_token="abc123",
            username="user",
            password="pass",
        )

        headers = alert_utils.resolve_auth(args)

        self.assertEqual(headers["Authorization"], "Bearer abc123")

    def test_resolve_auth_supports_basic_auth(self):
        args = argparse.Namespace(
            api_token=None,
            username="user",
            password="pass",
        )

        headers = alert_utils.resolve_auth(args)

        expected = base64.b64encode(b"user:pass").decode("ascii")
        self.assertEqual(headers["Authorization"], f"Basic {expected}")

    def test_build_output_path_keeps_folder_and_rule_group_structure(self):
        path = alert_utils.build_output_path(
            Path("alerts/raw"),
            {
                "folderUID": "infra folder",
                "ruleGroup": "CPU Alerts",
                "title": "DB CPU > 90%",
                "uid": "rule-1",
            },
            flat=False,
        )

        self.assertEqual(
            path,
            Path("alerts/raw/infra_folder/CPU_Alerts/DB_CPU_90__rule-1.json"),
        )

    def test_discover_alert_rule_files_ignores_index_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "index.json").write_text("[]", encoding="utf-8")
            rule_path = root / "infra" / "group" / "rule.json"
            rule_path.parent.mkdir(parents=True, exist_ok=True)
            rule_path.write_text('{"kind":"grafana-alert-rule","spec":{"title":"x","folderUID":"f","ruleGroup":"g","condition":"A","data":[]}}', encoding="utf-8")

            files = alert_utils.discover_alert_rule_files(root)

            self.assertEqual(files, [rule_path])

    def test_discover_alert_rule_files_rejects_export_root(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "raw").mkdir()

            with self.assertRaises(alert_utils.GrafanaError):
                alert_utils.discover_alert_rule_files(root)

    def test_build_export_document_strips_server_managed_fields(self):
        document = alert_utils.build_export_document(sample_rule())

        self.assertEqual(document["kind"], alert_utils.TOOL_KIND)
        self.assertEqual(document["apiVersion"], alert_utils.TOOL_API_VERSION)
        self.assertEqual(document["metadata"]["uid"], "rule-uid")
        self.assertNotIn("id", document["spec"])
        self.assertNotIn("updated", document["spec"])
        self.assertNotIn("provenance", document["spec"])

    def test_build_import_payload_accepts_tool_document(self):
        document = alert_utils.build_export_document(sample_rule())

        payload = alert_utils.build_import_payload(document)

        self.assertEqual(payload["uid"], "rule-uid")
        self.assertEqual(payload["folderUID"], "infra-folder")
        self.assertNotIn("id", payload)
        self.assertNotIn("updated", payload)

    def test_build_import_payload_rejects_provisioning_export_format(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_import_payload(
                {"apiVersion": 1, "groups": [{"name": "cpu-alerts"}]}
            )

    def test_build_import_payload_requires_expected_fields(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_import_payload({"title": "CPU High"})

    def test_export_alert_rules_writes_rule_files_and_indexes(self):
        args = alert_utils.parse_args(["--output-dir", "unused", "--overwrite"])
        fake_client = FakeAlertClient(
            rules=[
                sample_rule(),
                sample_rule(
                    uid="rule-2",
                    title="Memory High",
                    ruleGroup="memory alerts",
                    folderUID="ops-folder",
                ),
            ]
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.output_dir = tmpdir
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.export_alert_rules(args)

            self.assertEqual(result, 0)
            raw_dir = Path(tmpdir) / "raw"
            rule_path = raw_dir / "infra-folder" / "cpu-alerts" / "CPU_High__rule-uid.json"
            second_rule_path = raw_dir / "ops-folder" / "memory_alerts" / "Memory_High__rule-2.json"
            self.assertTrue(rule_path.exists())
            self.assertTrue(second_rule_path.exists())
            raw_index = alert_utils.load_json_file(raw_dir / "index.json")
            root_index = alert_utils.load_json_file(Path(tmpdir) / "index.json")
            self.assertEqual(len(raw_index), 2)
            self.assertEqual(root_index, raw_index)

    def test_import_alert_rules_updates_existing_rule_when_requested(self):
        args = alert_utils.parse_args(["--import-dir", "unused", "--replace-existing"])
        fake_client = FakeAlertClient(existing={"rule-uid": sample_rule()})

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            rule_path = Path(tmpdir) / "rule.json"
            alert_utils.write_json(
                alert_utils.build_export_document(sample_rule()),
                rule_path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alert_rules(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.lookups, ["rule-uid"])
            self.assertEqual(fake_client.created, [])
            self.assertEqual(len(fake_client.updated), 1)
            self.assertEqual(fake_client.updated[0][0], "rule-uid")

    def test_import_alert_rules_creates_rule_when_uid_not_found(self):
        args = alert_utils.parse_args(["--import-dir", "unused", "--replace-existing"])
        fake_client = FakeAlertClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            rule_path = Path(tmpdir) / "rule.json"
            alert_utils.write_json(
                alert_utils.build_export_document(sample_rule()),
                rule_path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alert_rules(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.lookups, ["rule-uid"])
            self.assertEqual(len(fake_client.created), 1)
            self.assertEqual(fake_client.updated, [])

    def test_import_alert_rules_creates_without_lookup_by_default(self):
        args = alert_utils.parse_args(["--import-dir", "unused"])
        fake_client = FakeAlertClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            rule_path = Path(tmpdir) / "rule.json"
            alert_utils.write_json(
                alert_utils.build_export_document(sample_rule(uid="new-rule")),
                rule_path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alert_rules(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.lookups, [])
            self.assertEqual(len(fake_client.created), 1)


if __name__ == "__main__":
    unittest.main()
