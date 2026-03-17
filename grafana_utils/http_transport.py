#!/usr/bin/env python3
"""Replaceable JSON HTTP transport adapters for the Grafana CLI tools."""

import json
import ssl
from typing import Any, Optional
from urllib import parse


AUTO_HTTP_TRANSPORT = "auto"
DEFAULT_HTTP_TRANSPORT = AUTO_HTTP_TRANSPORT
REQUESTS_TRANSPORT = "requests"
HTTPX_TRANSPORT = "httpx"


class HttpTransportError(RuntimeError):
    """Raised when the selected HTTP transport cannot complete a request."""


class HttpTransportApiError(HttpTransportError):
    """Raised when the remote server returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        # Purpose: initialize transport error state.
        # Args: status_code, url, body
        # Returns: None.

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
        # Purpose: send one JSON request and decode response.
        # Args: path, params, method, payload
        # Returns: Any decoded JSON payload.

        raise NotImplementedError


class BaseJsonHttpTransport(JsonHttpTransport):
    """Shared transport behavior for URL building and JSON response decoding."""

    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
        ca_cert: Optional[str] = None,
    ) -> None:
        # Purpose: initialize transport base configuration.
        # Args: base_url, headers, timeout, verify_ssl, ca_cert
        # Returns: None.

        self.base_url = base_url.rstrip("/")
        self.headers = dict(headers)
        self.timeout = timeout
        self.verify_ssl = verify_ssl
        self.ca_cert = ca_cert

    def verify_config(self) -> Any:
        # Purpose: resolve SSL verification argument.
        # Returns: certificate path, ssl.SSLContext, or boolean.

        if self.ca_cert:
            return self.ca_cert
        return self.verify_ssl

    def build_url(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
    ) -> str:
        # Purpose: build an endpoint URL with optional query string.
        # Args: path, params
        # Returns: full URL string.

        query = ""
        if params:
            query = "?" + parse.urlencode(params)
        return f"{self.base_url}{path}{query}"

    def decode_json_response(self, body: str, url: str) -> Any:
        # Purpose: parse JSON response body.
        # Args: body, url
        # Returns: parsed JSON value or None for empty body.

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
        ca_cert: Optional[str] = None,
    ) -> None:
        # Purpose: store transport state for lazy requests import.
        # Args: base_url, headers, timeout, verify_ssl, ca_cert
        # Returns: None.

        super().__init__(base_url, headers, timeout, verify_ssl, ca_cert=ca_cert)
        self._requests = None
        self._session = None

    def _get_requests_module(self) -> Any:
        # Purpose: lazy-import requests on first request.
        # Returns: loaded requests module.

        if self._requests is None:
            try:
                import requests
            except ImportError as exc:
                raise HttpTransportError(
                    "The requests transport is unavailable because requests is not installed."
                ) from exc
            self._requests = requests
            self._session = requests.Session()
            self._session.headers.update(self.headers)
        return self._requests

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
        # Purpose: request JSON using requests and decode the result.
        # Args: path, params, method, payload
        # Returns: decoded JSON value.

        url = self.build_url(path, params)
        requests_module = self._get_requests_module()
        try:
            response = self._session.request(
                method=method,
                url=url,
                json=payload,
                timeout=self.timeout,
                verify=self.verify_config(),
            )
        except requests_module.RequestException as exc:
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
        ca_cert: Optional[str] = None,
    ) -> None:
        # Purpose: initialize and configure the httpx client.
        # Args: base_url, headers, timeout, verify_ssl, ca_cert
        # Returns: None.

        super().__init__(base_url, headers, timeout, verify_ssl, ca_cert=ca_cert)
        try:
            import httpx
        except ImportError as exc:
            raise HttpTransportError(
                "The httpx transport is unavailable because httpx is not installed."
            ) from exc
        self._httpx = httpx
        verify = self.verify_config()
        if self.ca_cert:
            context = ssl.create_default_context(cafile=self.ca_cert)
            verify = context
        self._client = httpx.Client(
            headers=self.headers,
            timeout=self.timeout,
            verify=verify,
            http2=http2_is_available(),
        )

    def request_json(
        self,
        path: str,
        params: Optional[dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[dict[str, Any]] = None,
    ) -> Any:
        # Purpose: request JSON using httpx and decode the result.
        # Args: path, params, method, payload
        # Returns: decoded JSON value.

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
    ca_cert: Optional[str] = None,
    transport_name: str = DEFAULT_HTTP_TRANSPORT,
) -> JsonHttpTransport:
    """Build the requested JSON HTTP transport implementation."""
    # Purpose: choose implementation based on transport_name and installed deps.

    normalized_name = str(transport_name or DEFAULT_HTTP_TRANSPORT).strip().lower()
    if normalized_name == AUTO_HTTP_TRANSPORT:
        if httpx_is_available() and http2_is_available():
            return HttpxJsonHttpTransport(
                base_url,
                headers,
                timeout,
                verify_ssl,
                ca_cert=ca_cert,
            )
        return RequestsJsonHttpTransport(
            base_url,
            headers,
            timeout,
            verify_ssl,
            ca_cert=ca_cert,
        )
    if normalized_name == REQUESTS_TRANSPORT:
        return RequestsJsonHttpTransport(
            base_url,
            headers,
            timeout,
            verify_ssl,
            ca_cert=ca_cert,
        )
    if normalized_name == HTTPX_TRANSPORT:
        return HttpxJsonHttpTransport(
            base_url,
            headers,
            timeout,
            verify_ssl,
            ca_cert=ca_cert,
        )
    raise HttpTransportError(
        f"Unsupported HTTP transport {transport_name!r}. "
        f"Use {AUTO_HTTP_TRANSPORT!r}, {REQUESTS_TRANSPORT!r}, or {HTTPX_TRANSPORT!r}."
    )
