"""Datasource-focused Grafana API client helpers.

Purpose:
- Isolate datasource-specific live CRUD helpers from the dashboard client so
  future datasource mutation wiring can depend on a more explicit API surface.

Caveats:
- This client is intentionally not wired into the CLI yet.
- It currently focuses on list/add/delete flows needed by the unwired live
  mutation helpers.
"""

from typing import Any, Optional
from urllib import parse

from ..dashboards.common import GrafanaApiError, GrafanaError
from ..http_transport import (
    DEFAULT_HTTP_TRANSPORT,
    HttpTransportApiError,
    HttpTransportError,
    JsonHttpTransport,
    build_json_http_transport,
)


class GrafanaDatasourceClient:
    """Minimal HTTP wrapper around the Grafana datasource APIs."""

    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
        transport_name: str = DEFAULT_HTTP_TRANSPORT,
        transport: Optional[JsonHttpTransport] = None,
    ) -> None:
        self.base_url = base_url
        self.headers = dict(headers)
        self.timeout = timeout
        self.verify_ssl = verify_ssl
        self.transport_name = transport_name
        self.transport = transport or build_json_http_transport(
            base_url=base_url,
            headers={"Accept": "application/json", **headers},
            timeout=timeout,
            verify_ssl=verify_ssl,
            transport_name=transport_name,
        )

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
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

    def list_datasources(self) -> list[dict[str, Any]]:
        data = self.request_json("/api/datasources")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected datasource list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def fetch_datasource_by_uid_if_exists(self, uid: str) -> Optional[dict[str, Any]]:
        try:
            data = self.request_json(
                "/api/datasources/uid/%s" % parse.quote(str(uid), safe="")
            )
        except GrafanaApiError as exc:
            if exc.status_code == 404:
                return None
            raise
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected datasource payload for UID %s." % uid)
        return data

    def create_datasource(self, payload: dict[str, Any]) -> dict[str, Any]:
        data = self.request_json(
            "/api/datasources",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected datasource create response from Grafana.")
        return data

    def delete_datasource(self, datasource_id: Any) -> dict[str, Any]:
        data = self.request_json(
            "/api/datasources/%s" % parse.quote(str(datasource_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected datasource delete response for Grafana datasource %s."
                % datasource_id
            )
        return data

    def with_org_id(self, org_id: str) -> "GrafanaDatasourceClient":
        headers = dict(self.headers)
        headers["X-Grafana-Org-Id"] = str(org_id)
        return GrafanaDatasourceClient(
            base_url=self.base_url,
            headers=headers,
            timeout=self.timeout,
            verify_ssl=self.verify_ssl,
            transport_name=self.transport_name,
        )
