"""Staging helpers for unfinished access-management command surfaces.

These helpers are intentionally kept out of the current parser and dispatch
flow. They provide future parser builders, destructive-action validation, and
exact-match identity resolution for the remaining access commands.
"""

import argparse
from typing import Any, Iterable, Optional
from urllib import parse

from .common import DEFAULT_PAGE_SIZE, GrafanaError


def add_team_delete_cli_args(parser: argparse.ArgumentParser) -> None:
    """Register CLI flags for staged team delete commands."""
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--team-id",
        default=None,
        help="Delete the team identified by this Grafana team id.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the team by exact name before deleting it.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm destructive deletion of the target team.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the delete result as JSON.",
    )


def add_service_account_delete_cli_args(parser: argparse.ArgumentParser) -> None:
    """Register CLI flags for staged service-account delete commands."""
    identity_group = parser.add_mutually_exclusive_group(required=True)
    identity_group.add_argument(
        "--service-account-id",
        default=None,
        help="Delete the service account identified by this Grafana id.",
    )
    identity_group.add_argument(
        "--name",
        default=None,
        help="Resolve the service account by exact name before deleting it.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm destructive deletion of the target service account.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the delete result as JSON.",
    )


def add_service_account_token_delete_cli_args(
    parser: argparse.ArgumentParser,
) -> None:
    """Register CLI flags for staged service-account token delete commands."""
    owner_group = parser.add_mutually_exclusive_group(required=True)
    owner_group.add_argument(
        "--service-account-id",
        default=None,
        help="Resolve the token owner by Grafana service-account id.",
    )
    owner_group.add_argument(
        "--name",
        default=None,
        help="Resolve the token owner by exact service-account name.",
    )
    token_group = parser.add_mutually_exclusive_group(required=True)
    token_group.add_argument(
        "--token-id",
        default=None,
        help="Delete the token identified by this Grafana token id.",
    )
    token_group.add_argument(
        "--token-name",
        default=None,
        help="Resolve the token by exact token name before deleting it.",
    )
    parser.add_argument(
        "--yes",
        action="store_true",
        help="Confirm destructive deletion of the target service-account token.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Render the delete result as JSON.",
    )


def normalize_group_alias_argv(argv: Iterable[str]) -> list[str]:
    """Map a leading `group` resource alias onto the future `team` surface."""

    tokens = list(argv)
    if tokens and tokens[0] == "group":
        tokens[0] = "team"
    return tokens


def validate_destructive_confirmed(args: Any, action_label: str) -> None:
    """Enforce explicit --yes for destructive actions."""
    if not bool(getattr(args, "yes", False)):
        raise GrafanaError("%s requires --yes." % action_label)


def _select_exact_match(
    items: Iterable[dict[str, Any]],
    field_name: str,
    expected_value: str,
    item_label: str,
) -> dict[str, Any]:
    """Return the single exact match or explain none/multiple results."""
    matches = []
    for item in items:
        if str(item.get(field_name) or "") == expected_value:
            matches.append(item)
    if not matches:
        raise GrafanaError(
            "%s not found by %s: %s" % (item_label, field_name, expected_value)
        )
    if len(matches) > 1:
        raise GrafanaError(
            "%s %s matched multiple items: %s"
            % (item_label, field_name, expected_value)
        )
    return dict(matches[0])


def resolve_team_id(
    client: Any,
    team_id: Optional[Any] = None,
    name: Optional[str] = None,
    page_size: int = DEFAULT_PAGE_SIZE,
) -> str:
    """Resolve a team identifier by explicit id or exact name lookup."""
    if team_id is not None and team_id != "":
        return str(team_id)
    if not name:
        raise GrafanaError("Team delete requires either --team-id or --name.")
    team = _select_exact_match(
        client.iter_teams(query=name, page_size=page_size),
        "name",
        name,
        "Team",
    )
    return str(team.get("id") or "")


def resolve_service_account_id(
    client: Any,
    service_account_id: Optional[Any] = None,
    name: Optional[str] = None,
    page_size: int = DEFAULT_PAGE_SIZE,
) -> str:
    """Resolve a service-account identifier by explicit id or exact name lookup."""
    if service_account_id is not None and service_account_id != "":
        return str(service_account_id)
    if not name:
        raise GrafanaError(
            "Service-account delete requires either --service-account-id or --name."
        )
    service_account = _select_exact_match(
        client.list_service_accounts(query=name, page=1, per_page=page_size),
        "name",
        name,
        "Service account",
    )
    return str(service_account.get("id") or "")


def resolve_service_account_token_record(
    token_items: Iterable[dict[str, Any]],
    token_id: Optional[Any] = None,
    token_name: Optional[str] = None,
) -> dict[str, Any]:
    """Resolve one token record by id or exact name, with clear validation."""
    if token_id is not None and token_id != "":
        return _select_exact_match(
            token_items,
            "id",
            str(token_id),
            "Service-account token",
        )
    if token_name:
        return _select_exact_match(
            token_items,
            "name",
            token_name,
            "Service-account token",
        )
    raise GrafanaError(
        "Service-account token delete requires either --token-id or --token-name."
    )


def build_team_delete_request(team_id: Any) -> dict[str, str]:
    """Build a URL-safe DELETE request payload for one team."""
    quoted_team_id = parse.quote(str(team_id), safe="")
    return {
        "method": "DELETE",
        "path": "/api/teams/%s" % quoted_team_id,
    }


def build_service_account_delete_request(
    service_account_id: Any,
) -> dict[str, str]:
    """Build a URL-safe DELETE request payload for one service account."""
    quoted_service_account_id = parse.quote(str(service_account_id), safe="")
    return {
        "method": "DELETE",
        "path": "/api/serviceaccounts/%s" % quoted_service_account_id,
    }


def build_service_account_token_delete_request(
    service_account_id: Any,
    token_id: Any,
) -> dict[str, str]:
    """Build a URL-safe DELETE request payload for one service-account token."""
    quoted_service_account_id = parse.quote(str(service_account_id), safe="")
    quoted_token_id = parse.quote(str(token_id), safe="")
    return {
        "method": "DELETE",
        "path": "/api/serviceaccounts/%s/tokens/%s"
        % (quoted_service_account_id, quoted_token_id),
    }
