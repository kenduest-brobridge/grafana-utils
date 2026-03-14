"""Access-management focused Grafana API client helpers."""

from typing import Any, Dict, List, Optional
from urllib import parse

from ..access.common import GrafanaApiError, GrafanaError
from ..http_transport import (
    HttpTransportApiError,
    HttpTransportError,
    JsonHttpTransport,
    build_json_http_transport,
)


class GrafanaAccessClient:
    """Minimal HTTP wrapper around the Grafana user APIs used by the CLI."""

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

    def list_org_users(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/org/users")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected org user list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def iter_global_users(self, page_size: int) -> List[Dict[str, Any]]:
        users = []
        page = 1
        while True:
            batch = self.request_json(
                "/api/users",
                params={"page": page, "perpage": page_size},
            )
            if not isinstance(batch, list):
                raise GrafanaError("Unexpected global user list response from Grafana.")
            if not batch:
                break
            users.extend(item for item in batch if isinstance(item, dict))
            if len(batch) < page_size:
                break
            page += 1
        return users

    def list_user_teams(self, user_id: Any) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/users/%s/teams" % parse.quote(str(user_id), safe="")
        )
        if not isinstance(data, list):
            raise GrafanaError(
                "Unexpected team list response for Grafana user %s." % user_id
            )
        return [item for item in data if isinstance(item, dict)]

    def get_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/users/%s" % parse.quote(str(user_id), safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected user lookup response for Grafana user %s." % user_id
            )
        return data

    def create_user(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected user create response from Grafana.")
        return data

    def update_user(self, user_id: Any, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/users/%s" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected user update response for Grafana user %s." % user_id
            )
        return data

    def update_user_password(self, user_id: Any, password: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s/password" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload={"password": password},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected password update response for Grafana user %s."
                % user_id
            )
        return data

    def update_user_org_role(self, user_id: Any, role: str) -> Dict[str, Any]:
        data = self.request_json(
            "/api/org/users/%s" % parse.quote(str(user_id), safe=""),
            method="PATCH",
            payload={"role": role},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected org-role update response for Grafana user %s." % user_id
            )
        return data

    def update_user_permissions(
        self,
        user_id: Any,
        is_grafana_admin: bool,
    ) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s/permissions" % parse.quote(str(user_id), safe=""),
            method="PUT",
            payload={"isGrafanaAdmin": is_grafana_admin},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected permission update response for Grafana user %s."
                % user_id
            )
        return data

    def delete_global_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/admin/users/%s" % parse.quote(str(user_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected global delete response for Grafana user %s."
                % user_id
            )
        return data

    def delete_org_user(self, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/org/users/%s" % parse.quote(str(user_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected org delete response for Grafana user %s." % user_id
            )
        return data

    def list_service_accounts(
        self,
        query: Optional[str],
        page: int,
        per_page: int,
    ) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/serviceaccounts/search",
            params={
                "query": query or "",
                "page": page,
                "perpage": per_page,
            },
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account list response from Grafana."
            )
        items = data.get("serviceAccounts", [])
        if not isinstance(items, list):
            raise GrafanaError(
                "Unexpected service-account list response from Grafana."
            )
        return [item for item in items if isinstance(item, dict)]

    def list_teams(
        self,
        query: Optional[str],
        page: int,
        per_page: int,
    ) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/teams/search",
            params={
                "query": query or "",
                "page": page,
                "perpage": per_page,
            },
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected team list response from Grafana.")
        items = data.get("teams", [])
        if not isinstance(items, list):
            raise GrafanaError("Unexpected team list response from Grafana.")
        return [item for item in items if isinstance(item, dict)]

    def iter_teams(
        self,
        query: Optional[str],
        page_size: int,
    ) -> List[Dict[str, Any]]:
        teams = []
        page = 1
        while True:
            batch = self.list_teams(
                query=query,
                page=page,
                per_page=page_size,
            )
            if not batch:
                break
            teams.extend(batch)
            if len(batch) < page_size:
                break
            page += 1
        return teams

    def list_team_members(self, team_id: Any) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe="")
        )
        if not isinstance(data, list):
            raise GrafanaError(
                "Unexpected member list response for Grafana team %s." % team_id
            )
        return [item for item in data if isinstance(item, dict)]

    def get_team(self, team_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s" % parse.quote(str(team_id), safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected team lookup response for Grafana team %s." % team_id
            )
        return data

    def delete_team(self, team_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s" % parse.quote(str(team_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected team delete response for Grafana team %s." % team_id
            )
        return data

    def create_team(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected team create response from Grafana.")
        return data

    def add_team_member(self, team_id: Any, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe=""),
            method="POST",
            payload={"userId": user_id},
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected add-member response for Grafana team %s." % team_id
            )
        return data

    def remove_team_member(self, team_id: Any, user_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members/%s"
            % (
                parse.quote(str(team_id), safe=""),
                parse.quote(str(user_id), safe=""),
            ),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected remove-member response for Grafana team %s." % team_id
            )
        return data

    def update_team_members(self, team_id: Any, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/teams/%s/members" % parse.quote(str(team_id), safe=""),
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected team member update response for Grafana team %s."
                % team_id
            )
        return data

    def create_service_account(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account create response from Grafana."
            )
        return data

    def get_service_account(self, service_account_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts/%s" % parse.quote(str(service_account_id), safe="")
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account lookup response for Grafana service account %s."
                % service_account_id
            )
        return data

    def delete_service_account(self, service_account_id: Any) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts/%s" % parse.quote(str(service_account_id), safe=""),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account delete response for Grafana service account %s."
                % service_account_id
            )
        return data

    def list_service_account_tokens(
        self,
        service_account_id: Any,
    ) -> List[Dict[str, Any]]:
        data = self.request_json(
            "/api/serviceaccounts/%s/tokens"
            % parse.quote(str(service_account_id), safe="")
        )
        if not isinstance(data, list):
            raise GrafanaError(
                "Unexpected service-account token list response for Grafana service account %s."
                % service_account_id
            )
        return [item for item in data if isinstance(item, dict)]

    def create_service_account_token(
        self,
        service_account_id: Any,
        payload: Dict[str, Any],
    ) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts/%s/tokens"
            % parse.quote(str(service_account_id), safe=""),
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account token create response from Grafana."
            )
        return data

    def delete_service_account_token(
        self,
        service_account_id: Any,
        token_id: Any,
    ) -> Dict[str, Any]:
        data = self.request_json(
            "/api/serviceaccounts/%s/tokens/%s"
            % (
                parse.quote(str(service_account_id), safe=""),
                parse.quote(str(token_id), safe=""),
            ),
            method="DELETE",
        )
        if not isinstance(data, dict):
            raise GrafanaError(
                "Unexpected service-account token delete response for Grafana service account %s token %s."
                % (service_account_id, token_id)
            )
        return data
