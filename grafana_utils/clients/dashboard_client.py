"""Dashboard-focused Grafana API client helpers."""

from typing import Any, Dict, List, Optional, Set
from urllib import parse

from ..dashboards.common import GrafanaApiError, GrafanaError
from ..http_transport import (
    HttpTransportApiError,
    HttpTransportError,
    JsonHttpTransport,
    build_json_http_transport,
)


class GrafanaClient:
    """Minimal HTTP wrapper around the Grafana dashboard APIs used by this script."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
        timeout: int,
        verify_ssl: bool,
        transport: Optional[JsonHttpTransport] = None,
    ) -> None:
        self.base_url = base_url
        self.headers = dict(headers)
        self.timeout = timeout
        self.verify_ssl = verify_ssl
        self.transport = transport or build_json_http_transport(
            base_url=base_url,
            headers={"Accept": "application/json", **headers},
            timeout=timeout,
            verify_ssl=verify_ssl,
        )

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        """Send one request to Grafana and decode the JSON response."""
        try:
            return self.transport.request_json(
                path=path,
                params=params,
                method=method,
                payload=payload,
            )
        except HttpTransportApiError as exc:
            raise GrafanaApiError(exc.status_code, exc.url, exc.body) from exc
        except HttpTransportError as exc:
            raise GrafanaError(str(exc)) from exc

    def iter_dashboard_summaries(self, page_size: int) -> List[Dict[str, Any]]:
        """List dashboards through Grafana search pagination and deduplicate by UID."""
        dashboards: List[Dict[str, Any]] = []
        seen_uids: Set[str] = set()
        page = 1

        while True:
            batch = self.request_json(
                "/api/search",
                params={"type": "dash-db", "limit": page_size, "page": page},
            )
            if not isinstance(batch, list):
                raise GrafanaError("Unexpected search response from Grafana.")
            if not batch:
                break

            for item in batch:
                uid = item.get("uid")
                if not uid or uid in seen_uids:
                    continue
                seen_uids.add(uid)
                dashboards.append(item)

            if len(batch) < page_size:
                break
            page += 1

        return dashboards

    def fetch_folder_if_exists(self, uid: str) -> Optional[Dict[str, Any]]:
        """Fetch one folder payload or return None when the folder UID is missing."""
        try:
            data = self.request_json(f"/api/folders/{parse.quote(uid, safe='')}")
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected folder payload for UID %s." % uid)
        return data

    def fetch_dashboard(self, uid: str) -> Dict[str, Any]:
        """Fetch the full dashboard wrapper for a single Grafana UID."""
        data = self.fetch_dashboard_if_exists(uid)
        if data is None:
            raise GrafanaApiError(
                404,
                "/api/dashboards/uid/%s" % parse.quote(uid, safe=""),
                "Dashboard not found",
            )
        if not isinstance(data, dict) or "dashboard" not in data:
            raise GrafanaError("Unexpected dashboard payload for UID %s." % uid)
        return data

    def fetch_dashboard_if_exists(self, uid: str) -> Optional[Dict[str, Any]]:
        """Fetch the full dashboard wrapper or return None when the UID is missing."""
        data = None
        try:
            data = self.request_json("/api/dashboards/uid/%s" % parse.quote(uid, safe=""))
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        if not isinstance(data, dict) or "dashboard" not in data:
            raise GrafanaError("Unexpected dashboard payload for UID %s." % uid)
        return data

    def import_dashboard(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Create or update a dashboard through POST /api/dashboards/db."""
        data = self.request_json(
            "/api/dashboards/db",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected dashboard import response from Grafana.")
        return data

    def list_datasources(self) -> List[Dict[str, Any]]:
        """List datasource objects used when building prompt-style exports."""
        data = self.request_json("/api/datasources")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected datasource list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def fetch_current_org(self) -> Dict[str, Any]:
        """Fetch the current Grafana organization for the authenticated caller."""
        data = self.request_json("/api/org")
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected current org response from Grafana.")
        return data

    def list_orgs(self) -> List[Dict[str, Any]]:
        """List Grafana organizations visible to the current authenticated caller."""
        data = self.request_json("/api/orgs")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected org list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def with_org_id(self, org_id: str) -> "GrafanaClient":
        """Return a new client scoped to one explicit Grafana organization."""
        headers = dict(self.headers)
        headers["X-Grafana-Org-Id"] = str(org_id)
        return GrafanaClient(
            base_url=self.base_url,
            headers=headers,
            timeout=self.timeout,
            verify_ssl=self.verify_ssl,
        )
