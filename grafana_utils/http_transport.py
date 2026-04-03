#!/usr/bin/env python3
"""Replaceable JSON HTTP transport adapters for the Grafana CLI tools."""

import json
from typing import Any, Optional
from urllib import parse


AUTO_HTTP_TRANSPORT = "auto"
DEFAULT_HTTP_TRANSPORT = AUTO_HTTP_TRANSPORT
REQUESTS_TRANSPORT = "requests"
HTTPX_TRANSPORT = "httpx"
HTTP_TRANSPORT_CHOICES = (
    AUTO_HTTP_TRANSPORT,
    REQUESTS_TRANSPORT,
    HTTPX_TRANSPORT,
)


class HttpTransportError(RuntimeError):
    """Raised when the selected HTTP transport cannot complete a request."""


class HttpTransportApiError(HttpTransportError):
    """Raised when the remote server returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__(f"HTTP error {status_code} for {url}: {body}")


class JsonHttpTransport:
    """Interface for sending one JSON request and decoding the JSON response."""

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
        raise NotImplementedError


class BaseJsonHttpTransport(JsonHttpTransport):
    """Shared transport behavior for URL building and JSON response decoding."""

    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.headers = dict(headers)
        self.timeout = timeout
        self.verify_ssl = verify_ssl

    def build_url(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
    ) -> str:
        query = ""
        if params:
            query = "?" + parse.urlencode(params)
        return f"{self.base_url}{path}{query}"

    def decode_json_response(self, body: str, url: str) -> Any:
        if not body.strip():
            return None
        try:
            return json.loads(body)
        except json.JSONDecodeError as exc:
            raise HttpTransportError(f"Invalid JSON response from {url}") from exc


def http2_is_available() -> bool:
    """Return True when the runtime can actually negotiate HTTP/2 via httpx."""
    try:
        import h2  # noqa: F401
    except ImportError:
        return False
    return True


def httpx_is_available() -> bool:
    """Return True when the httpx transport can be imported."""
    try:
        import httpx  # noqa: F401
    except ImportError:
        return False
    return True


class RequestsJsonHttpTransport(BaseJsonHttpTransport):
    """JSON transport backed by the requests library."""

    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
    ) -> None:
        super().__init__(base_url, headers, timeout, verify_ssl)
        try:
            import requests
        except ImportError as exc:
            raise HttpTransportError(
                "The requests transport is unavailable because requests is not installed."
            ) from exc
        self._requests = requests
        self._session = requests.Session()
        self._session.headers.update(self.headers)

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
        url = self.build_url(path, params)
        try:
            response = self._session.request(
                method=method,
                url=url,
                json=payload,
                timeout=self.timeout,
                verify=self.verify_ssl,
            )
        except self._requests.RequestException as exc:
            raise HttpTransportError(f"Request failed for {url}: {exc}") from exc

        if response.status_code >= 400:
            raise HttpTransportApiError(
                response.status_code,
                url,
                response.text,
            )
        return self.decode_json_response(response.text, url)


class HttpxJsonHttpTransport(BaseJsonHttpTransport):
    """JSON transport backed by the httpx library."""

    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
    ) -> None:
        super().__init__(base_url, headers, timeout, verify_ssl)
        try:
            import httpx
        except ImportError as exc:
            raise HttpTransportError(
                "The httpx transport is unavailable because httpx is not installed."
            ) from exc
        self._httpx = httpx
        self._client = httpx.Client(
            headers=self.headers,
            timeout=self.timeout,
            verify=self.verify_ssl,
            http2=http2_is_available(),
        )

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
        url = self.build_url(path, params)
        try:
            response = self._client.request(
                method=method,
                url=url,
                json=payload,
            )
        except self._httpx.RequestError as exc:
            raise HttpTransportError(f"Request failed for {url}: {exc}") from exc

        if response.status_code >= 400:
            raise HttpTransportApiError(
                response.status_code,
                url,
                response.text,
            )
        return self.decode_json_response(response.text, url)


def build_json_http_transport(
    base_url: str,
    headers: dict[str, str],
    timeout: int,
    verify_ssl: bool,
    transport_name: str = DEFAULT_HTTP_TRANSPORT,
) -> JsonHttpTransport:
    """Build the requested JSON HTTP transport implementation."""
    normalized_name = str(transport_name or DEFAULT_HTTP_TRANSPORT).strip().lower()
    if normalized_name == AUTO_HTTP_TRANSPORT:
        if httpx_is_available() and http2_is_available():
            return HttpxJsonHttpTransport(base_url, headers, timeout, verify_ssl)
        return RequestsJsonHttpTransport(base_url, headers, timeout, verify_ssl)
    if normalized_name == REQUESTS_TRANSPORT:
        return RequestsJsonHttpTransport(base_url, headers, timeout, verify_ssl)
    if normalized_name == HTTPX_TRANSPORT:
        return HttpxJsonHttpTransport(base_url, headers, timeout, verify_ssl)
    raise HttpTransportError(
        f"Unsupported HTTP transport {transport_name!r}. "
        f"Use {AUTO_HTTP_TRANSPORT!r}, {REQUESTS_TRANSPORT!r}, or {HTTPX_TRANSPORT!r}."
    )
