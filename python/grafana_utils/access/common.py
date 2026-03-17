"""Shared access-management constants and exceptions."""

DEFAULT_PAGE_SIZE = 100
OUTPUT_FIELDS = [
    "id",
    "login",
    "email",
    "name",
    "orgRole",
    "grafanaAdmin",
    "scope",
    "teams",
]
TEAM_OUTPUT_FIELDS = [
    "id",
    "name",
    "email",
    "memberCount",
    "members",
]
SERVICE_ACCOUNT_OUTPUT_FIELDS = [
    "id",
    "name",
    "login",
    "role",
    "disabled",
    "tokens",
    "orgId",
]
SERVICE_ACCOUNT_TOKEN_OUTPUT_FIELDS = [
    "serviceAccountId",
    "name",
    "secondsToLive",
    "key",
]


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
