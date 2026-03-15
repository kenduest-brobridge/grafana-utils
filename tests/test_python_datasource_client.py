import ast
import importlib
import unittest


datasource_client = importlib.import_module("grafana_utils.clients.datasource_client")
GrafanaError = importlib.import_module("grafana_utils.dashboards.common").GrafanaError
GrafanaApiError = importlib.import_module(
    "grafana_utils.dashboards.common"
).GrafanaApiError


class StubTransport(object):
    def __init__(self, responses=None, error=None):
        self.responses = dict(responses or {})
        self.error = error
        self.calls = []

    def request_json(self, path, params=None, method="GET", payload=None):
        self.calls.append(
            {
                "path": path,
                "params": dict(params or {}),
                "method": method,
                "payload": payload,
            }
        )
        if self.error is not None:
            raise self.error
        key = (method, path)
        if key not in self.responses:
            raise AssertionError("Unexpected request %s %s" % (method, path))
        return self.responses[key]


class DatasourceClientTests(unittest.TestCase):
    def test_module_parses_as_python39_syntax(self):
        source = open(
            "grafana_utils/clients/datasource_client.py",
            "r",
            encoding="utf-8",
        ).read()
        try:
            ast.parse(
                source,
                filename="grafana_utils/clients/datasource_client.py",
                feature_version=(3, 9),
            )
        finally:
            pass

    def test_list_datasources_returns_dict_items_only(self):
        client = datasource_client.GrafanaDatasourceClient(
            base_url="http://grafana.example.com",
            headers={},
            timeout=30,
            verify_ssl=True,
            transport=StubTransport(
                responses={
                    ("GET", "/api/datasources"): [
                        {"id": 1, "uid": "prom-main"},
                        "ignore-me",
                    ]
                }
            ),
        )

        rows = client.list_datasources()

        self.assertEqual(rows, [{"id": 1, "uid": "prom-main"}])

    def test_fetch_datasource_by_uid_if_exists_returns_none_on_404(self):
        class MissingTransport(StubTransport):
            def request_json(self, path, params=None, method="GET", payload=None):
                raise datasource_client.GrafanaApiError(404, path, "not found")

        client = datasource_client.GrafanaDatasourceClient(
            base_url="http://grafana.example.com",
            headers={},
            timeout=30,
            verify_ssl=True,
            transport=MissingTransport(),
        )

        self.assertIsNone(client.fetch_datasource_by_uid_if_exists("missing"))

    def test_create_datasource_posts_payload(self):
        transport = StubTransport(
            responses={("POST", "/api/datasources"): {"id": 7, "message": "created"}}
        )
        client = datasource_client.GrafanaDatasourceClient(
            base_url="http://grafana.example.com",
            headers={},
            timeout=30,
            verify_ssl=True,
            transport=transport,
        )

        response = client.create_datasource(
            {"name": "Prometheus Main", "type": "prometheus"}
        )

        self.assertEqual(response["id"], 7)
        self.assertEqual(transport.calls[0]["method"], "POST")

    def test_delete_datasource_issues_delete(self):
        transport = StubTransport(
            responses={("DELETE", "/api/datasources/7"): {"message": "deleted"}}
        )
        client = datasource_client.GrafanaDatasourceClient(
            base_url="http://grafana.example.com",
            headers={},
            timeout=30,
            verify_ssl=True,
            transport=transport,
        )

        response = client.delete_datasource(7)

        self.assertEqual(response["message"], "deleted")
        self.assertEqual(transport.calls[0]["path"], "/api/datasources/7")

    def test_with_org_id_clones_client_headers(self):
        client = datasource_client.GrafanaDatasourceClient(
            base_url="http://grafana.example.com",
            headers={"Authorization": "Bearer token"},
            timeout=30,
            verify_ssl=True,
            transport=StubTransport(),
        )

        scoped = client.with_org_id("9")

        self.assertEqual(scoped.headers["X-Grafana-Org-Id"], "9")
        self.assertEqual(client.headers, {"Authorization": "Bearer token"})
