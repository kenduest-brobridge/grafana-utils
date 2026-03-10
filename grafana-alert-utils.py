#!/usr/bin/env python3
"""Export or import Grafana alerting resources."""

import argparse
import base64
import copy
import json
import re
import ssl
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple
from urllib import error, parse, request


DEFAULT_URL = "http://127.0.0.1:3000"
DEFAULT_TIMEOUT = 30
DEFAULT_OUTPUT_DIR = "alerts"
RAW_EXPORT_SUBDIR = "raw"
RULES_SUBDIR = "rules"
CONTACT_POINTS_SUBDIR = "contact-points"
MUTE_TIMINGS_SUBDIR = "mute-timings"
POLICIES_SUBDIR = "policies"

RULE_KIND = "grafana-alert-rule"
CONTACT_POINT_KIND = "grafana-contact-point"
MUTE_TIMING_KIND = "grafana-mute-timing"
POLICIES_KIND = "grafana-notification-policies"
TOOL_API_VERSION = 1

RESOURCE_SUBDIR_BY_KIND = {
    RULE_KIND: RULES_SUBDIR,
    CONTACT_POINT_KIND: CONTACT_POINTS_SUBDIR,
    MUTE_TIMING_KIND: MUTE_TIMINGS_SUBDIR,
    POLICIES_KIND: POLICIES_SUBDIR,
}
SERVER_MANAGED_FIELDS_BY_KIND = {
    RULE_KIND: {"id", "updated", "provenance"},
    CONTACT_POINT_KIND: {"provenance"},
    MUTE_TIMING_KIND: {"version", "provenance"},
    POLICIES_KIND: {"provenance"},
}


class GrafanaError(RuntimeError):
    """Raised when Grafana returns an unexpected response."""


class GrafanaApiError(GrafanaError):
    """Raised when Grafana returns an HTTP error response."""

    def __init__(self, status_code: int, url: str, body: str) -> None:
        self.status_code = status_code
        self.url = url
        self.body = body
        super().__init__(f"Grafana API error {status_code} for {url}: {body}")


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Export or import Grafana alerting resources."
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
            "Directory to write exported alerting resources into. Export writes files "
            f"under {RAW_EXPORT_SUBDIR}/."
        ),
    )
    parser.add_argument(
        "--import-dir",
        default=None,
        help=(
            "Import alerting resource JSON from this directory instead of exporting. "
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
        help=(
            "Write rule, contact-point, and mute-timing files directly into their "
            "resource directories instead of nested folder/group directories."
        ),
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing exported files if they already exist.",
    )
    parser.add_argument(
        "--replace-existing",
        action="store_true",
        help="Update existing resources with the same identity instead of failing on import.",
    )
    parser.add_argument(
        "--verify-ssl",
        action="store_true",
        help="Enable TLS certificate verification. Verification is disabled by default.",
    )
    return parser.parse_args(argv)


def env_value(name: str) -> Optional[str]:
    import os

    value = os.environ.get(name)
    return value if value else None


def resolve_auth(args: argparse.Namespace) -> Dict[str, str]:
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


def build_resource_dirs(raw_dir: Path) -> Dict[str, Path]:
    return {
        kind: raw_dir / subdir for kind, subdir in RESOURCE_SUBDIR_BY_KIND.items()
    }


def build_rule_output_path(output_dir: Path, rule: Dict[str, Any], flat: bool) -> Path:
    folder_uid = sanitize_path_component(rule.get("folderUID") or "unknown-folder")
    rule_group = sanitize_path_component(rule.get("ruleGroup") or "default-group")
    title = sanitize_path_component(rule.get("title") or "alert-rule")
    uid = sanitize_path_component(rule.get("uid") or title or "unknown")
    filename = f"{title}__{uid}.json"
    if flat:
        return output_dir / filename
    return output_dir / folder_uid / rule_group / filename


def build_contact_point_output_path(
    output_dir: Path,
    contact_point: Dict[str, Any],
    flat: bool,
) -> Path:
    name = sanitize_path_component(contact_point.get("name") or "contact-point")
    uid = sanitize_path_component(contact_point.get("uid") or name or "unknown")
    filename = f"{name}__{uid}.json"
    return output_dir / filename if flat else output_dir / name / filename


def build_mute_timing_output_path(
    output_dir: Path,
    mute_timing: Dict[str, Any],
    flat: bool,
) -> Path:
    name = sanitize_path_component(mute_timing.get("name") or "mute-timing")
    filename = f"{name}.json"
    return output_dir / filename if flat else output_dir / name / filename


def build_policies_output_path(output_dir: Path) -> Path:
    return output_dir / "notification-policies.json"


def discover_alert_resource_files(import_dir: Path) -> List[Path]:
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
        raise GrafanaError(f"No alerting resource JSON files found in {import_dir}")
    return files


class GrafanaAlertClient:
    def __init__(
        self,
        base_url: str,
        headers: Dict[str, str],
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
        params: Optional[Dict[str, Any]] = None,
        method: str = "GET",
        payload: Optional[Dict[str, Any]] = None,
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

    def list_alert_rules(self) -> List[Dict[str, Any]]:
        data = self.request_json("/api/v1/provisioning/alert-rules")
        if not isinstance(data, list):
            raise GrafanaError("Unexpected alert-rule list response from Grafana.")
        return [item for item in data if isinstance(item, dict)]

    def get_alert_rule(self, uid: str) -> Dict[str, Any]:
        data = self.request_json(
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}"
        )
        if not isinstance(data, dict):
            raise GrafanaError(f"Unexpected alert-rule payload for UID {uid}.")
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
            f"/api/v1/provisioning/alert-rules/{parse.quote(uid, safe='')}",
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
            f"/api/v1/provisioning/contact-points/{parse.quote(uid, safe='')}",
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
            f"/api/v1/provisioning/mute-timings/{parse.quote(name, safe='')}",
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


def strip_server_managed_fields(kind: str, payload: Dict[str, Any]) -> Dict[str, Any]:
    normalized = copy.deepcopy(payload)
    for field in SERVER_MANAGED_FIELDS_BY_KIND.get(kind, set()):
        normalized.pop(field, None)
    return normalized


def build_rule_metadata(rule: Dict[str, Any]) -> Dict[str, str]:
    return {
        "uid": str(rule.get("uid") or ""),
        "title": str(rule.get("title") or ""),
        "folderUID": str(rule.get("folderUID") or ""),
        "ruleGroup": str(rule.get("ruleGroup") or ""),
    }


def build_contact_point_metadata(contact_point: Dict[str, Any]) -> Dict[str, str]:
    return {
        "uid": str(contact_point.get("uid") or ""),
        "name": str(contact_point.get("name") or ""),
        "type": str(contact_point.get("type") or ""),
    }


def build_mute_timing_metadata(mute_timing: Dict[str, Any]) -> Dict[str, str]:
    return {"name": str(mute_timing.get("name") or "")}


def build_policies_metadata(policies: Dict[str, Any]) -> Dict[str, str]:
    return {"receiver": str(policies.get("receiver") or "")}


def build_tool_document(kind: str, spec: Dict[str, Any]) -> Dict[str, Any]:
    metadata_builders = {
        RULE_KIND: build_rule_metadata,
        CONTACT_POINT_KIND: build_contact_point_metadata,
        MUTE_TIMING_KIND: build_mute_timing_metadata,
        POLICIES_KIND: build_policies_metadata,
    }
    metadata_builder = metadata_builders[kind]
    return {
        "apiVersion": TOOL_API_VERSION,
        "kind": kind,
        "metadata": metadata_builder(spec),
        "spec": spec,
    }


def build_rule_export_document(rule: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(rule, dict):
        raise GrafanaError("Unexpected alert-rule payload from Grafana.")
    return build_tool_document(RULE_KIND, strip_server_managed_fields(RULE_KIND, rule))


def build_contact_point_export_document(contact_point: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(contact_point, dict):
        raise GrafanaError("Unexpected contact-point payload from Grafana.")
    return build_tool_document(
        CONTACT_POINT_KIND,
        strip_server_managed_fields(CONTACT_POINT_KIND, contact_point),
    )


def build_mute_timing_export_document(mute_timing: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(mute_timing, dict):
        raise GrafanaError("Unexpected mute-timing payload from Grafana.")
    return build_tool_document(
        MUTE_TIMING_KIND,
        strip_server_managed_fields(MUTE_TIMING_KIND, mute_timing),
    )


def build_policies_export_document(policies: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(policies, dict):
        raise GrafanaError("Unexpected notification policy payload from Grafana.")
    return build_tool_document(
        POLICIES_KIND,
        strip_server_managed_fields(POLICIES_KIND, policies),
    )


def reject_provisioning_export(document: Dict[str, Any]) -> None:
    if "groups" in document or "contactPoints" in document or "policies" in document:
        raise GrafanaError(
            "Grafana provisioning export format is not supported for API import. "
            "Use files exported by grafana-alert-utils.py."
        )


def detect_document_kind(document: Dict[str, Any]) -> str:
    kind = document.get("kind")
    if kind in RESOURCE_SUBDIR_BY_KIND:
        return str(kind)
    if "condition" in document and "data" in document:
        return RULE_KIND
    if "time_intervals" in document and "name" in document:
        return MUTE_TIMING_KIND
    if "type" in document and "settings" in document and "name" in document:
        return CONTACT_POINT_KIND
    if "receiver" in document or "routes" in document or "group_by" in document:
        return POLICIES_KIND
    raise GrafanaError("Cannot determine alerting resource kind from import document.")


def extract_tool_spec(document: Dict[str, Any], expected_kind: str) -> Dict[str, Any]:
    if document.get("kind") == expected_kind:
        if document.get("apiVersion") != TOOL_API_VERSION:
            raise GrafanaError(
                f"Unsupported {expected_kind} export version: {document.get('apiVersion')}"
            )
        spec = document.get("spec")
    else:
        spec = document
    if not isinstance(spec, dict):
        raise GrafanaError(f"{expected_kind} import document is missing a valid spec object.")
    return spec


def build_rule_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        RULE_KIND, extract_tool_spec(document, RULE_KIND)
    )
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


def build_contact_point_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        CONTACT_POINT_KIND, extract_tool_spec(document, CONTACT_POINT_KIND)
    )
    required_fields = ("name", "type", "settings")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Contact-point import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["settings"], dict):
        raise GrafanaError("Contact-point field 'settings' must be an object.")
    return payload


def build_mute_timing_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        MUTE_TIMING_KIND, extract_tool_spec(document, MUTE_TIMING_KIND)
    )
    required_fields = ("name", "time_intervals")
    missing = [field for field in required_fields if field not in payload]
    if missing:
        raise GrafanaError(
            "Mute-timing import document is missing required fields: "
            + ", ".join(missing)
        )
    if not isinstance(payload["time_intervals"], list):
        raise GrafanaError("Mute-timing field 'time_intervals' must be a list.")
    return payload


def build_policies_import_payload(document: Dict[str, Any]) -> Dict[str, Any]:
    reject_provisioning_export(document)
    payload = strip_server_managed_fields(
        POLICIES_KIND, extract_tool_spec(document, POLICIES_KIND)
    )
    if not isinstance(payload, dict):
        raise GrafanaError("Notification policies import document must be an object.")
    return payload


def build_import_operation(document: Dict[str, Any]) -> Tuple[str, Dict[str, Any]]:
    if not isinstance(document, dict):
        raise GrafanaError("Unexpected alerting resource document. Expected a JSON object.")
    kind = detect_document_kind(document)
    builders = {
        RULE_KIND: build_rule_import_payload,
        CONTACT_POINT_KIND: build_contact_point_import_payload,
        MUTE_TIMING_KIND: build_mute_timing_import_payload,
        POLICIES_KIND: build_policies_import_payload,
    }
    return kind, builders[kind](document)


def export_alerting_resources(args: argparse.Namespace) -> int:
    client = build_client(args)
    output_dir = Path(args.output_dir)
    raw_dir = output_dir / RAW_EXPORT_SUBDIR
    output_dir.mkdir(parents=True, exist_ok=True)
    raw_dir.mkdir(parents=True, exist_ok=True)

    resource_dirs = build_resource_dirs(raw_dir)
    for path in resource_dirs.values():
        path.mkdir(parents=True, exist_ok=True)

    rules = client.list_alert_rules()
    contact_points = client.list_contact_points()
    mute_timings = client.list_mute_timings()
    policies = client.get_notification_policies()

    root_index: Dict[str, List[Dict[str, str]]] = {
        RULES_SUBDIR: [],
        CONTACT_POINTS_SUBDIR: [],
        MUTE_TIMINGS_SUBDIR: [],
        POLICIES_SUBDIR: [],
    }

    for rule in rules:
        document = build_rule_export_document(rule)
        spec = document["spec"]
        output_path = build_rule_output_path(resource_dirs[RULE_KIND], spec, args.flat)
        write_json(document, output_path, args.overwrite)
        item = {
            "kind": RULE_KIND,
            "uid": str(spec.get("uid") or ""),
            "title": str(spec.get("title") or ""),
            "folderUID": str(spec.get("folderUID") or ""),
            "ruleGroup": str(spec.get("ruleGroup") or ""),
            "path": str(output_path),
        }
        root_index[RULES_SUBDIR].append(item)
        print(f"Exported alert rule {item['uid'] or 'unknown'} -> {output_path}")

    for contact_point in contact_points:
        document = build_contact_point_export_document(contact_point)
        spec = document["spec"]
        output_path = build_contact_point_output_path(
            resource_dirs[CONTACT_POINT_KIND], spec, args.flat
        )
        write_json(document, output_path, args.overwrite)
        item = {
            "kind": CONTACT_POINT_KIND,
            "uid": str(spec.get("uid") or ""),
            "name": str(spec.get("name") or ""),
            "type": str(spec.get("type") or ""),
            "path": str(output_path),
        }
        root_index[CONTACT_POINTS_SUBDIR].append(item)
        print(
            f"Exported contact point {item['uid'] or item['name'] or 'unknown'} -> {output_path}"
        )

    for mute_timing in mute_timings:
        document = build_mute_timing_export_document(mute_timing)
        spec = document["spec"]
        output_path = build_mute_timing_output_path(
            resource_dirs[MUTE_TIMING_KIND], spec, args.flat
        )
        write_json(document, output_path, args.overwrite)
        item = {
            "kind": MUTE_TIMING_KIND,
            "name": str(spec.get("name") or ""),
            "path": str(output_path),
        }
        root_index[MUTE_TIMINGS_SUBDIR].append(item)
        print(f"Exported mute timing {item['name'] or 'unknown'} -> {output_path}")

    policies_document = build_policies_export_document(policies)
    policies_path = build_policies_output_path(resource_dirs[POLICIES_KIND])
    write_json(policies_document, policies_path, args.overwrite)
    policies_item = {
        "kind": POLICIES_KIND,
        "receiver": str(policies_document["spec"].get("receiver") or ""),
        "path": str(policies_path),
    }
    root_index[POLICIES_SUBDIR].append(policies_item)
    print(
        "Exported notification policies "
        f"{policies_item['receiver'] or 'unknown'} -> {policies_path}"
    )

    for kind, subdir in RESOURCE_SUBDIR_BY_KIND.items():
        write_json(
            root_index[subdir],
            resource_dirs[kind] / "index.json",
            overwrite=True,
        )

    index_path = output_dir / "index.json"
    write_json(root_index, index_path, overwrite=True)
    print(
        "Exported "
        f"{len(root_index[RULES_SUBDIR])} alert rules, "
        f"{len(root_index[CONTACT_POINTS_SUBDIR])} contact points, "
        f"{len(root_index[MUTE_TIMINGS_SUBDIR])} mute timings, "
        f"{len(root_index[POLICIES_SUBDIR])} notification policy documents. "
        f"Root index: {index_path}"
    )
    return 0


def import_alerting_resources(args: argparse.Namespace) -> int:
    client = build_client(args)
    import_dir = Path(args.import_dir)
    resource_files = discover_alert_resource_files(import_dir)
    policies_seen = 0

    for resource_file in resource_files:
        document = load_json_file(resource_file)
        kind, payload = build_import_operation(document)
        if kind == POLICIES_KIND:
            policies_seen += 1
            if policies_seen > 1:
                raise GrafanaError(
                    "Multiple notification policy documents found in import set. "
                    "Import only one policy tree at a time."
                )

        if kind == RULE_KIND:
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
            identity = str(result.get("uid") or uid or "unknown")
        elif kind == CONTACT_POINT_KIND:
            uid = str(payload.get("uid") or "")
            if args.replace_existing and uid:
                existing = {str(item.get("uid") or "") for item in client.list_contact_points()}
                if uid in existing:
                    result = client.update_contact_point(uid, payload)
                    action = "updated"
                else:
                    result = client.create_contact_point(payload)
                    action = "created"
            else:
                result = client.create_contact_point(payload)
                action = "created"
            identity = str(result.get("uid") or uid or payload.get("name") or "unknown")
        elif kind == MUTE_TIMING_KIND:
            name = str(payload.get("name") or "")
            if args.replace_existing and name:
                existing = {str(item.get("name") or "") for item in client.list_mute_timings()}
                if name in existing:
                    result = client.update_mute_timing(name, payload)
                    action = "updated"
                else:
                    result = client.create_mute_timing(payload)
                    action = "created"
            else:
                result = client.create_mute_timing(payload)
                action = "created"
            identity = str(result.get("name") or name or "unknown")
        else:
            client.update_notification_policies(payload)
            action = "updated"
            identity = str(payload.get("receiver") or "root")

        print(f"Imported {resource_file} -> kind={kind} id={identity} action={action}")

    print(f"Imported {len(resource_files)} alerting resource files from {import_dir}")
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
            return import_alerting_resources(args)
        return export_alerting_resources(args)
    except GrafanaError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
