"""Staging auth helpers for later shared CLI integration.

This module intentionally stays disconnected from the current CLI entrypoints.
It mirrors the token-vs-Basic auth rules already present in the repo, but
packages them in a reusable form for dashboard, alert, and access CLIs.
"""

import base64
import getpass
from typing import Any, Iterable, Optional


class AuthConfigError(RuntimeError):
    """Raised when auth inputs are incomplete or mutually exclusive."""


def format_cli_auth_error_message(message: str) -> str:
    """Map shared auth validation errors to CLI-facing help text."""

    if message == "Choose either token auth or Basic auth, not both.":
        return (
            "Choose either token auth (--token / --api-token / --prompt-token) or Basic auth "
            "(--basic-user with --basic-password / --prompt-password), not both."
        )
    if message == "Choose either an explicit API token or --prompt-token, not both.":
        return "Choose either --token / --api-token or --prompt-token, not both."
    if (
        message
        == "Choose either an explicit Basic auth password or --prompt-password, not both."
    ):
        return "Choose either --basic-password or --prompt-password, not both."
    if (
        message
        == "Basic auth requires both username and password or --prompt-password."
    ):
        return (
            "Basic auth requires both --basic-user and "
            "--basic-password or --prompt-password."
        )
    if message == "--prompt-password requires a Basic auth username.":
        return "--prompt-password requires --basic-user."
    if (
        message
        == "Basic auth environment configuration requires both GRAFANA_USERNAME and GRAFANA_PASSWORD."
    ):
        return (
            "Basic auth requires both --basic-user and "
            "--basic-password or --prompt-password."
        )
    if message == "Authentication required. Provide a token or Basic auth credentials.":
        return (
            "Authentication required. Set --token / --api-token / "
            "--prompt-token / GRAFANA_API_TOKEN or --basic-user and --basic-password / "
            "--prompt-password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
        )
    return message


def _first_present(args: Any, names: Iterable[str]) -> Optional[str]:
    for name in names:
        value = getattr(args, name, None)
        if value:
            return value
    return None


def _env_value(env: Optional[dict[str, str]], name: str) -> Optional[str]:
    if env is None:
        import os

        value = os.environ.get(name)
    else:
        value = env.get(name)
    return value if value else None


def _encode_basic_auth(username: str, password: str) -> str:
    encoded = base64.b64encode(("%s:%s" % (username, password)).encode("utf-8")).decode(
        "ascii"
    )
    return "Basic %s" % encoded


def add_org_id_header(
    headers: dict[str, str],
    org_id: Optional[Any],
) -> dict[str, str]:
    """Return a copy of *headers* with X-Grafana-Org-Id added when present."""

    resolved = dict(headers)
    if org_id is not None and org_id != "":
        resolved["X-Grafana-Org-Id"] = str(org_id)
    return resolved


def resolve_auth_headers(
    token: Optional[str] = None,
    prompt_token: bool = False,
    username: Optional[str] = None,
    password: Optional[str] = None,
    prompt_password: bool = False,
    env: Optional[dict[str, str]] = None,
    token_prompt_reader=None,
    password_prompt_reader=None,
    prompt_reader=None,
) -> tuple[dict[str, str], str]:
    """Resolve Grafana auth headers from explicit flags plus environment."""

    prompt_reader = prompt_reader or getpass.getpass
    token_prompt_reader = token_prompt_reader or prompt_reader
    password_prompt_reader = password_prompt_reader or prompt_reader

    if token and prompt_token:
        raise AuthConfigError(
            "Choose either an explicit API token or --prompt-token, not both."
        )
    if (token or prompt_token) and (username or password or prompt_password):
        raise AuthConfigError("Choose either token auth or Basic auth, not both.")
    if prompt_password and password:
        raise AuthConfigError(
            "Choose either an explicit Basic auth password or --prompt-password, not both."
        )
    if username and not password and not prompt_password:
        raise AuthConfigError(
            "Basic auth requires both username and password or --prompt-password."
        )
    if password and not username:
        raise AuthConfigError(
            "Basic auth requires both username and password or --prompt-password."
        )
    if prompt_password and not username:
        raise AuthConfigError("--prompt-password requires a Basic auth username.")

    if prompt_token:
        token = token_prompt_reader("Grafana API token: ")

    if token:
        return {"Authorization": "Bearer %s" % token}, "token"

    if prompt_password and username:
        password = password_prompt_reader("Grafana Basic auth password: ")

    if username and password:
        return {"Authorization": _encode_basic_auth(username, password)}, "basic"

    env_token = _env_value(env, "GRAFANA_API_TOKEN")
    if env_token:
        return {"Authorization": "Bearer %s" % env_token}, "token"

    env_username = _env_value(env, "GRAFANA_USERNAME")
    env_password = _env_value(env, "GRAFANA_PASSWORD")
    if env_username and env_password:
        return {
            "Authorization": _encode_basic_auth(env_username, env_password)
        }, "basic"
    if env_username or env_password:
        raise AuthConfigError(
            "Basic auth environment configuration requires both GRAFANA_USERNAME "
            "and GRAFANA_PASSWORD."
        )

    raise AuthConfigError(
        "Authentication required. Provide a token or Basic auth credentials."
    )


def resolve_auth_from_namespace(
    args: Any,
    token_attr: str = "api_token",
    prompt_token_attr: str = "prompt_token",
    username_attrs: Optional[Iterable[str]] = None,
    password_attrs: Optional[Iterable[str]] = None,
    prompt_attr: str = "prompt_password",
    org_id_attr: str = "org_id",
    env: Optional[dict[str, str]] = None,
    prompt_reader=None,
    token_prompt_reader=None,
    password_prompt_reader=None,
) -> tuple[dict[str, str], str]:
    """Resolve auth headers from an argparse namespace-like object."""

    if username_attrs is None:
        username_attrs = ("username", "auth_username")
    if password_attrs is None:
        password_attrs = ("password", "auth_password")

    headers, auth_mode = resolve_auth_headers(
        token=getattr(args, token_attr, None),
        prompt_token=bool(getattr(args, prompt_token_attr, False)),
        username=_first_present(args, username_attrs),
        password=_first_present(args, password_attrs),
        prompt_password=bool(getattr(args, prompt_attr, False)),
        env=env,
        prompt_reader=prompt_reader,
        token_prompt_reader=token_prompt_reader,
        password_prompt_reader=password_prompt_reader,
    )
    org_id = getattr(args, org_id_attr, None)
    return add_org_id_header(headers, org_id), auth_mode


def resolve_cli_auth_from_namespace(
    args: Any,
    prompt_reader=None,
    token_attr: str = "api_token",
    prompt_token_attr: str = "prompt_token",
    username_attrs: Optional[Iterable[str]] = None,
    password_attrs: Optional[Iterable[str]] = None,
    prompt_attr: str = "prompt_password",
    org_id_attr: str = "org_id",
    env: Optional[dict[str, str]] = None,
    token_prompt_reader=None,
    password_prompt_reader=None,
) -> tuple[dict[str, str], str]:
    """Resolve auth or raise AuthConfigError with CLI-facing wording."""

    try:
        return resolve_auth_from_namespace(
            args,
            token_attr=token_attr,
            prompt_token_attr=prompt_token_attr,
            username_attrs=username_attrs,
            password_attrs=password_attrs,
            prompt_attr=prompt_attr,
            org_id_attr=org_id_attr,
            env=env,
            prompt_reader=prompt_reader,
            token_prompt_reader=token_prompt_reader,
            password_prompt_reader=password_prompt_reader,
        )
    except AuthConfigError as exc:
        raise AuthConfigError(format_cli_auth_error_message(str(exc)))
