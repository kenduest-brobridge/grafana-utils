"""Shared dashboard constants and exceptions."""

BUILTIN_DATASOURCE_TYPES = {"__expr__", "grafana"}
BUILTIN_DATASOURCE_NAMES = {
    "-- Dashboard --",
    "-- Grafana --",
    "-- Mixed --",
    "grafana",
    "expr",
    "__expr__",
}
DATASOURCE_TYPE_ALIASES = {
    "prom": "prometheus",
    "prometheus": "prometheus",
    "loki": "loki",
    "elastic": "elasticsearch",
    "elasticsearch": "elasticsearch",
    "opensearch": "grafana-opensearch-datasource",
    "mysql": "mysql",
    "postgres": "postgres",
    "postgresql": "postgres",
    "mssql": "mssql",
    "influxdb": "influxdb",
    "tempo": "tempo",
    "jaeger": "jaeger",
    "zipkin": "zipkin",
    "cloudwatch": "cloudwatch",
}


class GrafanaError(RuntimeError):
    """Raised when Grafana returns an unexpected response."""


class GrafanaApiError(GrafanaError):
    """Raised when Grafana returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__("Grafana API error %s for %s: %s" % (status_code, url, body))
