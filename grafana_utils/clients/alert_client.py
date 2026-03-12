"""Alerting-focused Grafana API client helpers."""

from typing import Any, Dict, List, Optional
from urllib import parse

from ..alerts.common import GrafanaApiError, GrafanaError
from ..http_transport import (
    HttpTransportApiError,
    HttpTransportError,
    JsonHttpTransport,
    build_json_http_transport,
)


class GrafanaAlertClient:
    """Minimal HTTP wrapper around the Grafana alerting provisioning APIs."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
        timeout: int,
        verify_ssl: bool,
        transport: Optional[JsonHttpTransport] = None,
    ) -> None:
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
        """Send one request to Grafana and decode the JSON response body."""
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

    def list_alert_rules(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/alert-rules")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected alert-rule list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def search_dashboards(self, query: str) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/search",
            params={"type": "dash-db", "query": query, "limit": 500},
        )
        if not isinstance(data, list):
            raise GrafanaError("Unexpected dashboard search response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_dashboard(self, uid: str) -> Dict[str, Any]:
        data = self.request_json("/api/dashboards/uid/%s" % parse.quote(uid, safe=""))
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected dashboard payload for UID %s." % uid)
        return data

    def get_alert_rule(self, uid: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/alert-rules/%s" % parse.quote(uid, safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule payload for UID %s." % uid)
        return data

    def create_alert_rule(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/alert-rules",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule create response from Grafana.")
        return data

    def update_alert_rule(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/alert-rules/%s" % parse.quote(uid, safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule update response from Grafana.")
        return data

    def list_contact_points(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/contact-points")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected contact-point list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_contact_point(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/contact-points",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point create response from Grafana.")
        return data

    def update_contact_point(self, uid: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/contact-points/%s" % parse.quote(uid, safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected contact-point update response from Grafana.")
        return data

    def list_mute_timings(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/mute-timings")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected mute-timing list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def create_mute_timing(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/mute-timings",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing create response from Grafana.")
        return data

    def update_mute_timing(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/mute-timings/%s" % parse.quote(name, safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected mute-timing update response from Grafana.")
        return data

    def get_notification_policies(self) -> Dict[str, Any]:
        data = self.request_json("/api/v1/provisioning/policies")
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected notification policy response from Grafana.")
        return data

    def update_notification_policies(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/policies",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected notification policy update response from Grafana."
            )
        return data

    def list_templates(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/templates")
        if data is None:
            return []
        if not isinstance(data, list):
            raise GrafanaError("Unexpected template list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_template(self, name: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/templates/%s" % parse.quote(name, safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected template payload for name %s." % name)
        return data

    def update_template(self, name: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        body = dict(payload)
        body.pop("name", None)
        data = self.request_json(
            "/api/v1/provisioning/templates/%s" % parse.quote(name, safe=""),
            method="PUT",
            payload=body,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected template update response from Grafana.")
        return data
