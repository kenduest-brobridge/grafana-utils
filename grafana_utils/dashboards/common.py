"""Shared dashboard constants and exceptions."""

DEFAULT_DASHBOARD_TITLE = "dashboard"
DEFAULT_FOLDER_TITLE = "General"
DEFAULT_FOLDER_UID = "general"
DEFAULT_ORG_ID = "1"
DEFAULT_ORG_NAME = "Main Org."
DEFAULT_UNKNOWN_UID = "unknown"

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
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__("Grafana API error %s for %s: %s" % (status_code, url, body))
