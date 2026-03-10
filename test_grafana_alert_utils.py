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


def sample_contact_point(**overrides):
    contact_point = {
        "uid": "cp-uid",
        "name": "Webhook Main",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1:18080/notify"},
        "disableResolveMessage": False,
        "provenance": "api",
    }
    contact_point.update(overrides)
    return contact_point


def sample_mute_timing(**overrides):
    mute_timing = {
        "name": "weekday-maintenance",
        "time_intervals": [
            {
                "times": [{"start_time": "00:00", "end_time": "23:59"}],
                "weekdays": ["monday:friday"],
                "location": "UTC",
            }
        ],
        "version": "version-1",
        "provenance": "api",
    }
    mute_timing.update(overrides)
    return mute_timing


def sample_policies(**overrides):
    policies = {
        "receiver": "Webhook Main",
        "group_by": ["grafana_folder", "alertname"],
        "routes": [
            {
                "receiver": "Webhook Main",
                "object_matchers": [["severity", "=", "warning"]],
                "mute_time_intervals": ["weekday-maintenance"],
            }
        ],
        "provenance": "api",
    }
    policies.update(overrides)
    return policies


class FakeAlertClient:
    def __init__(
        self,
        rules=None,
        contact_points=None,
        mute_timings=None,
        policies=None,
        existing_rules=None,
    ):
        self.rules = [dict(rule) for rule in (rules or [])]
        self.contact_points = [dict(item) for item in (contact_points or [])]
        self.mute_timings = [dict(item) for item in (mute_timings or [])]
        self.policies = dict(policies or sample_policies())
        self.existing_rules = {
            uid: dict(rule) for uid, rule in (existing_rules or {}).items()
        }
        self.created_rules = []
        self.updated_rules = []
        self.rule_lookups = []
        self.created_contact_points = []
        self.updated_contact_points = []
        self.created_mute_timings = []
        self.updated_mute_timings = []
        self.updated_policies = []

    def list_alert_rules(self):
        return [dict(rule) for rule in self.rules]

    def get_alert_rule(self, uid):
        self.rule_lookups.append(uid)
        if uid not in self.existing_rules:
            raise alert_utils.GrafanaApiError(404, f"https://grafana/{uid}", "not found")
        return dict(self.existing_rules[uid])

    def create_alert_rule(self, payload):
        self.created_rules.append(dict(payload))
        return {"uid": payload.get("uid") or "created-uid"}

    def update_alert_rule(self, uid, payload):
        self.updated_rules.append((uid, dict(payload)))
        return {"uid": uid}

    def list_contact_points(self):
        return [dict(item) for item in self.contact_points]

    def create_contact_point(self, payload):
        self.created_contact_points.append(dict(payload))
        return {"uid": payload.get("uid") or "created-contact-point"}

    def update_contact_point(self, uid, payload):
        self.updated_contact_points.append((uid, dict(payload)))
        return {"uid": uid}

    def list_mute_timings(self):
        return [dict(item) for item in self.mute_timings]

    def create_mute_timing(self, payload):
        self.created_mute_timings.append(dict(payload))
        return {"name": payload.get("name") or "created-mute-timing"}

    def update_mute_timing(self, name, payload):
        self.updated_mute_timings.append((name, dict(payload)))
        return {"name": name}

    def get_notification_policies(self):
        return dict(self.policies)

    def update_notification_policies(self, payload):
        self.updated_policies.append(dict(payload))
        self.policies = dict(payload)
        return {"message": "policies updated"}


class AlertUtilsTests(unittest.TestCase):
    def test_parse_args_supports_import_mode(self):
        args = alert_utils.parse_args(["--import-dir", "alerts/raw"])

        self.assertEqual(args.import_dir, "alerts/raw")

    def test_parse_args_defaults_output_dir_to_alerts(self):
        args = alert_utils.parse_args([])

        self.assertEqual(args.output_dir, "alerts")

    def test_parse_args_defaults_url_to_local_grafana(self):
        args = alert_utils.parse_args([])

        self.assertEqual(args.url, "http://127.0.0.1:3000")

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

    def test_build_rule_output_path_keeps_folder_and_rule_group_structure(self):
        path = alert_utils.build_rule_output_path(
            Path("alerts/raw/rules"),
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
            Path("alerts/raw/rules/infra_folder/CPU_Alerts/DB_CPU_90__rule-1.json"),
        )

    def test_build_contact_point_output_path_uses_name_and_uid(self):
        path = alert_utils.build_contact_point_output_path(
            Path("alerts/raw/contact-points"),
            {"name": "Webhook Main", "uid": "cp-uid"},
            flat=False,
        )

        self.assertEqual(
            path,
            Path("alerts/raw/contact-points/Webhook_Main/Webhook_Main__cp-uid.json"),
        )

    def test_build_mute_timing_output_path_uses_name(self):
        path = alert_utils.build_mute_timing_output_path(
            Path("alerts/raw/mute-timings"),
            {"name": "weekday maintenance"},
            flat=False,
        )

        self.assertEqual(
            path,
            Path("alerts/raw/mute-timings/weekday_maintenance/weekday_maintenance.json"),
        )

    def test_build_resource_dirs(self):
        dirs = alert_utils.build_resource_dirs(Path("alerts/raw"))

        self.assertEqual(dirs[alert_utils.RULE_KIND], Path("alerts/raw/rules"))
        self.assertEqual(
            dirs[alert_utils.CONTACT_POINT_KIND],
            Path("alerts/raw/contact-points"),
        )
        self.assertEqual(
            dirs[alert_utils.MUTE_TIMING_KIND],
            Path("alerts/raw/mute-timings"),
        )
        self.assertEqual(
            dirs[alert_utils.POLICIES_KIND],
            Path("alerts/raw/policies"),
        )

    def test_discover_alert_resource_files_ignores_index_json(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "index.json").write_text("[]", encoding="utf-8")
            resource_path = root / "rules" / "infra" / "group" / "rule.json"
            resource_path.parent.mkdir(parents=True, exist_ok=True)
            resource_path.write_text(
                '{"kind":"grafana-alert-rule","spec":{"title":"x","folderUID":"f","ruleGroup":"g","condition":"A","data":[]}}',
                encoding="utf-8",
            )

            files = alert_utils.discover_alert_resource_files(root)

            self.assertEqual(files, [resource_path])

    def test_discover_alert_resource_files_rejects_export_root(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "raw").mkdir()

            with self.assertRaises(alert_utils.GrafanaError):
                alert_utils.discover_alert_resource_files(root)

    def test_build_rule_export_document_strips_server_managed_fields(self):
        document = alert_utils.build_rule_export_document(sample_rule())

        self.assertEqual(document["kind"], alert_utils.RULE_KIND)
        self.assertEqual(document["apiVersion"], alert_utils.TOOL_API_VERSION)
        self.assertEqual(document["metadata"]["uid"], "rule-uid")
        self.assertNotIn("id", document["spec"])
        self.assertNotIn("updated", document["spec"])
        self.assertNotIn("provenance", document["spec"])

    def test_build_contact_point_export_document_strips_server_managed_fields(self):
        document = alert_utils.build_contact_point_export_document(
            sample_contact_point()
        )

        self.assertEqual(document["kind"], alert_utils.CONTACT_POINT_KIND)
        self.assertEqual(document["metadata"]["uid"], "cp-uid")
        self.assertNotIn("provenance", document["spec"])

    def test_build_mute_timing_export_document_strips_server_managed_fields(self):
        document = alert_utils.build_mute_timing_export_document(sample_mute_timing())

        self.assertEqual(document["kind"], alert_utils.MUTE_TIMING_KIND)
        self.assertEqual(document["metadata"]["name"], "weekday-maintenance")
        self.assertNotIn("version", document["spec"])
        self.assertNotIn("provenance", document["spec"])

    def test_build_policies_export_document_strips_server_managed_fields(self):
        document = alert_utils.build_policies_export_document(sample_policies())

        self.assertEqual(document["kind"], alert_utils.POLICIES_KIND)
        self.assertEqual(document["metadata"]["receiver"], "Webhook Main")
        self.assertNotIn("provenance", document["spec"])

    def test_build_import_operation_accepts_rule_tool_document(self):
        document = alert_utils.build_rule_export_document(sample_rule())

        kind, payload = alert_utils.build_import_operation(document)

        self.assertEqual(kind, alert_utils.RULE_KIND)
        self.assertEqual(payload["uid"], "rule-uid")
        self.assertEqual(payload["folderUID"], "infra-folder")
        self.assertNotIn("id", payload)

    def test_build_import_operation_accepts_contact_point_tool_document(self):
        document = alert_utils.build_contact_point_export_document(
            sample_contact_point()
        )

        kind, payload = alert_utils.build_import_operation(document)

        self.assertEqual(kind, alert_utils.CONTACT_POINT_KIND)
        self.assertEqual(payload["uid"], "cp-uid")
        self.assertEqual(payload["type"], "webhook")

    def test_build_import_operation_accepts_mute_timing_tool_document(self):
        document = alert_utils.build_mute_timing_export_document(sample_mute_timing())

        kind, payload = alert_utils.build_import_operation(document)

        self.assertEqual(kind, alert_utils.MUTE_TIMING_KIND)
        self.assertEqual(payload["name"], "weekday-maintenance")
        self.assertEqual(len(payload["time_intervals"]), 1)

    def test_build_import_operation_accepts_policies_tool_document(self):
        document = alert_utils.build_policies_export_document(sample_policies())

        kind, payload = alert_utils.build_import_operation(document)

        self.assertEqual(kind, alert_utils.POLICIES_KIND)
        self.assertEqual(payload["receiver"], "Webhook Main")

    def test_build_import_operation_accepts_plain_rule_document(self):
        kind, payload = alert_utils.build_import_operation(sample_rule())

        self.assertEqual(kind, alert_utils.RULE_KIND)
        self.assertEqual(payload["uid"], "rule-uid")

    def test_build_import_operation_rejects_provisioning_export_format(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_import_operation(
                {"apiVersion": 1, "contactPoints": [{"name": "Webhook Main"}]}
            )

    def test_build_rule_import_payload_requires_expected_fields(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_rule_import_payload({"title": "CPU High"})

    def test_build_contact_point_import_payload_requires_expected_fields(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_contact_point_import_payload({"name": "Webhook Main"})

    def test_build_mute_timing_import_payload_requires_expected_fields(self):
        with self.assertRaises(alert_utils.GrafanaError):
            alert_utils.build_mute_timing_import_payload({"name": "weekday-maintenance"})

    def test_export_alerting_resources_writes_all_resource_types(self):
        args = alert_utils.parse_args(["--output-dir", "unused", "--overwrite"])
        fake_client = FakeAlertClient(
            rules=[sample_rule()],
            contact_points=[sample_contact_point()],
            mute_timings=[sample_mute_timing()],
            policies=sample_policies(),
        )

        with tempfile.TemporaryDirectory() as tmpdir:
            args.output_dir = tmpdir
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.export_alerting_resources(args)

            self.assertEqual(result, 0)
            raw_dir = Path(tmpdir) / "raw"
            self.assertTrue(
                (
                    raw_dir
                    / "rules"
                    / "infra-folder"
                    / "cpu-alerts"
                    / "CPU_High__rule-uid.json"
                ).exists()
            )
            self.assertTrue(
                (
                    raw_dir
                    / "contact-points"
                    / "Webhook_Main"
                    / "Webhook_Main__cp-uid.json"
                ).exists()
            )
            self.assertTrue(
                (
                    raw_dir
                    / "mute-timings"
                    / "weekday-maintenance"
                    / "weekday-maintenance.json"
                ).exists()
            )
            self.assertTrue(
                (raw_dir / "policies" / "notification-policies.json").exists()
            )
            root_index = alert_utils.load_json_file(Path(tmpdir) / "index.json")
            self.assertEqual(len(root_index["rules"]), 1)
            self.assertEqual(len(root_index["contact-points"]), 1)
            self.assertEqual(len(root_index["mute-timings"]), 1)
            self.assertEqual(len(root_index["policies"]), 1)

    def test_import_alerting_resources_updates_existing_rule_when_requested(self):
        args = alert_utils.parse_args(["--import-dir", "unused", "--replace-existing"])
        fake_client = FakeAlertClient(existing_rules={"rule-uid": sample_rule()})

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            rule_path = Path(tmpdir) / "rule.json"
            alert_utils.write_json(
                alert_utils.build_rule_export_document(sample_rule()),
                rule_path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alerting_resources(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.rule_lookups, ["rule-uid"])
            self.assertEqual(fake_client.created_rules, [])
            self.assertEqual(len(fake_client.updated_rules), 1)
            self.assertEqual(fake_client.updated_rules[0][0], "rule-uid")

    def test_import_alerting_resources_updates_existing_contact_point_when_requested(self):
        args = alert_utils.parse_args(["--import-dir", "unused", "--replace-existing"])
        fake_client = FakeAlertClient(contact_points=[sample_contact_point()])

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            path = Path(tmpdir) / "contact-point.json"
            alert_utils.write_json(
                alert_utils.build_contact_point_export_document(sample_contact_point()),
                path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alerting_resources(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.created_contact_points, [])
            self.assertEqual(len(fake_client.updated_contact_points), 1)
            self.assertEqual(fake_client.updated_contact_points[0][0], "cp-uid")

    def test_import_alerting_resources_updates_existing_mute_timing_when_requested(self):
        args = alert_utils.parse_args(["--import-dir", "unused", "--replace-existing"])
        fake_client = FakeAlertClient(mute_timings=[sample_mute_timing()])

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            path = Path(tmpdir) / "mute-timing.json"
            alert_utils.write_json(
                alert_utils.build_mute_timing_export_document(sample_mute_timing()),
                path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alerting_resources(args)

            self.assertEqual(result, 0)
            self.assertEqual(fake_client.created_mute_timings, [])
            self.assertEqual(len(fake_client.updated_mute_timings), 1)
            self.assertEqual(
                fake_client.updated_mute_timings[0][0], "weekday-maintenance"
            )

    def test_import_alerting_resources_updates_policies(self):
        args = alert_utils.parse_args(["--import-dir", "unused"])
        fake_client = FakeAlertClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            path = Path(tmpdir) / "notification-policies.json"
            alert_utils.write_json(
                alert_utils.build_policies_export_document(sample_policies()),
                path,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                result = alert_utils.import_alerting_resources(args)

            self.assertEqual(result, 0)
            self.assertEqual(len(fake_client.updated_policies), 1)
            self.assertEqual(fake_client.updated_policies[0]["receiver"], "Webhook Main")

    def test_import_alerting_resources_rejects_multiple_policy_documents(self):
        args = alert_utils.parse_args(["--import-dir", "unused"])
        fake_client = FakeAlertClient()

        with tempfile.TemporaryDirectory() as tmpdir:
            args.import_dir = tmpdir
            first = Path(tmpdir) / "notification-policies-a.json"
            second = Path(tmpdir) / "notification-policies-b.json"
            alert_utils.write_json(
                alert_utils.build_policies_export_document(sample_policies()),
                first,
                overwrite=True,
            )
            alert_utils.write_json(
                alert_utils.build_policies_export_document(
                    sample_policies(receiver="Webhook Secondary")
                ),
                second,
                overwrite=True,
            )
            with mock.patch.object(alert_utils, "build_client", return_value=fake_client):
                with self.assertRaises(alert_utils.GrafanaError):
                    alert_utils.import_alerting_resources(args)


if __name__ == "__main__":
    unittest.main()
