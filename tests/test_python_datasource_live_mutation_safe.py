import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "live_mutation_safe.py"
safe_mutation = importlib.import_module("grafana_utils.datasource.live_mutation_safe")
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class FakeDatasourceClient(object):
    def __init__(self, datasources=None):
        self._datasources = list(datasources or [])
        self.calls = []

    def list_datasources(self):
        return list(self._datasources)

    def create_datasource(self, payload):
        self.calls.append(("create", payload))
        return {"status": "created"}

    def delete_datasource(self, datasource_id):
        self.calls.append(("delete", datasource_id))
        return {"status": "deleted"}


class DatasourceLiveMutationSafeTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_build_add_payload_omits_none_json_fields(self):
        payload = safe_mutation.build_add_payload(
            {
                "name": "Prometheus Main",
                "type": "prometheus",
                "jsonData": None,
                "secureJsonData": None,
            }
        )

        self.assertEqual(
            payload,
            {
                "name": "Prometheus Main",
                "type": "prometheus",
            },
        )

    def test_plan_add_datasource_distinguishes_existing_name(self):
        client = FakeDatasourceClient(
            datasources=[{"id": 1, "uid": "prom-live", "name": "Prometheus Main"}]
        )

        plan = safe_mutation.plan_add_datasource(
            client,
            {"name": "Prometheus Main", "type": "prometheus"},
        )

        self.assertEqual(plan["match"], "exists-name")
        self.assertEqual(plan["action"], "would-fail-existing-name")

    def test_plan_add_datasource_distinguishes_uid_name_mismatch(self):
        client = FakeDatasourceClient(
            datasources=[{"id": 1, "uid": "prom-live", "name": "Prometheus Main"}]
        )

        plan = safe_mutation.plan_add_datasource(
            client,
            {"uid": "prom-live", "name": "Other Name", "type": "prometheus"},
        )

        self.assertEqual(plan["match"], "uid-name-mismatch")
        self.assertEqual(plan["action"], "would-fail-uid-name-mismatch")

    def test_add_datasource_uses_client_helper_when_available(self):
        client = FakeDatasourceClient(datasources=[])

        result = safe_mutation.add_datasource(
            client,
            {"uid": "prom-main", "name": "Prometheus Main", "type": "prometheus"},
        )

        self.assertEqual(result["action"], "created")
        self.assertEqual(client.calls, [("create", result["payload"])])

    def test_delete_datasource_uses_client_helper_when_available(self):
        client = FakeDatasourceClient(
            datasources=[{"id": 7, "uid": "prom-main", "name": "Prometheus Main"}]
        )

        result = safe_mutation.delete_datasource(client, uid="prom-main")

        self.assertEqual(result["action"], "deleted")
        self.assertEqual(client.calls, [("delete", 7)])

    def test_delete_datasource_surfaces_specific_mismatch_action(self):
        client = FakeDatasourceClient(
            datasources=[{"id": 7, "uid": "prom-main", "name": "Prometheus Main"}]
        )

        with self.assertRaisesRegex(GrafanaError, "would-fail-uid-name-mismatch"):
            safe_mutation.delete_datasource(
                client,
                uid="prom-main",
                name="Other Name",
            )
