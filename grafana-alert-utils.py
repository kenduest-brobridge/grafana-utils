#!/usr/bin/env python3
"""Export or import Grafana alert rules."""

from __future__ import annotations

import argparse
import base64
import copy
import json
import re
import ssl
import sys
from pathlib import Path
from typing import Any
from urllib import error, parse, request


DEFAULT_URL = "https://10.21.104.120"
DEFAULT_TIMEOUT = 30
DEFAULT_OUTPUT_DIR = "alerts"
RAW_EXPORT_SUBDIR = "raw"
TOOL_KIND = "grafana-alert-rule"
TOOL_API_VERSION = 1
SERVER_MANAGED_FIELDS = {"id", "updated", "provenance"}


class GrafanaError(RuntimeError):
    """Raised when Grafana returns an unexpected response."""


class GrafanaApiError(GrafanaError):
    """Raised when Grafana returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__(f"Grafana API error {status_code} for {url}: {body}")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export or import Grafana alert rules."
    )
    parser.add_argument(
        "--url",
        default=DEFAULT_URL,
        help=f"Grafana base URL (default: {DEFAULT_URL})",
    )
    parser.add_argument(
        "--api-token",
        default=None,
        help="Grafana API token. Falls back to GRAFANA_API_TOKEN.",
    )
    parser.add_argument(
        "--username",
        default=None,
        help="Grafana username. Falls back to GRAFANA_USERNAME.",
    )
    parser.add_argument(
        "--password",
        default=None,
        help="Grafana password. Falls back to GRAFANA_PASSWORD.",
    )
    parser.add_argument(
        "--output-dir",
        default=DEFAULT_OUTPUT_DIR,
        help=(
            "Directory to write exported alert rules into. Export writes files "
            f"under {RAW_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Import alert-rule JSON from this directory instead of exporting. "
            f"Point this to the {RAW_EXPORT_SUBDIR}/ export directory explicitly."
        ),
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT}).",
    )
    parser.add_argument(
        "--flat",
        action="store_true",
        help="Write all alert rules into the raw output root instead of folder/group directories.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing alert-rule files if they already exist.",
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update existing alert rules with the same UID instead of failing on import.",
    )
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    return parser.parse_args(argv)


def env_value(name: str) -> str | None:
    import os

    value = os.environ.get(name)
    return value if value else None


def resolve_auth(args: argparse.Namespace) -> dict[str, str]:
    token = args.api_token or env_value("GRAFANA_API_TOKEN")
    if token:
        return {"Authorization": f"Bearer {token}"}

    username = args.username or env_value("GRAFANA_USERNAME")
    password = args.password or env_value("GRAFANA_PASSWORD")
    if username and password:
        encoded = base64.b64encode(f"{username}:{password}".encode("utf-8")).decode(
            "ascii"
        )
        return {"Authorization": f"Basic {encoded}"}

    raise GrafanaError(
        "Authentication required. Set --api-token / GRAFANA_API_TOKEN or "
        "--username and --password / GRAFANA_USERNAME and GRAFANA_PASSWORD."
    )


def build_ssl_context(verify_ssl: bool) -> ssl.SSLContext:
    if verify_ssl:
        return ssl.create_default_context()
    return ssl._create_unverified_context()


def sanitize_path_component(value: str) -> str:
    normalized = re.sub(r"[^\w.\- ]+", "_", value.strip(), flags=re.UNICODE)
    normalized = re.sub(r"\s+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    normalized = normalized.strip("._")
    return normalized or "untitled"


def build_output_path(
    output_dir: Path,
    rule: dict[str, Any],
    flat: bool,
) -> Path:
    folder_uid = sanitize_path_component(rule.get("folderUID") or "unknown-folder")
    rule_group = sanitize_path_component(rule.get("ruleGroup") or "default-group")
    title = sanitize_path_component(rule.get("title") or "alert-rule")
    uid = sanitize_path_component(rule.get("uid") or title or "unknown")
    filename = f"{title}__{uid}.json"
    if flat:
        return output_dir / filename
    return output_dir / folder_uid / rule_group / filename


def write_json(payload: Any, output_path: Path, overwrite: bool) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if output_path.exists() and not overwrite:
        raise GrafanaError(
            f"Refusing to overwrite existing file: {output_path}. Use --overwrite."
        )
    output_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def load_json_file(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise GrafanaError(f"JSON file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise GrafanaError(f"Invalid JSON file: {path}") from exc


def discover_alert_rule_files(import_dir: Path) -> list[Path]:
    if not import_dir.exists():
        raise GrafanaError(f"Import directory does not exist: {import_dir}")
    if not import_dir.is_dir():
        raise GrafanaError(f"Import path is not a directory: {import_dir}")
    if (import_dir / RAW_EXPORT_SUBDIR).is_dir():
        raise GrafanaError(
            f"Import path {import_dir} looks like the export root. "
            f"Point --import-dir at {import_dir / RAW_EXPORT_SUBDIR}."
        )

    files = [
        path
        for path in sorted(import_dir.rglob("*.json"))
        if path.name != "index.json"
    ]
    if not files:
        raise GrafanaError(f"No alert-rule JSON files found in {import_dir}")
    return files


class GrafanaAlertClient:
    def __init__(
        self,
        base_url: str,
        headers: dict[str, str],
        timeout: int,
        verify_ssl: bool,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.headers = {"Accept": "application/json", **headers}
        self.timeout = timeout
        self.ssl_context = build_ssl_context(verify_ssl)

    def request_json(
        self,
        path: str,
        params: dict[str, Any] | None = None,
        method: str = "GET",
        payload: dict[str, Any] | None = None,
    ) -> Any:
        query = ""
        if params:
            query = "?" + parse.urlencode(params)
        url = f"{self.base_url}{path}{query}"
        headers = dict(self.headers)
        data = None
        if payload is not None:
            data = json.dumps(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"
        req = request.Request(url, headers=headers, data=data, method=method)
        try:
            with request.urlopen(
                req,
                timeout=self.timeout,
                context=self.ssl_context,
            ) as response:
                body = response.read().decode("utf-8")
        except error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            raise GrafanaApiError(exc.code, url, body) from exc
        except error.URLError as exc:
            raise GrafanaError(f"Request failed for {url}: {exc.reason}") from exc

        if not body.strip():
            return None

        try:
            return json.loads(body)
        except json.JSONDecodeError as exc:
            raise GrafanaError(f"Invalid JSON response from {url}") from exc

    def list_alert_rules(self) -> list[dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/alert-rules")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected alert-rule list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_alert_rule(self, uid: str) -> dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected alert-rule payload for UID {uid}.")
        return data

    def create_alert_rule(self, payload: dict[str, Any]) -> dict[str, Any]:
        data = self.request_json(
            "/api/v1/provisioning/alert-rules",
            method="POST",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule create response from Grafana.")
        return data

    def update_alert_rule(self, uid: str, payload: dict[str, Any]) -> dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}",
            method="PUT",
            payload=payload,
        )
        if not isinstance(data, dict):
            raise GrafanaError("Unexpected alert-rule update response from Grafana.")
        return data


def strip_server_managed_fields(rule: dict[str, Any]) -> dict[str, Any]:
    normalized = copy.deepcopy(rule)
    for field in SERVER_MANAGED_FIELDS:
        normalized.pop(field, None)
    return normalized


def build_rule_metadata(rule: dict[str, Any]) -> dict[str, str]:
    return {
        "uid": str(rule.get("uid") or ""),
        "title": str(rule.get("title") or ""),
        "folderUID": str(rule.get("folderUID") or ""),
        "ruleGroup": str(rule.get("ruleGroup") or ""),
    }


def build_export_document(rule: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(rule, dict):
        raise GrafanaError("Unexpected alert-rule payload from Grafana.")
    spec = strip_server_managed_fields(rule)
    return {
        "apiVersion": TOOL_API_VERSION,
        "kind": TOOL_KIND,
        "metadata": build_rule_metadata(spec),
        "spec": spec,
    }


def build_import_payload(document: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise GrafanaError("Unexpected alert-rule document. Expected a JSON object.")
    if "groups" in document:
        raise GrafanaError(
            "Grafana provisioning export format is not supported for API import. "
            "Use files exported by grafana-alert-utils.py."
        )

    if document.get("kind") == TOOL_KIND:
        if document.get("apiVersion") != TOOL_API_VERSION:
            raise GrafanaError(
                f"Unsupported alert-rule export version: {document.get('apiVersion')}"
            )
        spec = document.get("spec")
    else:
        spec = document

    if not isinstance(spec, dict):
        raise GrafanaError("Alert-rule import document is missing a valid spec object.")

    payload = strip_server_managed_fields(spec)
    required_fields = ("title", "folderUID", "ruleGroup", "condition", "data")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Alert-rule import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["data"], list):
        raise GrafanaError("Alert-rule field 'data' must be a list.")
    return payload


def export_alert_rules(args: argparse.Namespace) -> int:
    client = build_client(args)
    output_dir = Path(args.output_dir)
    raw_dir = output_dir / RAW_EXPORT_SUBDIR
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_dir.mkdir(parents=True, exist_ok=True)

    rules = client.list_alert_rules()
    if not rules:
        print("No alert rules found.", file=sys.stderr)
        return 0

    index: list[dict[str, str]] = []
    for rule in rules:
        document = build_export_document(rule)
        spec = document["spec"]
        output_path = build_output_path(raw_dir, spec, args.flat)
        write_json(document, output_path, args.overwrite)
        item = {
            "uid": str(spec.get("uid") or ""),
            "title": str(spec.get("title") or ""),
            "folderUID": str(spec.get("folderUID") or ""),
            "ruleGroup": str(spec.get("ruleGroup") or ""),
            "path": str(output_path),
            "format": TOOL_KIND,
        }
        index.append(item)
        print(f"Exported alert rule {item['uid'] or 'unknown'} -> {output_path}")

    raw_index_path = raw_dir / "index.json"
    write_json(index, raw_index_path, overwrite=True)
    index_path = output_dir / "index.json"
    write_json(index, index_path, overwrite=True)
    print(
        f"Exported {len(index)} alert rules. Raw index: {raw_index_path} "
        f"Root index: {index_path}"
    )
    return 0


def import_alert_rules(args: argparse.Namespace) -> int:
    client = build_client(args)
    import_dir = Path(args.import_dir)
    rule_files = discover_alert_rule_files(import_dir)

    for rule_file in rule_files:
        document = load_json_file(rule_file)
        payload = build_import_payload(document)
        uid = str(payload.get("uid") or "")
        if args.replace_existing and uid:
            try:
                client.get_alert_rule(uid)
            except GrafanaApiError as exc:
                if exc.status_code != 404:
                    raise
                result = client.create_alert_rule(payload)
                action = "created"
            else:
                result = client.update_alert_rule(uid, payload)
                action = "updated"
        else:
            result = client.create_alert_rule(payload)
            action = "created"

        result_uid = str(result.get("uid") or uid or "unknown")
        print(f"Imported {rule_file} -> uid={result_uid} action={action}")

    print(f"Imported {len(rule_files)} alert-rule files from {import_dir}")
    return 0


def build_client(args: argparse.Namespace) -> GrafanaAlertClient:
    headers = resolve_auth(args)
    return GrafanaAlertClient(
        base_url=args.url,
        headers=headers,
        timeout=args.timeout,
        verify_ssl=args.verify_ssl,
    )


def main() -> int:
    args = parse_args()
    try:
        if args.import_dir:
            return import_alert_rules(args)
        return export_alert_rules(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
