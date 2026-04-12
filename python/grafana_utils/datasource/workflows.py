"""Workflow and helper logic for the Python datasource CLI."""

import argparse
import csv
import difflib
import json
from copy import deepcopy
import sys
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from urllib import parse

from .. import yaml_compat as yaml
from ..dashboard_cli import (
    GrafanaError,
    build_client as build_dashboard_client,
    build_data_source_record,
    build_datasource_inventory_record,
    render_data_source_table,
    write_json_document,
)
from ..datasource_contract import (
    DATASOURCE_CONTRACT_FIELDS,
    normalize_datasource_record,
    validate_datasource_contract_record,
)
from ..datasource_diff import (
    build_live_datasource_diff_records,
    compare_datasource_bundle_to_live,
    load_datasource_diff_bundle,
)
from .live_mutation_render_safe import (
    build_live_mutation_dry_run_record,
    render_live_mutation_dry_run_json,
    render_live_mutation_dry_run_table,
)
from .live_mutation_safe import (
    add_datasource as add_live_datasource,
    delete_datasource as delete_live_datasource,
)
from ..dashboards.output_support import sanitize_path_component
from ..datasource_secret_provider_workbench import (
    collect_provider_references,
    iter_provider_names,
)
from ..datasource_secret_workbench import (
    collect_secret_placeholders,
    iter_secret_placeholder_names,
    resolve_secret_placeholders,
)
from .parser import (
    DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_IMPORT_FORMAT_CHOICES,
    DATASOURCE_PROVISIONING_FILENAME,
    DATASOURCE_PROVISIONING_SUBDIR,
    DIFF_OUTPUT_FORMAT_CHOICES,
    EXPORT_METADATA_FILENAME,
    IMPORT_DRY_RUN_COLUMN_ALIASES,
    IMPORT_DRY_RUN_COLUMN_HEADERS,
    LIST_OUTPUT_COLUMN_ALIASES,
    LIST_OUTPUT_COLUMN_HEADERS,
    ROOT_INDEX_KIND,
    TOOL_SCHEMA_VERSION,
)
from .catalog import (
    build_supported_datasource_catalog_document,
    build_add_defaults_for_supported_type,
    normalize_supported_datasource_type,
    render_supported_datasource_catalog_text,
)


def build_client(args):
    """Build client implementation."""
    return build_dashboard_client(args)


def build_export_index(datasource_records, datasources_file):
    """Build export index implementation."""
    document = {
        "kind": ROOT_INDEX_KIND,
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "datasourcesFile": datasources_file,
        "count": len(datasource_records),
        "items": [
            {
                "uid": record.get("uid") or "",
                "name": record.get("name") or "",
                "type": record.get("type") or "",
                "org": record.get("org") or "",
                "orgId": record.get("orgId") or "",
            }
            for record in datasource_records
        ],
    }
    document["provisioningFile"] = "%s/%s" % (
        DATASOURCE_PROVISIONING_SUBDIR,
        DATASOURCE_PROVISIONING_FILENAME,
    )
    return document


def build_export_metadata(datasource_count, datasources_file):
    """Build export metadata implementation."""
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "kind": ROOT_INDEX_KIND,
        "variant": "root",
        "resource": "datasource",
        "datasourceCount": datasource_count,
        "datasourcesFile": datasources_file,
        "provisioningFile": "%s/%s"
        % (DATASOURCE_PROVISIONING_SUBDIR, DATASOURCE_PROVISIONING_FILENAME),
        "indexFile": "index.json",
        "format": "grafana-datasource-inventory-v1",
    }


def build_all_orgs_export_index(items):
    """Build all orgs export index implementation."""
    return {
        "kind": ROOT_INDEX_KIND,
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "variant": "all-orgs-root",
        "count": len(items),
        "items": items,
    }


def build_all_orgs_export_metadata(org_count, datasource_count):
    """Build all orgs export metadata implementation."""
    return {
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "kind": ROOT_INDEX_KIND,
        "variant": "all-orgs-root",
        "resource": "datasource",
        "orgCount": org_count,
        "datasourceCount": datasource_count,
        "indexFile": "index.json",
        "format": "grafana-datasource-inventory-v1",
    }


def build_export_records(client):
    """Build export records implementation."""
    org = client.fetch_current_org()
    return [
        normalize_datasource_record(build_datasource_inventory_record(item, org))
        for item in client.list_datasources()
    ]


def build_all_orgs_output_dir(output_dir, org):
    """Build all orgs output dir implementation."""
    org_id = sanitize_path_component(str(org.get("id") or "unknown"))
    org_name = sanitize_path_component(str(org.get("name") or "org"))
    return output_dir / ("org_%s_%s" % (org_id, org_name))


def fetch_datasource_by_uid_if_exists(client, uid):
    """Fetch datasource by uid if exists implementation."""
    if not uid:
        return None
    try:
        data = client.request_json(
            "/api/datasources/uid/%s" % parse.quote(uid, safe="")
        )
    except Exception as exc:
        if isinstance(exc, exporter_api_error_type()):
            if exc.status_code == 404:
                return None
        raise
    if not isinstance(data, dict):
        raise GrafanaError("Unexpected datasource payload for UID %s." % uid)
    return data


def exporter_api_error_type():
    """Exporter api error type implementation."""
    from ..dashboards.common import GrafanaApiError

    return GrafanaApiError


def load_json_document(path):
    """Load json document implementation."""
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc))
    except ValueError as exc:
        raise GrafanaError("Invalid JSON in %s: %s" % (path, exc))


def load_json_object_argument(value, label):
    """Load json object argument implementation."""
    if value is None:
        return None
    try:
        data = json.loads(value)
    except ValueError as exc:
        raise GrafanaError("Invalid JSON for %s: %s" % (label, exc))
    if not isinstance(data, dict):
        raise GrafanaError("%s must decode to a JSON object." % label)
    return data


def load_json_object_file(path_value, label):
    """Load a JSON object from one file path."""
    if path_value is None:
        return None
    path = Path(path_value)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GrafanaError("Failed to read %s: %s" % (path, exc))
    except ValueError as exc:
        raise GrafanaError("Invalid JSON for %s: %s" % (label, exc))
    if not isinstance(data, dict):
        raise GrafanaError("%s must decode to a JSON object." % label)
    return data


def merge_json_object_fields(base, extra, label):
    """Merge json object fields implementation."""
    if extra is None:
        return dict(base or {})
    merged = dict(base or {})
    for key, value in extra.items():
        if key in merged:
            raise GrafanaError(
                "%s would overwrite existing key %r. Move that field to one place."
                % (label, key)
            )
        merged[key] = value
    return merged


def merge_json_object_defaults(existing, incoming):
    """Merge json object defaults implementation."""
    merged = deepcopy(existing or {})
    for key, value in (incoming or {}).items():
        current = merged.get(key)
        if isinstance(current, dict) and isinstance(value, dict):
            merged[key] = merge_json_object_defaults(current, value)
        else:
            merged[key] = value
    return merged


def parse_http_header_arguments(values):
    """Parse http header arguments implementation."""
    json_data = {}
    secure_json_data = {}
    for index, item in enumerate(values or [], 1):
        raw = str(item)
        if "=" not in raw:
            raise GrafanaError(
                "--http-header requires NAME=VALUE form. Invalid value: %r." % raw
            )
        name, value = raw.split("=", 1)
        name = name.strip()
        if not name:
            raise GrafanaError(
                "--http-header requires a non-empty header name. Invalid value: %r."
                % raw
            )
        json_data["httpHeaderName%s" % index] = name
        secure_json_data["httpHeaderValue%s" % index] = value
    return json_data, secure_json_data


def coerce_datasource_bool(value):
    """Coerce datasource bool-like values for provisioning output."""
    if isinstance(value, bool):
        return value
    normalized = str(value or "").strip().lower()
    return normalized in ("1", "true", "yes", "on")


def build_datasource_provisioning_document(records):
    """Build a Grafana datasource provisioning document from export records."""
    datasources = []
    for record in records:
        org_id_text = str(record.get("orgId") or "1").strip() or "1"
        try:
            org_id = int(org_id_text)
        except ValueError:
            org_id = 1
        item = {
            "name": str(record.get("name") or ""),
            "type": str(record.get("type") or ""),
            "access": str(record.get("access") or ""),
            "orgId": org_id,
            "uid": str(record.get("uid") or ""),
            "url": str(record.get("url") or ""),
            "isDefault": coerce_datasource_bool(record.get("isDefault")),
            "editable": False,
        }
        for source_key, target_key in (
            ("basicAuth", "basicAuth"),
            ("basicAuthUser", "basicAuthUser"),
            ("user", "user"),
            ("withCredentials", "withCredentials"),
            ("database", "database"),
            ("jsonData", "jsonData"),
            ("secureJsonDataPlaceholders", "secureJsonData"),
        ):
            if source_key not in record:
                continue
            value = record.get(source_key)
            if value in (None, ""):
                continue
            if source_key in ("basicAuth", "withCredentials"):
                value = coerce_datasource_bool(value)
            item[target_key] = deepcopy(value)
        datasources.append(item)
    return {"apiVersion": 1, "datasources": datasources}


def load_secret_value_map(args):
    """Load secret values from CLI arguments."""
    inline = load_json_object_argument(getattr(args, "secret_values", None), "--secret-values")
    file_map = load_json_object_file(
        getattr(args, "secret_values_file", None), "--secret-values-file"
    )
    if inline is not None and file_map is not None:
        raise GrafanaError("Choose either --secret-values or --secret-values-file, not both.")
    return inline if inline is not None else file_map


def resolve_secure_json_placeholders(secure_json_placeholders, secret_values):
    """Resolve secureJsonData placeholder declarations into concrete secrets."""
    placeholders = collect_secret_placeholders(secure_json_placeholders)
    if not placeholders:
        return {}
    if secret_values is None:
        raise GrafanaError("--secure-json-data-placeholders requires --secret-values.")
    resolved = {}
    for placeholder in placeholders:
        if placeholder.placeholder_name not in secret_values:
            raise GrafanaError(
                "Missing datasource secret placeholder '%s'."
                % placeholder.placeholder_name
            )
        resolved[placeholder.field_name] = secret_values[placeholder.placeholder_name]
    return resolved


def merge_secret_injection(spec, placeholders_value, secret_values, label):
    """Merge placeholder-driven secret injections into a datasource spec."""
    if placeholders_value is None:
        return spec
    resolved = resolve_secure_json_placeholders(placeholders_value, secret_values)
    secure = dict(spec.get("secureJsonData") or {})
    secure.update(resolved)
    spec = dict(spec)
    if secure:
        spec["secureJsonData"] = secure
    return spec


def load_local_datasource_bundle(input_dir, input_format):
    """Load local datasource inventory bundle for list/diff workflows."""
    if input_format is None:
        input_format = "inventory"
    if input_format not in DATASOURCE_IMPORT_FORMAT_CHOICES:
        raise GrafanaError("Unsupported datasource input format: %s" % input_format)
    if input_format == "provisioning":
        return load_provisioning_datasource_bundle(Path(input_dir))
    return load_import_bundle(Path(input_dir))


def resolve_datasource_provisioning_file(input_path):
    """Resolve a datasource provisioning input path to datasources.yaml/yml."""
    path = Path(input_path)
    if path.is_file():
        return path
    candidates = [
        path / "datasources.yaml",
        path / "datasources.yml",
        path / "provisioning" / "datasources.yaml",
        path / "provisioning" / "datasources.yml",
        path / "provisioning" / "datasources" / "datasources.yaml",
        path / "provisioning" / "datasources" / "datasources.yml",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise GrafanaError(
        "Datasource provisioning import did not find datasources.yaml under %s."
        % path
    )


def load_provisioning_datasource_bundle(input_path):
    """Load Grafana datasource provisioning YAML as normalized records."""
    provisioning_path = resolve_datasource_provisioning_file(input_path)
    try:
        document = yaml.safe_load(provisioning_path.read_text(encoding="utf-8")) or {}
    except Exception as exc:
        raise GrafanaError(
            "Failed to parse datasource provisioning YAML %s: %s"
            % (provisioning_path, exc)
        ) from exc
    raw_records = document.get("datasources") if isinstance(document, dict) else None
    if not isinstance(raw_records, list):
        raise GrafanaError(
            "Datasource provisioning file must contain a datasources list: %s"
            % provisioning_path
        )
    records = []
    for index, item in enumerate(raw_records):
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource provisioning entry must be a YAML object: %s#%s"
                % (provisioning_path, index)
            )
        record = normalize_datasource_record(
            {
                "uid": item.get("uid"),
                "name": item.get("name"),
                "type": item.get("type"),
                "access": item.get("access"),
                "url": item.get("url"),
                "isDefault": item.get("isDefault"),
                "org": "",
                "orgId": item.get("orgId"),
            }
        )
        for optional_key in (
            "database",
            "basicAuth",
            "basicAuthUser",
            "user",
            "withCredentials",
            "jsonData",
            "secureJsonData",
            "secureJsonFields",
            "secureJsonDataPlaceholders",
        ):
            if optional_key in item:
                record[optional_key] = deepcopy(item.get(optional_key))
        records.append(record)
    return {
        "records": records,
        "metadata": {
            "kind": ROOT_INDEX_KIND,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "resource": "datasource",
            "variant": "provisioning",
            "datasourceCount": len(records),
            "datasourcesFile": str(provisioning_path),
        },
        "datasources_path": provisioning_path,
    }


def parse_output_columns(value, aliases, headers):
    """Parse comma-separated output columns."""
    if value is None:
        return None
    columns = []
    for item in str(value).split(","):
        column = item.strip()
        if column:
            columns.append(aliases.get(column, column))
    if not columns:
        raise GrafanaError("Output columns requires one or more comma-separated values.")
    if columns == ["all"]:
        return list(headers.keys())
    unsupported = [column for column in columns if column not in headers]
    if unsupported:
        raise GrafanaError(
            "Unsupported output column(s): %s. Supported values: %s."
            % (", ".join(unsupported), ", ".join(sorted(aliases.keys())))
        )
    return columns


def render_datasource_list_table(records, include_header=True, selected_columns=None):
    """Render datasource list table with optional column selection."""
    columns = list(selected_columns or ["uid", "name", "type", "access", "url", "isDefault"])
    headers = [LIST_OUTPUT_COLUMN_HEADERS[column] for column in columns]
    rows = [[str(item.get(column) or "") for column in columns] for item in records]
    widths = [len(value) for value in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values):
        return "  ".join(values[index].ljust(widths[index]) for index in range(len(values)))

    lines = []
    if include_header:
        lines.append(render_row(headers))
        lines.append(render_row(["-" * width for width in widths]))
    lines.extend(render_row(row) for row in rows)
    return lines


def render_datasource_list_json(records):
    """Render datasource list JSON."""
    return json.dumps(records, indent=2, sort_keys=False)


def project_datasource_records(records, selected_columns=None):
    """Project datasource records to selected output columns."""
    if not selected_columns:
        return list(records)
    return [
        {column: record.get(column) for column in selected_columns}
        for record in records
    ]


def render_datasource_list_csv(records, selected_columns=None):
    """Render datasource summaries as CSV with optional selected columns."""
    columns = list(selected_columns or ["uid", "name", "type", "access", "url", "isDefault"])
    output = StringIO()
    writer = csv.DictWriter(output, fieldnames=columns)
    writer.writeheader()
    for record in records:
        writer.writerow({column: record.get(column) for column in columns})
    return output.getvalue()


def render_datasource_list_text(records, selected_columns=None):
    """Render datasource summaries as key=value text lines."""
    columns = list(selected_columns or ["uid", "name", "type", "access", "url"])
    return [
        " ".join("%s=%s" % (column, str(record.get(column) or "")) for column in columns)
        for record in records
    ]


def render_datasource_list_yaml(records, selected_columns=None):
    """Render datasource summaries as YAML."""
    if selected_columns:
        records = project_datasource_records(records, selected_columns)
    return yaml.safe_dump(records)


def _iter_supported_datasource_type_rows():
    """Flatten the datasource catalog for table/csv output."""
    document = build_supported_datasource_catalog_document()
    for category in document["categories"]:
        for item in category["types"]:
            yield {
                "category": category["category"],
                "type": item["type"],
                "displayName": item["displayName"],
                "profile": item["profile"],
                "queryLanguage": item["queryLanguage"],
                "aliases": ",".join(item.get("aliases") or []),
                "presetProfiles": ",".join(item.get("presetProfiles") or []),
            }


def render_supported_datasource_catalog_table():
    """Render supported datasource catalog rows as a table."""
    rows = list(_iter_supported_datasource_type_rows())
    columns = [
        ("category", "CATEGORY"),
        ("type", "TYPE"),
        ("displayName", "NAME"),
        ("profile", "PROFILE"),
        ("queryLanguage", "QUERY"),
        ("aliases", "ALIASES"),
        ("presetProfiles", "PRESETS"),
    ]
    widths = [len(header) for _, header in columns]
    values = []
    for row in rows:
        value_row = [str(row.get(key) or "") for key, _ in columns]
        values.append(value_row)
        for index, value in enumerate(value_row):
            widths[index] = max(widths[index], len(value))

    def render_row(row):
        return "  ".join(row[index].ljust(widths[index]) for index in range(len(row)))

    lines = [render_row([header for _, header in columns])]
    lines.append(render_row(["-" * width for width in widths]))
    lines.extend(render_row(row) for row in values)
    return lines


def render_supported_datasource_catalog_csv():
    """Render supported datasource catalog rows as CSV."""
    output = StringIO()
    fieldnames = [
        "category",
        "type",
        "displayName",
        "profile",
        "queryLanguage",
        "aliases",
        "presetProfiles",
    ]
    writer = csv.DictWriter(output, fieldnames=fieldnames)
    writer.writeheader()
    for row in _iter_supported_datasource_type_rows():
        writer.writerow(row)
    return output.getvalue()


def apply_secret_placeholders_to_record(record, secret_values):
    """Apply secureJsonData placeholder declarations to one datasource record."""
    if secret_values is None:
        return dict(record)
    placeholders = collect_secret_placeholders(record.get("secureJsonDataPlaceholders"))
    if not placeholders:
        return dict(record)
    resolved = resolve_secret_placeholders(placeholders, secret_values)
    merged = dict(record)
    secure_json_data = dict(merged.get("secureJsonData") or {})
    secure_json_data.update(resolved)
    merged["secureJsonData"] = secure_json_data
    return merged


def load_import_bundle(import_dir):
    """Load import bundle implementation."""
    if not import_dir.exists():
        raise GrafanaError("Import directory does not exist: %s" % import_dir)
    if not import_dir.is_dir():
        raise GrafanaError("Import path is not a directory: %s" % import_dir)
    metadata_path = import_dir / EXPORT_METADATA_FILENAME
    datasources_path = import_dir / DATASOURCE_EXPORT_FILENAME
    index_path = import_dir / "index.json"
    if not metadata_path.is_file():
        raise GrafanaError("Datasource import metadata is missing: %s" % metadata_path)
    if not datasources_path.is_file():
        raise GrafanaError("Datasource import file is missing: %s" % datasources_path)
    if not index_path.is_file():
        raise GrafanaError("Datasource import index is missing: %s" % index_path)
    metadata = load_json_document(metadata_path)
    if not isinstance(metadata, dict):
        raise GrafanaError(
            "Datasource import metadata must be a JSON object: %s" % metadata_path
        )
    if metadata.get("kind") != ROOT_INDEX_KIND:
        raise GrafanaError(
            "Unexpected datasource export manifest kind in %s: %r"
            % (metadata_path, metadata.get("kind"))
        )
    if metadata.get("schemaVersion") != TOOL_SCHEMA_VERSION:
        raise GrafanaError(
            "Unsupported datasource export schemaVersion %r in %s. Expected %s."
            % (metadata.get("schemaVersion"), metadata_path, TOOL_SCHEMA_VERSION)
        )
    if metadata.get("resource") != "datasource":
        raise GrafanaError(
            "Datasource import metadata in %s does not describe datasource inventory."
            % metadata_path
        )
    raw_records = load_json_document(datasources_path)
    if not isinstance(raw_records, list):
        raise GrafanaError(
            "Datasource import file must contain a JSON array: %s" % datasources_path
        )
    records = []
    allowed_secret_fields = {
        "secureJsonDataPlaceholders",
        "secureJsonDataProviders",
    }
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource import entry must be a JSON object: %s" % datasources_path
            )
        extra_fields = sorted(
            key
            for key in item.keys()
            if key not in set(DATASOURCE_CONTRACT_FIELDS) | allowed_secret_fields
        )
        if extra_fields:
            raise GrafanaError(
                "Datasource import entry in %s contains unsupported datasource field(s): %s. Supported fields: %s."
                % (
                    datasources_path,
                    ", ".join(extra_fields),
                    ", ".join(list(DATASOURCE_CONTRACT_FIELDS) + sorted(allowed_secret_fields)),
                )
            )
        contract_item = {
            key: value for key, value in item.items() if key in DATASOURCE_CONTRACT_FIELDS
        }
        try:
            validate_datasource_contract_record(
                contract_item,
                "Datasource import entry in %s" % datasources_path,
            )
        except ValueError as exc:
            raise GrafanaError(str(exc))
        normalized = normalize_datasource_record(item)
        if item.get("secureJsonDataPlaceholders") is not None:
            normalized["secureJsonDataPlaceholders"] = item.get(
                "secureJsonDataPlaceholders"
            )
        if item.get("secureJsonDataProviders") is not None:
            normalized["secureJsonDataProviders"] = item.get("secureJsonDataProviders")
        records.append(normalized)
    index_document = load_json_document(index_path)
    if not isinstance(index_document, dict):
        raise GrafanaError(
            "Datasource import index must be a JSON object: %s" % index_path
        )
    return {
        "metadata": metadata,
        "records": records,
        "index": index_document,
        "datasources_path": datasources_path,
    }


def _summarize_secret_details(record):
    """Internal helper for summarize secret details."""
    parts = []
    secret_fields = [
        str(field).strip()
        for field in record.get("secretFields") or []
        if str(field).strip()
    ]
    if secret_fields:
        parts.append("fields=%s" % ", ".join(secret_fields))
    placeholder_names = [
        str(name).strip()
        for name in record.get("secretPlaceholderNames") or []
        if str(name).strip()
    ]
    if placeholder_names:
        parts.append("placeholders=%s" % ", ".join(placeholder_names))
    provider_names = [
        str(name).strip()
        for name in record.get("providerNames") or []
        if str(name).strip()
    ]
    if provider_names:
        parts.append("providers=%s" % ", ".join(provider_names))
    return "; ".join(parts)


def load_import_bundle_preview(import_dir):
    """Load import bundle preview implementation."""
    if not import_dir.exists():
        raise GrafanaError("Import directory does not exist: %s" % import_dir)
    if not import_dir.is_dir():
        raise GrafanaError("Import path is not a directory: %s" % import_dir)
    metadata_path = import_dir / EXPORT_METADATA_FILENAME
    datasources_path = import_dir / DATASOURCE_EXPORT_FILENAME
    index_path = import_dir / "index.json"
    if not metadata_path.is_file():
        raise GrafanaError("Datasource import metadata is missing: %s" % metadata_path)
    if not datasources_path.is_file():
        raise GrafanaError("Datasource import file is missing: %s" % datasources_path)
    if not index_path.is_file():
        raise GrafanaError("Datasource import index is missing: %s" % index_path)
    metadata = load_json_document(metadata_path)
    if not isinstance(metadata, dict):
        raise GrafanaError(
            "Datasource import metadata must be a JSON object: %s" % metadata_path
        )
    if metadata.get("kind") != ROOT_INDEX_KIND:
        raise GrafanaError(
            "Unexpected datasource export manifest kind in %s: %r"
            % (metadata_path, metadata.get("kind"))
        )
    if metadata.get("schemaVersion") != TOOL_SCHEMA_VERSION:
        raise GrafanaError(
            "Unsupported datasource export schemaVersion %r in %s. Expected %s."
            % (metadata.get("schemaVersion"), metadata_path, TOOL_SCHEMA_VERSION)
        )
    if metadata.get("resource") != "datasource":
        raise GrafanaError(
            "Datasource import metadata in %s does not describe datasource inventory."
            % metadata_path
        )
    raw_records = load_json_document(datasources_path)
    if not isinstance(raw_records, list):
        raise GrafanaError(
            "Datasource import file must contain a JSON array: %s" % datasources_path
        )
    records = []
    allowed_secret_fields = {
        "secureJsonDataPlaceholders",
        "secureJsonDataProviders",
    }
    for item in raw_records:
        if not isinstance(item, dict):
            raise GrafanaError(
                "Datasource import entry must be a JSON object: %s" % datasources_path
            )
        extra_fields = sorted(
            key
            for key in item.keys()
            if key not in set(DATASOURCE_CONTRACT_FIELDS) | allowed_secret_fields
        )
        if extra_fields:
            raise GrafanaError(
                "Datasource import entry in %s contains unsupported datasource field(s): %s. Supported fields: %s."
                % (
                    datasources_path,
                    ", ".join(extra_fields),
                    ", ".join(
                        list(DATASOURCE_CONTRACT_FIELDS)
                        + sorted(allowed_secret_fields)
                    ),
                )
            )
        normalized = normalize_datasource_record(item)
        placeholders = collect_secret_placeholders(
            item.get("secureJsonDataPlaceholders")
        )
        providers = collect_provider_references(item.get("secureJsonDataProviders"))
        secret_fields = [placeholder.field_name for placeholder in placeholders]
        secret_placeholder_names = list(iter_secret_placeholder_names(placeholders))
        provider_names = list(iter_provider_names(providers))
        preview_record = dict(normalized)
        if secret_fields:
            preview_record["secretFields"] = secret_fields
        if secret_placeholder_names:
            preview_record["secretPlaceholderNames"] = secret_placeholder_names
        if provider_names:
            preview_record["providerNames"] = provider_names
        secret_summary = _summarize_secret_details(preview_record)
        if secret_summary:
            preview_record["secretSummary"] = secret_summary
        records.append(preview_record)
    index_document = load_json_document(index_path)
    if not isinstance(index_document, dict):
        raise GrafanaError(
            "Datasource import index must be a JSON object: %s" % index_path
        )
    return {
        "metadata": metadata,
        "records": records,
        "index": index_document,
        "datasources_path": datasources_path,
    }


def resolve_export_org_id(bundle):
    """Resolve export org id implementation."""
    org_ids = set()
    index_document = bundle.get("index")
    if isinstance(index_document, dict):
        for item in index_document.get("items") or []:
            if isinstance(item, dict):
                org_id = str(item.get("orgId") or "").strip()
                if org_id:
                    org_ids.add(org_id)
    for record in bundle.get("records") or []:
        org_id = str(record.get("orgId") or "").strip()
        if org_id:
            org_ids.add(org_id)
    if not org_ids:
        return None
    if len(org_ids) > 1:
        raise GrafanaError(
            "Datasource export metadata spans multiple orgIds (%s). Remove "
            "--require-matching-export-org or point --import-dir at one org-specific export."
            % ", ".join(sorted(org_ids))
        )
    return list(org_ids)[0]


def resolve_export_org_name(bundle):
    """Resolve export org name implementation."""
    org_names = set()
    index_document = bundle.get("index")
    if isinstance(index_document, dict):
        for item in index_document.get("items") or []:
            if isinstance(item, dict):
                org_name = str(item.get("org") or "").strip()
                if org_name:
                    org_names.add(org_name)
    for record in bundle.get("records") or []:
        org_name = str(record.get("org") or "").strip()
        if org_name:
            org_names.add(org_name)
    if not org_names:
        return None
    if len(org_names) > 1:
        raise GrafanaError(
            "Datasource export metadata spans multiple org names (%s). Point "
            "--import-dir at one org-specific export." % ", ".join(sorted(org_names))
        )
    return list(org_names)[0]


def _normalize_org_id(org):
    """Internal helper for normalize org id."""
    if not isinstance(org, dict):
        return None
    value = org.get("id")
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _clone_import_args(args, **overrides):
    """Internal helper for clone import args."""
    values = dict(vars(args))
    values.update(overrides)
    return argparse.Namespace(**values)


def _resolve_existing_orgs_by_id(client):
    """Internal helper for resolve existing orgs by id."""
    orgs_by_id = {}
    for item in client.list_orgs():
        org_id = _normalize_org_id(item)
        if org_id:
            orgs_by_id[org_id] = dict(item)
    return orgs_by_id


def _resolve_created_org_id(created_payload):
    """Internal helper for resolve created org id."""
    if not isinstance(created_payload, dict):
        return None
    org_id = created_payload.get("orgId")
    if org_id is None:
        org_id = created_payload.get("id")
    if org_id is None:
        return None
    text = str(org_id).strip()
    return text or None


def create_organization(client, name):
    """Create organization implementation."""
    return client.create_organization({"name": name})


def _discover_org_export_dirs(import_dir):
    """Internal helper for discover org export dirs."""
    if not import_dir.exists():
        raise GrafanaError("Import directory does not exist: %s" % import_dir)
    if not import_dir.is_dir():
        raise GrafanaError("Import path is not a directory: %s" % import_dir)
    org_dirs = []
    for child in sorted(import_dir.iterdir(), key=lambda item: item.name):
        if not child.is_dir():
            continue
        if not child.name.startswith("org_"):
            continue
        metadata_path = child / EXPORT_METADATA_FILENAME
        datasources_path = child / DATASOURCE_EXPORT_FILENAME
        index_path = child / "index.json"
        if (
            metadata_path.is_file()
            and datasources_path.is_file()
            and index_path.is_file()
        ):
            org_dirs.append(child)
    if org_dirs:
        return org_dirs
    if (import_dir / DATASOURCE_EXPORT_FILENAME).is_file():
        raise GrafanaError(
            "Datasource import with --use-export-org expects the combined export root, "
            "not one org-specific datasource export directory."
        )
    raise GrafanaError(
        "Datasource import with --use-export-org could not find any org-prefixed "
        "datasource export directories under %s." % import_dir
    )


def build_effective_import_client(args, client):
    """Build effective import client implementation."""
    org_id = getattr(args, "org_id", None)
    auth_header = client.headers.get("Authorization", "")
    if org_id and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )
    if org_id:
        return client.with_org_id(str(org_id))
    return client


def validate_export_org_match(args, client, bundle):
    """Validate export org match implementation."""
    target_org = client.fetch_current_org()
    target_org_id = str(target_org.get("id") or "").strip()
    if not target_org_id:
        raise GrafanaError("Grafana did not return a usable target org id.")
    if not bool(getattr(args, "require_matching_export_org", False)):
        return target_org_id
    source_org_id = resolve_export_org_id(bundle)
    if not source_org_id:
        raise GrafanaError(
            "Could not determine one source export orgId while "
            "--require-matching-export-org is active."
        )
    if source_org_id != target_org_id:
        raise GrafanaError(
            "Raw export orgId %s does not match target Grafana org id %s. "
            "Remove --require-matching-export-org to allow cross-org import."
            % (source_org_id, target_org_id)
        )
    return target_org_id


def build_existing_datasource_lookups(client):
    """Build existing datasource lookups implementation."""
    by_uid = {}
    by_name = {}
    for datasource in client.list_datasources():
        uid = str(datasource.get("uid") or "")
        name = str(datasource.get("name") or "")
        if uid:
            by_uid.setdefault(uid, []).append(datasource)
        if name:
            by_name.setdefault(name, []).append(datasource)
    return {"by_uid": by_uid, "by_name": by_name}


def resolve_datasource_match(record, lookups):
    """Resolve datasource match implementation."""
    uid = str(record.get("uid") or "")
    name = str(record.get("name") or "")
    if uid:
        matches = lookups["by_uid"].get(uid) or []
        if len(matches) > 1:
            return {"state": "ambiguous", "target": None}
        if len(matches) == 1:
            return {"state": "exists-uid", "target": matches[0]}
    if name:
        matches = lookups["by_name"].get(name) or []
        if len(matches) > 1:
            return {"state": "ambiguous", "target": None}
        if len(matches) == 1:
            return {"state": "exists-name", "target": matches[0]}
    return {"state": "missing", "target": None}


def determine_import_mode(args):
    """Determine import mode implementation."""
    if bool(getattr(args, "update_existing_only", False)):
        return "update-or-skip-missing"
    if bool(getattr(args, "replace_existing", False)):
        return "create-or-update"
    return "create-only"


def determine_datasource_action(args, record, match):
    """Determine datasource action implementation."""
    state = match["state"]
    existing = match.get("target")
    if state == "ambiguous":
        return "would-fail-ambiguous"
    if existing is not None:
        existing_uid = str(existing.get("uid") or "")
        incoming_uid = str(record.get("uid") or "")
        if (
            state == "exists-name"
            and existing_uid
            and incoming_uid
            and existing_uid != incoming_uid
        ):
            return "would-fail-uid-mismatch"
        existing_type = str(existing.get("type") or "")
        incoming_type = str(record.get("type") or "")
        if existing_type and incoming_type and existing_type != incoming_type:
            return "would-fail-plugin-type-change"
    if state == "missing":
        if bool(getattr(args, "update_existing_only", False)):
            return "would-skip-missing"
        return "would-create"
    if bool(getattr(args, "replace_existing", False)) or bool(
        getattr(args, "update_existing_only", False)
    ):
        return "would-update"
    return "would-fail-existing"


def build_import_payload(record, existing=None):
    """Build import payload implementation."""
    payload = {
        "name": record.get("name") or "",
        "type": record.get("type") or "",
        "access": record.get("access") or "",
        "url": record.get("url") or "",
        "isDefault": str(record.get("isDefault") or "").lower() == "true",
    }
    uid = record.get("uid") or ""
    if uid:
        payload["uid"] = uid
    if record.get("secureJsonData"):
        payload["secureJsonData"] = dict(record.get("secureJsonData") or {})
    if existing is not None:
        datasource_id = existing.get("id")
        if datasource_id is not None:
            payload["id"] = datasource_id
    return payload


def parse_import_dry_run_columns(value):
    """Parse import dry run columns implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    if value is None:
        return None
    columns = []
    for item in str(value).split(","):
        column = item.strip()
        if column:
            columns.append(IMPORT_DRY_RUN_COLUMN_ALIASES.get(column, column))
    if not columns:
        raise GrafanaError(
            "--output-columns requires one or more comma-separated datasource import dry-run column ids."
        )
    unsupported = [
        column for column in columns if column not in IMPORT_DRY_RUN_COLUMN_HEADERS
    ]
    if unsupported:
        raise GrafanaError(
            "Unsupported datasource import dry-run column(s): %s. Supported values: %s."
            % (
                ", ".join(unsupported),
                ", ".join(sorted(IMPORT_DRY_RUN_COLUMN_ALIASES.keys())),
            )
        )
    return columns


def render_import_dry_run_table(records, include_header, selected_columns=None):
    """Render import dry run table implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1561
    #   Downstream callees: 580

    columns = list(
        selected_columns
        or [
            "uid",
            "name",
            "type",
            "destination",
            "action",
            "orgId",
            "file",
            "secretSummary",
        ]
    )
    headers = [IMPORT_DRY_RUN_COLUMN_HEADERS[column] for column in columns]
    rows = [[item.get(column) or "" for column in columns] for item in records]
    widths = [len(value) for value in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            values[index].ljust(widths[index]) for index in range(len(values))
        )

    lines = []
    if include_header:
        lines.append(render_row(headers))
        lines.append(render_row(["-" * width for width in widths]))
    for row in rows:
        lines.append(render_row(row))
    return lines


def render_import_dry_run_json(mode, records, target_org_id):
    """Render import dry run json implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1561
    #   Downstream callees: 無

    summary = {
        "datasourceCount": len(records),
        "createCount": len(
            [item for item in records if item["action"] == "would-create"]
        ),
        "updateCount": len(
            [item for item in records if item["action"] == "would-update"]
        ),
        "skipCount": len(
            [item for item in records if item["action"] == "would-skip-missing"]
        ),
        "blockedCount": len(
            [
                item
                for item in records
                if item["action"]
                in (
                    "would-fail-existing",
                    "would-fail-ambiguous",
                    "would-fail-plugin-type-change",
                    "would-fail-uid-mismatch",
                )
            ]
        ),
    }
    source_org_id = ""
    if records:
        source_org_id = str(records[0].get("sourceOrgId") or "")
    return json.dumps(
        {
            "mode": mode,
            "sourceOrgId": source_org_id,
            "targetOrgId": target_org_id,
            "datasources": records,
            "summary": summary,
        },
        indent=2,
        sort_keys=False,
    )


def render_data_source_csv(datasources):
    """Render data source csv implementation."""
    writer = csv.DictWriter(
        sys.stdout,
        fieldnames=["uid", "name", "type", "url", "isDefault"],
        lineterminator="\n",
    )
    writer.writeheader()
    for datasource in datasources:
        writer.writerow(build_data_source_record(datasource))


def render_data_source_json(datasources):
    """Render data source json implementation."""
    return json.dumps(
        [build_data_source_record(item) for item in datasources],
        indent=2,
        sort_keys=False,
    )


def build_add_datasource_spec(args):
    """Build add datasource spec implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 880
    #   Downstream callees: 167, 180, 195

    normalized_type = normalize_supported_datasource_type(args.type)
    spec = {
        "name": args.name,
        "type": normalized_type,
    }
    preset_profile = getattr(args, "preset_profile", None)
    if preset_profile is None and bool(getattr(args, "apply_supported_defaults", False)):
        preset_profile = "starter"
    if preset_profile is not None:
        defaults = build_add_defaults_for_supported_type(
            normalized_type,
            preset_profile=preset_profile,
        )
        if "access" in defaults and not getattr(args, "access", None):
            spec["access"] = defaults["access"]
        if defaults.get("jsonData"):
            spec["jsonData"] = dict(defaults["jsonData"])
    if getattr(args, "uid", None):
        spec["uid"] = args.uid
    if getattr(args, "access", None):
        spec["access"] = args.access
    if getattr(args, "datasource_url", None):
        spec["url"] = args.datasource_url
    if bool(getattr(args, "is_default", False)):
        spec["isDefault"] = True
    if (
        bool(getattr(args, "basic_auth", False))
        or getattr(args, "basic_auth_user", None)
        or getattr(args, "basic_auth_password", None)
    ):
        spec["basicAuth"] = True
    if getattr(args, "basic_auth_user", None):
        spec["basicAuthUser"] = args.basic_auth_user
    if getattr(args, "user", None):
        spec["user"] = args.user
    if bool(getattr(args, "with_credentials", False)):
        spec["withCredentials"] = True

    json_data = load_json_object_argument(
        getattr(args, "json_data", None), "--json-data"
    )
    secure_json_data = load_json_object_argument(
        getattr(args, "secure_json_data", None),
        "--secure-json-data",
    )
    secure_json_data_placeholders = load_json_object_argument(
        getattr(args, "secure_json_data_placeholders", None),
        "--secure-json-data-placeholders",
    )
    secret_values = load_secret_value_map(args)

    derived_json_data = {}
    if bool(getattr(args, "tls_skip_verify", False)):
        derived_json_data["tlsSkipVerify"] = True
    if getattr(args, "server_name", None):
        derived_json_data["serverName"] = args.server_name
    header_json_data, header_secure_json_data = parse_http_header_arguments(
        getattr(args, "http_header", None)
    )
    derived_json_data.update(header_json_data)
    json_data = merge_json_object_fields(json_data, derived_json_data, "--json-data")
    if json_data:
        spec["jsonData"] = merge_json_object_defaults(spec.get("jsonData"), json_data)

    derived_secure_json_data = {}
    if getattr(args, "basic_auth_password", None):
        derived_secure_json_data["basicAuthPassword"] = args.basic_auth_password
    if getattr(args, "password", None):
        derived_secure_json_data["password"] = args.password
    derived_secure_json_data.update(header_secure_json_data)
    secure_json_data = merge_json_object_fields(
        secure_json_data,
        derived_secure_json_data,
        "--secure-json-data",
    )
    if secure_json_data:
        spec["secureJsonData"] = secure_json_data

    spec = merge_secret_injection(
        spec,
        secure_json_data_placeholders,
        secret_values,
        "--secure-json-data-placeholders",
    )

    if getattr(args, "basic_auth_password", None) and not getattr(
        args, "basic_auth_user", None
    ):
        raise GrafanaError("--basic-auth-password requires --basic-auth-user.")

    return spec


def build_modify_datasource_updates(args):
    """Build modify datasource updates implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 934
    #   Downstream callees: 167, 180, 195

    spec = {}
    if getattr(args, "set_url", None) is not None:
        spec["url"] = args.set_url
    if getattr(args, "set_access", None) is not None:
        spec["access"] = args.set_access
    if getattr(args, "set_default", None) is not None:
        spec["isDefault"] = bool(args.set_default)
    if (
        bool(getattr(args, "basic_auth", False))
        or getattr(args, "basic_auth_user", None)
        or getattr(args, "basic_auth_password", None)
    ):
        spec["basicAuth"] = True
    if getattr(args, "basic_auth_user", None) is not None:
        spec["basicAuthUser"] = args.basic_auth_user
    if getattr(args, "user", None) is not None:
        spec["user"] = args.user
    if bool(getattr(args, "with_credentials", False)):
        spec["withCredentials"] = True

    json_data = load_json_object_argument(
        getattr(args, "json_data", None), "--json-data"
    )
    secure_json_data = load_json_object_argument(
        getattr(args, "secure_json_data", None),
        "--secure-json-data",
    )
    secure_json_data_placeholders = load_json_object_argument(
        getattr(args, "secure_json_data_placeholders", None),
        "--secure-json-data-placeholders",
    )
    secret_values = load_secret_value_map(args)

    derived_json_data = {}
    if bool(getattr(args, "tls_skip_verify", False)):
        derived_json_data["tlsSkipVerify"] = True
    if getattr(args, "server_name", None) is not None:
        derived_json_data["serverName"] = args.server_name
    header_json_data, header_secure_json_data = parse_http_header_arguments(
        getattr(args, "http_header", None)
    )
    derived_json_data.update(header_json_data)
    json_data = merge_json_object_fields(json_data, derived_json_data, "--json-data")
    if json_data:
        spec["jsonData"] = json_data

    derived_secure_json_data = {}
    if getattr(args, "basic_auth_password", None) is not None:
        derived_secure_json_data["basicAuthPassword"] = args.basic_auth_password
    if getattr(args, "password", None) is not None:
        derived_secure_json_data["password"] = args.password
    derived_secure_json_data.update(header_secure_json_data)
    secure_json_data = merge_json_object_fields(
        secure_json_data,
        derived_secure_json_data,
        "--secure-json-data",
    )
    if secure_json_data:
        spec["secureJsonData"] = secure_json_data

    spec = merge_secret_injection(
        spec,
        secure_json_data_placeholders,
        secret_values,
        "--secure-json-data-placeholders",
    )

    if not spec:
        raise GrafanaError("Datasource modify requires at least one change flag.")
    return spec


def split_live_add_supported_spec(spec):
    """Split live add supported spec implementation."""
    safe_spec = {}
    extra_top_level = {}
    for key, value in spec.items():
        if key in ("basicAuth", "basicAuthUser", "user", "withCredentials"):
            extra_top_level[key] = value
            continue
        safe_spec[key] = value
    return safe_spec, extra_top_level


def build_modify_datasource_payload(existing, updates):
    """Build modify datasource payload implementation."""
    payload = {
        "id": existing.get("id"),
        "uid": existing.get("uid") or "",
        "name": existing.get("name") or "",
        "type": existing.get("type") or "",
        "access": existing.get("access") or "",
        "url": existing.get("url") or "",
        "isDefault": bool(existing.get("isDefault")),
    }
    for key in (
        "orgId",
        "basicAuth",
        "basicAuthUser",
        "user",
        "database",
        "withCredentials",
    ):
        if key in existing and existing.get(key) is not None:
            payload[key] = existing.get(key)
    existing_json_data = existing.get("jsonData")
    payload["jsonData"] = deepcopy(existing_json_data or {})

    for key in (
        "name",
        "url",
        "access",
        "isDefault",
        "basicAuth",
        "basicAuthUser",
        "user",
        "withCredentials",
    ):
        if key in updates:
            payload[key] = updates[key]

    if "jsonData" in updates:
        payload["jsonData"] = merge_json_object_defaults(
            payload.get("jsonData"), updates["jsonData"]
        )

    if "secureJsonData" in updates:
        payload["secureJsonData"] = dict(updates["secureJsonData"])

    if (
        "secureJsonData" in payload
        and payload["secureJsonData"].get("basicAuthPassword") is not None
        and not str(payload.get("basicAuthUser") or "").strip()
    ):
        raise GrafanaError(
            "Datasource modify requires --basic-auth-user or an existing basicAuthUser when setting a basic auth password."
        )
    return payload


def plan_modify_datasource(client, uid, updates):
    """Plan modify datasource implementation."""
    existing = fetch_datasource_by_uid_if_exists(client, uid)
    if existing is None:
        return {
            "action": "would-fail-missing",
            "match": "missing",
            "target": None,
            "payload": None,
        }
    payload = build_modify_datasource_payload(existing, updates)
    return {
        "action": "would-update",
        "match": "exists-uid",
        "target": existing,
        "payload": payload,
    }


def render_modify_dry_run_json(record):
    """Render modify dry run json implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 934
    #   Downstream callees: 無

    return json.dumps(
        {
            "items": [record],
            "summary": {
                "itemCount": 1,
                "updateCount": 1 if record.get("action") == "would-update" else 0,
                "blockedCount": (
                    1
                    if str(record.get("action") or "").startswith("would-fail-")
                    else 0
                ),
            },
        },
        indent=2,
        sort_keys=False,
    )


def _validate_live_mutation_dry_run_args(args, verb):
    """Internal helper for validate live mutation dry run args."""
    # Call graph: see callers/callees.
    #   Upstream callers: 880, 934, 994
    #   Downstream callees: 無

    if getattr(args, "table", False) and not args.dry_run:
        raise GrafanaError(
            "--table is only supported with --dry-run for datasource %s." % verb
        )
    if getattr(args, "json", False) and not args.dry_run:
        raise GrafanaError(
            "--json is only supported with --dry-run for datasource %s." % verb
        )
    if getattr(args, "table", False) and getattr(args, "json", False):
        raise GrafanaError(
            "--table and --json are mutually exclusive for datasource %s." % verb
        )
    if getattr(args, "no_header", False) and not getattr(args, "table", False):
        raise GrafanaError(
            "--no-header is only supported with --dry-run --table for datasource %s."
            % verb
        )


def add_datasource(args):
    """Add datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1697
    #   Downstream callees: 52, 656, 778, 865

    _validate_live_mutation_dry_run_args(args, "add")
    spec = build_add_datasource_spec(args)
    safe_spec, extra_top_level = split_live_add_supported_spec(spec)
    client = build_client(args)
    result = add_live_datasource(client, safe_spec, dry_run=True)
    if args.dry_run:
        record = build_live_mutation_dry_run_record("add", result, spec=safe_spec)
        if getattr(args, "json", False):
            print(render_live_mutation_dry_run_json([record]))
            return 0
        if getattr(args, "table", False):
            for line in render_live_mutation_dry_run_table(
                [record],
                include_header=not bool(getattr(args, "no_header", False)),
            ):
                print(line)
        else:
            print(
                "Dry-run datasource add uid=%s name=%s match=%s action=%s"
                % (
                    record.get("uid") or "-",
                    record.get("name") or "-",
                    record.get("match") or "-",
                    record.get("action") or "-",
                )
            )
        print("Dry-run checked 1 datasource add request")
        return 0
    if result.get("action") != "would-create":
        raise GrafanaError(
            "Datasource add blocked for name=%s uid=%s match=%s action=%s"
            % (
                safe_spec.get("name") or "-",
                safe_spec.get("uid") or "-",
                result.get("match") or "-",
                result.get("action") or "-",
            )
        )
    payload = dict(result.get("payload") or {})
    payload.update(extra_top_level)
    client.request_json(
        "/api/datasources",
        method="POST",
        payload=payload,
    )
    print(
        "Created datasource uid=%s name=%s"
        % (payload.get("uid") or "-", payload.get("name") or "-")
    )
    return 0


def modify_datasource(args):
    """Modify datasource implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1697
    #   Downstream callees: 52, 720, 830, 849, 865

    _validate_live_mutation_dry_run_args(args, "modify")
    updates = build_modify_datasource_updates(args)
    client = build_client(args)
    result = plan_modify_datasource(client, getattr(args, "uid", None), updates)
    record = build_live_mutation_dry_run_record(
        "modify",
        result,
        spec=result.get("payload") or {"uid": getattr(args, "uid", None)},
        uid=getattr(args, "uid", None),
    )
    if args.dry_run:
        if getattr(args, "json", False):
            print(render_modify_dry_run_json(record))
            return 0
        if getattr(args, "table", False):
            for line in render_live_mutation_dry_run_table(
                [record],
                include_header=not bool(getattr(args, "no_header", False)),
            ):
                print(line)
        else:
            print(
                "Dry-run datasource modify uid=%s name=%s match=%s action=%s"
                % (
                    record.get("uid") or "-",
                    record.get("name") or "-",
                    record.get("match") or "-",
                    record.get("action") or "-",
                )
            )
        print("Dry-run checked 1 datasource modify request")
        return 0
    if result.get("action") != "would-update":
        raise GrafanaError(
            "Datasource modify blocked for uid=%s match=%s action=%s"
            % (
                getattr(args, "uid", None) or "-",
                result.get("match") or "-",
                result.get("action") or "-",
            )
        )
    payload = result.get("payload") or {}
    client.request_json(
        "/api/datasources/%s" % payload["id"],
        method="PUT",
        payload=payload,
    )
    print(
        "Modified datasource uid=%s name=%s id=%s"
        % (
            payload.get("uid") or "-",
            payload.get("name") or "-",
            payload.get("id") or "-",
        )
    )
    return 0


def delete_datasource(args):
    """Delete datasource implementation."""
    _validate_live_mutation_dry_run_args(args, "delete")
    if not bool(getattr(args, "dry_run", False)) and not bool(getattr(args, "yes", False)):
        raise GrafanaError("Datasource delete requires --yes unless --dry-run is set.")
    client = build_client(args)
    result = delete_live_datasource(
        client,
        uid=getattr(args, "uid", None),
        name=getattr(args, "name", None),
        dry_run=bool(args.dry_run),
    )
    if args.dry_run:
        record = build_live_mutation_dry_run_record(
            "delete",
            result,
            uid=getattr(args, "uid", None),
            name=getattr(args, "name", None),
        )
        if getattr(args, "json", False):
            print(render_live_mutation_dry_run_json([record]))
            return 0
        if getattr(args, "table", False):
            for line in render_live_mutation_dry_run_table(
                [record],
                include_header=not bool(getattr(args, "no_header", False)),
            ):
                print(line)
        else:
            print(
                "Dry-run datasource delete uid=%s name=%s match=%s action=%s"
                % (
                    record.get("uid") or "-",
                    record.get("name") or "-",
                    record.get("match") or "-",
                    record.get("action") or "-",
                )
            )
        print("Dry-run checked 1 datasource delete request")
        return 0
    target = result.get("target") or {}
    print(
        "Deleted datasource uid=%s name=%s id=%s"
        % (
            target.get("uid") or "-",
            target.get("name") or "-",
            target.get("id") or "-",
        )
    )
    return 0


def list_datasources(args):
    """List datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 116, 1697, 448
    #   Downstream callees: 334, 52, 635, 647

    input_dir = getattr(args, "input_dir", None)
    if input_dir:
        if getattr(args, "org_id", None) or bool(getattr(args, "all_orgs", False)):
            raise GrafanaError("Datasource list with --input-dir does not support --org-id or --all-orgs.")
        if bool(getattr(args, "interactive", False)):
            raise GrafanaError("Datasource list interactive mode is not implemented in Python yet.")
        bundle = load_local_datasource_bundle(input_dir, getattr(args, "input_format", "inventory"))
        records = bundle["records"]
        if getattr(args, "list_columns", False):
            for column in LIST_OUTPUT_COLUMN_HEADERS:
                print(column)
            return 0
        selected_columns = parse_output_columns(
            getattr(args, "output_columns", None),
            LIST_OUTPUT_COLUMN_ALIASES,
            LIST_OUTPUT_COLUMN_HEADERS,
        )
        if args.json:
            print(
                json.dumps(
                    project_datasource_records(records, selected_columns),
                    indent=2,
                    sort_keys=False,
                )
            )
            return 0
        if args.csv:
            print(render_datasource_list_csv(records, selected_columns), end="")
            return 0
        if getattr(args, "yaml", False):
            print(render_datasource_list_yaml(records, selected_columns), end="")
            return 0
        if getattr(args, "text", False):
            for line in render_datasource_list_text(records, selected_columns):
                print(line)
            return 0
        for line in render_datasource_list_table(
            records,
            include_header=not bool(getattr(args, "no_header", False)),
            selected_columns=selected_columns,
        ):
            print(line)
        print("")
        print("Listed %s local datasource(s) from %s" % (len(records), input_dir))
        return 0

    client = build_client(args)
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )

    datasources = []
    if all_orgs:
        for org in client.list_orgs():
            scoped_org_id = _normalize_org_id(org)
            if not scoped_org_id:
                continue
            scoped_client = client.with_org_id(scoped_org_id)
            scoped_org = scoped_client.fetch_current_org()
            for datasource in scoped_client.list_datasources():
                item = dict(datasource)
                item["org"] = str(scoped_org.get("name") or "")
                item["orgId"] = str(scoped_org.get("id") or "")
                datasources.append(item)
        datasources.sort(
            key=lambda item: (
                str(item.get("orgId") or ""),
                str(item.get("name") or ""),
                str(item.get("uid") or ""),
            )
        )
    elif org_id:
        scoped_client = client.with_org_id(str(org_id))
        scoped_org = scoped_client.fetch_current_org()
        for datasource in scoped_client.list_datasources():
            item = dict(datasource)
            item["org"] = str(scoped_org.get("name") or "")
            item["orgId"] = str(scoped_org.get("id") or "")
            datasources.append(item)
    else:
        datasources = client.list_datasources()
    selected_columns = parse_output_columns(
        getattr(args, "output_columns", None),
        LIST_OUTPUT_COLUMN_ALIASES,
        LIST_OUTPUT_COLUMN_HEADERS,
    )
    if args.csv:
        print(render_datasource_list_csv(datasources, selected_columns), end="")
        return 0
    if args.json:
        print(
            json.dumps(
                project_datasource_records(datasources, selected_columns),
                indent=2,
                sort_keys=False,
            )
        )
        return 0
    if getattr(args, "yaml", False):
        print(render_datasource_list_yaml(datasources, selected_columns), end="")
        return 0
    if getattr(args, "text", False):
        for line in render_datasource_list_text(datasources, selected_columns):
            print(line)
        return 0
    table_renderer = render_datasource_list_table if selected_columns else render_data_source_table
    table_kwargs = {"selected_columns": selected_columns} if selected_columns else {}
    for line in table_renderer(
        datasources,
        include_header=not bool(getattr(args, "no_header", False)),
        **table_kwargs,
    ):
        print(line)
    print("")
    print("Listed %s data source(s) from %s" % (len(datasources), args.url))
    return 0


def _collect_live_datasource_records(args, client=None):
    """Collect live datasource records with optional org scope."""
    client = client or build_client(args)
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    auth_header = client.headers.get("Authorization", "")
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )
    records = []
    if all_orgs:
        for org in client.list_orgs():
            scoped_org_id = _normalize_org_id(org)
            if not scoped_org_id:
                continue
            scoped_client = client.with_org_id(scoped_org_id)
            scoped_org = scoped_client.fetch_current_org()
            for datasource in scoped_client.list_datasources():
                item = dict(datasource)
                item["org"] = str(scoped_org.get("name") or "")
                item["orgId"] = str(scoped_org.get("id") or "")
                records.append(item)
        records.sort(
            key=lambda item: (
                str(item.get("orgId") or ""),
                str(item.get("name") or ""),
                str(item.get("uid") or ""),
            )
        )
        return records
    if org_id:
        scoped_client = client.with_org_id(str(org_id))
        scoped_org = scoped_client.fetch_current_org()
        for datasource in scoped_client.list_datasources():
            item = dict(datasource)
            item["org"] = str(scoped_org.get("name") or "")
            item["orgId"] = str(scoped_org.get("id") or "")
            records.append(item)
        return records
    return client.list_datasources()


def _browse_record_client(base_client, record):
    org_id = record.get("orgId") or record.get("org_id")
    if org_id:
        return base_client.with_org_id(str(org_id))
    return base_client


def _browse_selected_record(rows, value):
    try:
        selected_index = int(value) - 1
    except ValueError:
        return None, "Unknown command. Use a number, e N, d N, /filter, r, or q."
    if selected_index < 0 or selected_index >= len(rows):
        return None, "Selection out of range."
    return rows[selected_index], None


def _browse_confirm_delete(base_client, record, input_reader, output_writer):
    uid = str(record.get("uid") or "")
    name = str(record.get("name") or "")
    if not uid:
        output_writer("Datasource delete requires a datasource UID.")
        return False
    answer = input_reader("Delete datasource %s (%s)? type yes: " % (uid, name)).strip()
    if answer != "yes":
        output_writer("Cancelled datasource delete.")
        return False
    scoped_client = _browse_record_client(base_client, record)
    target = fetch_datasource_by_uid_if_exists(scoped_client, uid) or record
    target_id = target.get("id") or record.get("id")
    if not target_id:
        raise GrafanaError("Datasource browse delete requires a live datasource id.")
    scoped_client.request_json("/api/datasources/%s" % target_id, method="DELETE")
    output_writer("Deleted datasource %s." % uid)
    return True


def _browse_edit_selected(base_client, record, input_reader, output_writer):
    uid = str(record.get("uid") or "")
    if not uid:
        output_writer("Datasource edit requires a datasource UID.")
        return False
    scoped_client = _browse_record_client(base_client, record)
    existing = fetch_datasource_by_uid_if_exists(scoped_client, uid)
    if existing is None:
        raise GrafanaError("Datasource browse edit could not find UID %s." % uid)
    prompts = (
        ("name", "Name", existing.get("name") or ""),
        ("url", "URL", existing.get("url") or ""),
        ("access", "Access", existing.get("access") or ""),
    )
    updates = {}
    for key, label, current in prompts:
        value = input_reader("%s [%s]: " % (label, current)).strip()
        if value and value != str(current):
            updates[key] = value
    current_default = bool(existing.get("isDefault"))
    default_value = input_reader(
        "Default datasource? [%s]: " % ("yes" if current_default else "no")
    ).strip().lower()
    if default_value in {"yes", "y", "true", "1"} and not current_default:
        updates["isDefault"] = True
    elif default_value in {"no", "n", "false", "0"} and current_default:
        updates["isDefault"] = False
    if not updates:
        output_writer("No datasource changes detected for %s." % uid)
        return False
    payload = build_modify_datasource_payload(existing, updates)
    target_id = payload.get("id")
    if not target_id:
        raise GrafanaError("Datasource browse edit requires a live datasource id.")
    scoped_client.request_json(
        "/api/datasources/%s" % target_id,
        method="PUT",
        payload=payload,
    )
    output_writer("Updated datasource %s." % uid)
    return True


def browse_datasources(args, input_reader=input, output_writer=print, is_tty=None):
    """Browse live datasource inventory in a compact interactive terminal loop."""
    is_tty = is_tty or (lambda: sys.stdin.isatty() and sys.stdout.isatty())
    if not is_tty():
        raise GrafanaError("Datasource browse requires an interactive terminal (TTY).")
    client = build_client(args)
    records = _collect_live_datasource_records(args, client=client)
    if not records:
        output_writer("No datasources matched.")
        return 0
    filtered = list(records)

    def render_rows(rows):
        output_writer("Datasource browse: number=view JSON, e N=edit, d N=delete, /text=filter, r=reset, q=quit.")
        for index, record in enumerate(rows, 1):
            output_writer(
                "%d. %s | %s | %s | org=%s"
                % (
                    index,
                    str(record.get("uid") or "-"),
                    str(record.get("name") or "-"),
                    str(record.get("type") or "-"),
                    str(record.get("org") or record.get("orgId") or "-"),
                )
            )

    render_rows(filtered)
    while True:
        choice = input_reader("datasource> ").strip()
        if choice.lower() in {"q", "quit", "exit"}:
            return 0
        if choice.lower() in {"r", "reset"}:
            filtered = list(records)
            render_rows(filtered)
            continue
        if choice.startswith("/"):
            needle = choice[1:].strip().lower()
            filtered = [
                record
                for record in records
                if needle in json.dumps(record, sort_keys=True).lower()
            ]
            render_rows(filtered)
            continue
        if choice.lower().startswith("e "):
            selected, error = _browse_selected_record(filtered, choice.split(None, 1)[1])
            if error:
                output_writer(error)
                continue
            if _browse_edit_selected(client, selected, input_reader, output_writer):
                records = _collect_live_datasource_records(args, client=client)
                filtered = list(records)
                render_rows(filtered)
            continue
        if choice.lower().startswith("d "):
            selected, error = _browse_selected_record(filtered, choice.split(None, 1)[1])
            if error:
                output_writer(error)
                continue
            if _browse_confirm_delete(client, selected, input_reader, output_writer):
                records = _collect_live_datasource_records(args, client=client)
                filtered = list(records)
                render_rows(filtered)
            continue
        selected, error = _browse_selected_record(filtered, choice)
        if error:
            output_writer(error)
            continue
        output_writer(json.dumps(selected, indent=2, sort_keys=False))


def export_datasources(args):
    """Export datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1697
    #   Downstream callees: 102, 116, 125, 334, 52, 57, 77, 91

    client = build_client(args)
    auth_header = client.headers.get("Authorization", "")
    all_orgs = bool(getattr(args, "all_orgs", False))
    org_id = getattr(args, "org_id", None)
    if (all_orgs or org_id) and not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource org switching does not support API token auth. Use Grafana "
            "username/password login with --basic-user and --basic-password."
        )

    output_dir = Path(args.export_dir)
    clients = []
    if all_orgs:
        for org in client.list_orgs():
            scoped_org_id = _normalize_org_id(org)
            if scoped_org_id:
                clients.append((dict(org), client.with_org_id(scoped_org_id)))
    elif org_id:
        scoped_client = client.with_org_id(str(org_id))
        clients.append((scoped_client.fetch_current_org(), scoped_client))
    else:
        clients.append((client.fetch_current_org(), client))

    total_count = 0
    index_items = []
    export_targets = []
    for org, scoped_client in clients:
        scoped_output_dir = output_dir
        if all_orgs:
            scoped_output_dir = build_all_orgs_output_dir(output_dir, org)
        records = build_export_records(scoped_client)
        datasources_path = scoped_output_dir / DATASOURCE_EXPORT_FILENAME
        provisioning_path = (
            scoped_output_dir
            / DATASOURCE_PROVISIONING_SUBDIR
            / DATASOURCE_PROVISIONING_FILENAME
        )
        index_path = scoped_output_dir / "index.json"
        metadata_path = scoped_output_dir / EXPORT_METADATA_FILENAME
        existing_paths = [
            path
            for path in [datasources_path, provisioning_path, index_path, metadata_path]
            if path.exists()
        ]
        if existing_paths and not args.overwrite:
            raise GrafanaError(
                "Refusing to overwrite existing file: %s. Use --overwrite."
                % existing_paths[0]
            )
        total_count += len(records)
        export_targets.append(
            {
                "org": dict(org),
                "records": records,
                "datasources_path": datasources_path,
                "provisioning_path": provisioning_path,
                "index_path": index_path,
                "metadata_path": metadata_path,
            }
        )
        if all_orgs:
            for item in build_export_index(records, DATASOURCE_EXPORT_FILENAME)[
                "items"
            ]:
                aggregate_item = dict(item)
                aggregate_item["exportDir"] = str(scoped_output_dir)
                index_items.append(aggregate_item)

    if all_orgs:
        root_index_path = output_dir / "index.json"
        root_metadata_path = output_dir / EXPORT_METADATA_FILENAME
        existing_paths = [
            path for path in [root_index_path, root_metadata_path] if path.exists()
        ]
        if existing_paths and not args.overwrite:
            raise GrafanaError(
                "Refusing to overwrite existing file: %s. Use --overwrite."
                % existing_paths[0]
            )
    if not args.dry_run:
        for item in export_targets:
            write_json_document(item["records"], item["datasources_path"])
            if not bool(getattr(args, "without_datasource_provisioning", False)):
                item["provisioning_path"].parent.mkdir(parents=True, exist_ok=True)
                item["provisioning_path"].write_text(
                    yaml.safe_dump(
                        build_datasource_provisioning_document(item["records"])
                    ),
                    encoding="utf-8",
                )
            write_json_document(
                build_export_index(item["records"], DATASOURCE_EXPORT_FILENAME),
                item["index_path"],
            )
            write_json_document(
                build_export_metadata(
                    datasource_count=len(item["records"]),
                    datasources_file=DATASOURCE_EXPORT_FILENAME,
                ),
                item["metadata_path"],
            )
        if all_orgs:
            write_json_document(
                build_all_orgs_export_index(index_items),
                output_dir / "index.json",
            )
            write_json_document(
                build_all_orgs_export_metadata(
                    org_count=len(export_targets),
                    datasource_count=total_count,
                ),
                output_dir / EXPORT_METADATA_FILENAME,
            )
    summary_verb = "Would export" if args.dry_run else "Exported"
    if all_orgs:
        print(
            "%s %s datasource(s) across %s org(s). Root index: %s Manifest: %s"
            % (
                summary_verb,
                total_count,
                len(export_targets),
                output_dir / "index.json",
                output_dir / EXPORT_METADATA_FILENAME,
            )
        )
        return 0
    target = export_targets[0]
    print(
        "%s %s datasource(s). Datasources: %s Provisioning: %s Index: %s Manifest: %s"
        % (
            summary_verb,
            len(target["records"]),
            target["datasources_path"],
            (
                "skipped"
                if bool(getattr(args, "without_datasource_provisioning", False))
                else target["provisioning_path"]
            ),
            target["index_path"],
            target["metadata_path"],
        )
    )
    return 0


def _serialize_datasource_diff_record(record):
    """Internal helper for serialize datasource diff record."""
    if record is None:
        return "{}"
    return json.dumps(record, sort_keys=True, indent=2)


def _print_datasource_unified_diff(
    remote_record, local_record, remote_label, local_label
):
    """Internal helper for print datasource unified diff."""
    remote_lines = _serialize_datasource_diff_record(remote_record).splitlines(True)
    local_lines = _serialize_datasource_diff_record(local_record).splitlines(True)
    for line in difflib.unified_diff(
        remote_lines,
        local_lines,
        fromfile=remote_label,
        tofile=local_label,
    ):
        sys.stdout.write(line)
    if remote_lines or local_lines:
        sys.stdout.write("\n")


def diff_datasources(args):
    """Diff datasources implementation."""
    client = build_client(args)
    bundle = load_local_datasource_bundle(Path(args.diff_dir), getattr(args, "input_format", "inventory"))
    live_records = build_live_datasource_diff_records(client)
    report = compare_datasource_bundle_to_live(bundle, live_records)
    diff_dir = Path(args.diff_dir)

    if getattr(args, "output_format", "text") == "json":
        print(
            json.dumps(
                {
                    "diffCount": report["summary"]["diffCount"],
                    "bundleCount": report["summary"]["bundleCount"],
                    "items": report["items"],
                    "summary": report["summary"],
                },
                indent=2,
                sort_keys=False,
            )
        )
        return 1 if report["summary"]["diffCount"] else 0

    for item in report["items"]:
        identity = item["identity"]
        status = item["status"]
        if status == "match":
            print("Diff same %s -> datasource=%s" % (diff_dir, identity))
            continue
        if status == "different":
            print(
                "Diff different %s -> datasource=%s fields=%s"
                % (diff_dir, identity, ",".join(item["changedFields"]))
            )
        elif status == "missing-live":
            print("Diff missing-live %s -> datasource=%s" % (diff_dir, identity))
        elif status == "extra-live":
            print("Diff extra-live %s -> datasource=%s" % (diff_dir, identity))
        elif status == "ambiguous-live-uid":
            print("Diff ambiguous-live-uid %s -> datasource=%s" % (diff_dir, identity))
        elif status == "ambiguous-live-name":
            print("Diff ambiguous-live-name %s -> datasource=%s" % (diff_dir, identity))
        else:
            print("Diff %s %s -> datasource=%s" % (status, diff_dir, identity))
        _print_datasource_unified_diff(
            item.get("live"),
            item.get("local"),
            "remote/%s" % identity,
            "local/%s" % identity,
        )

    diff_count = report["summary"]["diffCount"]
    bundle_count = report["summary"]["bundleCount"]
    print(
        "Diff checked %s datasource(s); %s difference(s) found."
        % (bundle_count, diff_count)
    )
    if diff_count:
        print(
            "Found %s datasource difference(s) across %s exported datasource(s)."
            % (diff_count, bundle_count)
        )
        return 1

    print("No datasource differences across %s exported datasource(s)." % bundle_count)
    return 0


def _resolve_multi_org_import_targets(args, client):
    """Internal helper for resolve multi org import targets."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1446
    #   Downstream callees: 217, 284, 309, 352, 362, 375, 380

    import_dir = Path(args.import_dir)
    selected_org_ids = set(
        str(item).strip()
        for item in (getattr(args, "only_org_id", None) or [])
        if str(item).strip()
    )
    create_missing_orgs = bool(getattr(args, "create_missing_orgs", False))
    dry_run = bool(getattr(args, "dry_run", False))
    orgs_by_id = _resolve_existing_orgs_by_id(client)
    targets = []
    matched_source_org_ids = set()
    bundle_loader = load_import_bundle_preview if dry_run else load_import_bundle
    for org_dir in _discover_org_export_dirs(import_dir):
        bundle = bundle_loader(org_dir)
        source_org_id = resolve_export_org_id(bundle)
        if not source_org_id:
            raise GrafanaError(
                "Could not determine one source export orgId from %s while "
                "--use-export-org is active." % org_dir
            )
        if selected_org_ids and source_org_id not in selected_org_ids:
            continue
        matched_source_org_ids.add(source_org_id)
        source_org_name = resolve_export_org_name(bundle) or ""
        target_org_id = source_org_id
        org_action = "exists"
        created_org = False
        preview_only = False
        if source_org_id not in orgs_by_id:
            if dry_run:
                preview_only = True
                if create_missing_orgs:
                    org_action = "would-create-org"
                    target_org_id = "<new>"
                else:
                    org_action = "missing-org"
                    target_org_id = ""
            elif not create_missing_orgs:
                raise GrafanaError(
                    "Export orgId %s was not found in the destination Grafana org list. "
                    "Use --create-missing-orgs to create it from the export metadata."
                    % source_org_id
                )
            elif not source_org_name:
                raise GrafanaError(
                    "Cannot create missing destination org for export orgId %s because "
                    "the datasource export does not contain one stable org name."
                    % source_org_id
                )
            else:
                created_payload = create_organization(client, source_org_name)
                target_org_id = _resolve_created_org_id(created_payload)
                if not target_org_id:
                    raise GrafanaError(
                        "Created organization for export orgId %s did not return a usable id."
                        % source_org_id
                    )
                orgs_by_id[target_org_id] = {
                    "id": target_org_id,
                    "name": source_org_name,
                }
                created_org = True
                org_action = "created-org"
        targets.append(
            {
                "bundle_dir": org_dir,
                "bundle": bundle,
                "source_org_id": source_org_id,
                "source_org_name": source_org_name,
                "target_org_id": target_org_id,
                "org_action": org_action,
                "created_org": created_org,
                "preview_only": preview_only,
                "datasource_count": len(bundle["records"]),
            }
        )
    if selected_org_ids:
        missing_org_ids = sorted(selected_org_ids - matched_source_org_ids)
        if missing_org_ids:
            raise GrafanaError(
                "Selected export orgIds were not found in %s: %s"
                % (import_dir, ", ".join(missing_org_ids))
            )
    if not targets:
        raise GrafanaError(
            "No org-scoped datasource exports matched %s under %s."
            % (
                (
                    "--only-org-id selection"
                    if selected_org_ids
                    else "the combined multi-org export root"
                ),
                import_dir,
            )
        )
    return targets


def _render_routed_datasource_import_table(args, targets):
    """Internal helper for render routed datasource import table."""
    headers = [
        "SOURCE_ORG_ID",
        "SOURCE_ORG_NAME",
        "ORG_ACTION",
        "TARGET_ORG_ID",
        "DATASOURCE_COUNT",
    ]
    rows = [
        [
            str(item["source_org_id"]),
            item["source_org_name"] or "-",
            item["org_action"],
            item["target_org_id"] or "-",
            str(item["datasource_count"]),
        ]
        for item in targets
    ]
    widths = [len(item) for item in headers]
    for row in rows:
        for index, value in enumerate(row):
            widths[index] = max(widths[index], len(value))

    def render_row(values):
        # Purpose: implementation note.
        # Args: see function signature.
        # Returns: see implementation.

        return "  ".join(
            ["%-*s" % (widths[index], value) for index, value in enumerate(values)]
        )

    lines = []
    if not bool(getattr(args, "no_header", False)):
        lines.append(render_row(headers))
        lines.append(render_row(["-" * width for width in widths]))
    for row in rows:
        lines.append(render_row(row))
    return lines


def _run_import_datasources_by_export_org(args, client):
    """Internal helper for run import datasources by export org."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1690
    #   Downstream callees: 1303, 1401, 1561, 345

    auth_header = client.headers.get("Authorization", "")
    if not auth_header.startswith("Basic "):
        raise GrafanaError(
            "Datasource import with --use-export-org does not support API token auth. "
            "Use Grafana username/password login with --basic-user and --basic-password."
        )
    targets = _resolve_multi_org_import_targets(args, client)
    if bool(getattr(args, "dry_run", False)) and bool(getattr(args, "json", False)):
        org_entries = []
        import_entries = []
        for target in targets:
            org_entry = {
                "sourceOrgId": target["source_org_id"],
                "sourceOrgName": target["source_org_name"],
                "orgAction": target["org_action"],
                "targetOrgId": target["target_org_id"] or "",
                "datasourceCount": target["datasource_count"],
                "importDir": str(target["bundle_dir"]),
            }
            org_entries.append(org_entry)
            import_entry = dict(org_entry)
            import_entry.update(
                {
                    "mode": None,
                    "datasources": [],
                    "summary": {
                        "datasourceCount": target["datasource_count"],
                        "importDir": str(target["bundle_dir"]),
                    },
                }
            )
            if not target["preview_only"]:
                scoped_args = _clone_import_args(
                    args,
                    import_dir=str(target["bundle_dir"]),
                    org_id=target["target_org_id"],
                    use_export_org=False,
                    only_org_id=None,
                    create_missing_orgs=False,
                    require_matching_export_org=False,
                )
                stream = StringIO()
                with redirect_stdout(stream):
                    _run_import_datasources_for_single_org(scoped_args)
                import_entry.update(json.loads(stream.getvalue()))
            import_entries.append(import_entry)
        summary = {
            "orgCount": len(org_entries),
            "existingOrgCount": len(
                [item for item in org_entries if item["orgAction"] == "exists"]
            ),
            "missingOrgCount": len(
                [item for item in org_entries if item["orgAction"] == "missing-org"]
            ),
            "wouldCreateOrgCount": len(
                [
                    item
                    for item in org_entries
                    if item["orgAction"] == "would-create-org"
                ]
            ),
            "datasourceCount": sum([item["datasourceCount"] for item in org_entries]),
        }
        print(
            json.dumps(
                {
                    "mode": "routed-import-preview",
                    "orgs": org_entries,
                    "imports": import_entries,
                    "summary": summary,
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    if bool(getattr(args, "dry_run", False)) and bool(getattr(args, "table", False)):
        for line in _render_routed_datasource_import_table(args, targets):
            print(line)
        return 0
    for target in targets:
        if bool(getattr(args, "dry_run", False)):
            print(
                "Dry-run export orgId=%s name=%s orgAction=%s targetOrgId=%s datasources=%s from %s"
                % (
                    target["source_org_id"],
                    target["source_org_name"] or "-",
                    target["org_action"],
                    target["target_org_id"] or "-",
                    target["datasource_count"],
                    target["bundle_dir"],
                )
            )
            if target["preview_only"]:
                continue
        elif target["created_org"]:
            print(
                "Created destination org from export orgId=%s name=%s -> targetOrgId=%s"
                % (
                    target["source_org_id"],
                    target["source_org_name"] or "-",
                    target["target_org_id"],
                )
            )
        scoped_args = _clone_import_args(
            args,
            import_dir=str(target["bundle_dir"]),
            org_id=target["target_org_id"],
            use_export_org=False,
            only_org_id=None,
            create_missing_orgs=False,
            require_matching_export_org=False,
        )
        _run_import_datasources_for_single_org(scoped_args)
    return 0


def _run_import_datasources_for_single_org(args):
    """Internal helper for run import datasources for single org."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1446, 1690
    #   Downstream callees: 217, 411, 425, 448, 462, 481, 490, 52, 521, 564, 596

    if getattr(args, "table", False) and not args.dry_run:
        raise GrafanaError(
            "--table is only supported with --dry-run for datasource import."
        )
    if getattr(args, "json", False) and not args.dry_run:
        raise GrafanaError(
            "--json is only supported with --dry-run for datasource import."
        )
    if getattr(args, "table", False) and getattr(args, "json", False):
        raise GrafanaError(
            "--table and --json are mutually exclusive for datasource import."
        )
    if getattr(args, "no_header", False) and not getattr(args, "table", False):
        raise GrafanaError(
            "--no-header is only supported with --dry-run --table for datasource import."
        )
    client = build_effective_import_client(args, build_client(args))
    if getattr(args, "input_format", "inventory") == "provisioning":
        if getattr(args, "use_export_org", False):
            raise GrafanaError("--use-export-org is only supported with inventory datasource import.")
        bundle = load_provisioning_datasource_bundle(Path(args.import_dir))
    else:
        bundle_loader = load_import_bundle_preview if args.dry_run else load_import_bundle
        bundle = bundle_loader(Path(args.import_dir))
    secret_values = load_secret_value_map(args)
    if secret_values is not None:
        bundle["records"] = [
            apply_secret_placeholders_to_record(record, secret_values)
            for record in bundle["records"]
        ]
    target_org_id = validate_export_org_match(args, client, bundle)
    lookups = build_existing_datasource_lookups(client)
    mode = determine_import_mode(args)
    records = []
    imported_count = 0
    skipped_missing_count = 0
    total = len(bundle["records"])
    if not getattr(args, "json", False):
        print("Import mode: %s" % mode)
    for index, record in enumerate(bundle["records"], 1):
        match = resolve_datasource_match(record, lookups)
        action = determine_datasource_action(args, record, match)
        dry_run_record = {
            "uid": record.get("uid") or "",
            "name": record.get("name") or "",
            "type": record.get("type") or "",
            "destination": match["state"],
            "action": action,
            "orgId": target_org_id,
            "sourceOrgId": record.get("orgId") or "",
            "file": "%s#%s" % (bundle["datasources_path"], index - 1),
            "secretSummary": record.get("secretSummary") or "",
            "secretFields": list(record.get("secretFields") or []),
            "secretPlaceholderNames": list(record.get("secretPlaceholderNames") or []),
            "providerNames": list(record.get("providerNames") or []),
        }
        if args.dry_run:
            records.append(dry_run_record)
            if getattr(args, "table", False) or getattr(args, "json", False):
                continue
            print(
                "Dry-run datasource uid=%s name=%s dest=%s action=%s file=%s%s"
                % (
                    dry_run_record["uid"] or "-",
                    dry_run_record["name"] or "-",
                    dry_run_record["destination"],
                    dry_run_record["action"],
                    dry_run_record["file"],
                    (
                        " secret=%s" % dry_run_record["secretSummary"]
                        if dry_run_record["secretSummary"]
                        else ""
                    ),
                )
            )
            continue
        if action == "would-skip-missing":
            skipped_missing_count += 1
            if getattr(args, "verbose", False):
                print(
                    "Skipped datasource uid=%s name=%s dest=missing action=skip-missing"
                    % (record.get("uid") or "-", record.get("name") or "-")
                )
            elif getattr(args, "progress", False):
                print(
                    "Skipping datasource %s/%s: %s"
                    % (index, total, record.get("uid") or record.get("name") or "-")
                )
            continue
        if action in (
            "would-fail-existing",
            "would-fail-ambiguous",
            "would-fail-plugin-type-change",
            "would-fail-uid-mismatch",
        ):
            raise GrafanaError(
                "Datasource import blocked for uid=%s name=%s action=%s"
                % (record.get("uid") or "-", record.get("name") or "-", action)
            )
        payload = build_import_payload(record, match.get("target"))
        if action == "would-update":
            datasource_id = payload.get("id")
            if datasource_id is None:
                raise GrafanaError(
                    "Datasource import could not determine destination datasource id for update."
                )
            client.request_json(
                "/api/datasources/%s" % datasource_id,
                method="PUT",
                payload=payload,
            )
        else:
            client.request_json("/api/datasources", method="POST", payload=payload)
        imported_count += 1
        if getattr(args, "verbose", False):
            print(
                "Imported datasource uid=%s name=%s action=%s"
                % (
                    record.get("uid") or "-",
                    record.get("name") or "-",
                    "update" if action == "would-update" else "create",
                )
            )
        elif getattr(args, "progress", False):
            print(
                "Importing datasource %s/%s: %s"
                % (index, total, record.get("uid") or record.get("name") or "-")
            )
    if args.dry_run:
        if getattr(args, "json", False):
            print(render_import_dry_run_json(mode, records, target_org_id))
            return 0
        if getattr(args, "table", False):
            for line in render_import_dry_run_table(
                records,
                include_header=not bool(getattr(args, "no_header", False)),
                selected_columns=getattr(args, "output_columns", None),
            ):
                print(line)
        print(
            "Dry-run checked %s datasource(s) from %s" % (len(records), args.import_dir)
        )
        return 0
    if skipped_missing_count:
        print(
            "Imported %s datasource(s) from %s; skipped %s missing destination datasources"
            % (imported_count, args.import_dir, skipped_missing_count)
        )
    else:
        print("Imported %s datasource(s) from %s" % (imported_count, args.import_dir))
    return 0


def import_datasources(args):
    """Import datasources implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 1697
    #   Downstream callees: 1446, 1561, 52

    if bool(getattr(args, "use_export_org", False)):
        return _run_import_datasources_by_export_org(args, build_client(args))
    return _run_import_datasources_for_single_org(args)


def dispatch_datasource_command(args):
    """Dispatch datasource command implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 1044, 1102, 1250, 1690, 880, 934, 994

    if args.command == "types":
        document = build_supported_datasource_catalog_document()
        if getattr(args, "json", False):
            print(json.dumps(document, indent=2))
            return 0
        if getattr(args, "yaml", False):
            print(yaml.safe_dump(document), end="")
            return 0
        if getattr(args, "csv", False):
            print(render_supported_datasource_catalog_csv(), end="")
            return 0
        if getattr(args, "table", False):
            for line in render_supported_datasource_catalog_table():
                print(line)
            return 0
        for line in render_supported_datasource_catalog_text():
            print(line)
        return 0
    if args.command == "list":
        return list_datasources(args)
    if args.command == "browse":
        return browse_datasources(args)
    if args.command == "export":
        return export_datasources(args)
    if args.command == "import":
        return import_datasources(args)
    if args.command == "add":
        return add_datasource(args)
    if args.command == "modify":
        return modify_datasource(args)
    if args.command == "delete":
        return delete_datasource(args)
    return diff_datasources(args)
