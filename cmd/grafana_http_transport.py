#!/usr/bin/env python3
"""Replaceable JSON HTTP transport adapters for the Grafana CLI tools."""

import json
from typing import Any, Dict, Optional
from urllib import parse


DEFAULT_HTTP_TRANSPORT = "requests"
REQUESTS_TRANSPORT = "requests"
HTTPX_TRANSPORT = "httpx"


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
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        raise NotImplementedError


class BaseJsonHttpTransport(JsonHttpTransport):
    """Shared transport behavior for URL building and JSON response decoding."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
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
        params: Optional[Dict[str, Any]] = None,
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


class RequestsJsonHttpTransport(BaseJsonHttpTransport):
    """JSON transport backed by the requests library."""

    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
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

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        url = self.build_url(path, params)
        try:
            response = self._requests.request(
                method=method,
                url=url,
                headers=self.headers,
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
        headers: Dict[str, str],
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

    def request_json(
        self,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
    ) -> Any:
        url = self.build_url(path, params)
        try:
            response = self._httpx.request(
                method=method,
                url=url,
                headers=self.headers,
                json=payload,
                timeout=self.timeout,
                verify=self.verify_ssl,
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
    headers: Dict[str, str],
    timeout: int,
    verify_ssl: bool,
    transport_name: str = DEFAULT_HTTP_TRANSPORT,
) -> JsonHttpTransport:
    """Build the requested JSON HTTP transport implementation."""
    normalized_name = str(transport_name or DEFAULT_HTTP_TRANSPORT).strip().lower()
    if normalized_name == REQUESTS_TRANSPORT:
        return RequestsJsonHttpTransport(base_url, headers, timeout, verify_ssl)
    if normalized_name == HTTPX_TRANSPORT:
        return HttpxJsonHttpTransport(base_url, headers, timeout, verify_ssl)
    raise HttpTransportError(
        f"Unsupported HTTP transport {transport_name!r}. "
        f"Use {REQUESTS_TRANSPORT!r} or {HTTPX_TRANSPORT!r}."
    )
