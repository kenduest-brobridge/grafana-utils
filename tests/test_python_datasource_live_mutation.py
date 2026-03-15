import ast
import importlib
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = REPO_ROOT / "grafana_utils" / "datasource" / "live_mutation.py"
if str(REPO_ROOT) not in __import__("sys").path:
    __import__("sys").path.insert(0, str(REPO_ROOT))

live_mutation = importlib.import_module("grafana_utils.datasource.live_mutation")
GrafanaError = importlib.import_module("grafana_utils.dashboard_cli").GrafanaError


class FakeDatasourceClient(object):
    def __init__(self, datasources=None):
        self._datasources = list(datasources or [])
        self.calls = []

    def list_datasources(self):
        return list(self._datasources)

    def request_json(self, path, params=None, method="GET", payload=None):
        self.calls.append(
            {
                "path": path,
                "params": dict(params or {}),
                "method": method,
                "payload": payload,
            }
        )
        return {"status": "ok"}


class DatasourceLiveMutationTests(unittest.TestCase):
    def test_live_mutation_module_parses_as_python39_syntax(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        ast.parse(source, filename=str(MODULE_PATH), feature_version=(3, 9))

    def test_build_add_payload_keeps_optional_json_fields(self):
        payload = live_mutation.build_add_payload(
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": "true",
                "jsonData": {"httpMethod": "POST"},
                "secureJsonData": {"httpHeaderValue1": "secret"},
            }
        )

        self.assertEqual(
            payload,
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "access": "proxy",
                "url": "http://prometheus:9090",
                "isDefault": True,
                "jsonData": {"httpMethod": "POST"},
                "secureJsonData": {"httpHeaderValue1": "secret"},
            },
        )

    def test_plan_add_datasource_returns_would_create_when_missing(self):
        client = FakeDatasourceClient(datasources=[])

        plan = live_mutation.plan_add_datasource(
            client,
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
            },
        )

        self.assertEqual(plan["action"], "would-create")
        self.assertEqual(plan["match"], "missing")

    def test_add_datasource_posts_payload_when_not_dry_run(self):
        client = FakeDatasourceClient(datasources=[])

        result = live_mutation.add_datasource(
            client,
            {
                "uid": "prom-main",
                "name": "Prometheus Main",
                "type": "prometheus",
                "url": "http://prometheus:9090",
            },
        )

        self.assertEqual(result["action"], "created")
        self.assertEqual(
            client.calls,
            [
                {
                    "path": "/api/datasources",
                    "params": {},
                    "method": "POST",
                    "payload": {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "url": "http://prometheus:9090",
                    },
                }
            ],
        )

    def test_add_datasource_rejects_existing_uid_or_name(self):
        client = FakeDatasourceClient(
            datasources=[
                {
                    "id": 7,
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                }
            ]
        )

        with self.assertRaisesRegex(GrafanaError, "would-fail-existing"):
            live_mutation.add_datasource(
                client,
                {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                },
            )

    def test_plan_delete_datasource_returns_would_delete_for_uid_match(self):
        client = FakeDatasourceClient(
            datasources=[
                {
                    "id": 7,
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                }
            ]
        )

        plan = live_mutation.plan_delete_datasource(client, uid="prom-main")

        self.assertEqual(plan["action"], "would-delete")
        self.assertEqual(plan["match"], "exists-uid")
        self.assertEqual(plan["target"]["id"], 7)

    def test_delete_datasource_issues_delete_for_live_id(self):
        client = FakeDatasourceClient(
            datasources=[
                {
                    "id": 7,
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                }
            ]
        )

        result = live_mutation.delete_datasource(client, uid="prom-main")

        self.assertEqual(result["action"], "deleted")
        self.assertEqual(
            client.calls,
            [
                {
                    "path": "/api/datasources/7",
                    "params": {},
                    "method": "DELETE",
                    "payload": None,
                }
            ],
        )

    def test_delete_datasource_dry_run_does_not_call_api(self):
        client = FakeDatasourceClient(
            datasources=[
                {"id": 9, "uid": "logs-main", "name": "Loki Logs", "type": "loki"}
            ]
        )

        plan = live_mutation.delete_datasource(client, uid="logs-main", dry_run=True)

        self.assertEqual(plan["action"], "would-delete")
        self.assertEqual(client.calls, [])

    def test_delete_datasource_rejects_missing_target(self):
        client = FakeDatasourceClient(datasources=[])

        with self.assertRaisesRegex(GrafanaError, "would-fail-missing"):
            live_mutation.delete_datasource(client, uid="missing-main")

    def test_delete_datasource_rejects_uid_name_mismatch(self):
        client = FakeDatasourceClient(
            datasources=[
                {"id": 9, "uid": "logs-main", "name": "Loki Logs", "type": "loki"}
            ]
        )

        with self.assertRaisesRegex(GrafanaError, "would-fail-ambiguous"):
            live_mutation.delete_datasource(
                client,
                uid="logs-main",
                name="Prometheus Main",
            )

    def test_normalize_add_spec_rejects_unknown_fields(self):
        with self.assertRaisesRegex(GrafanaError, "unsupported field"):
            live_mutation.normalize_add_spec(
                {
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "orgId": "1",
                }
            )
