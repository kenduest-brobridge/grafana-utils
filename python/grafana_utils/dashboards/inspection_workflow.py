"""Dashboard inspection workflow orchestration helpers."""

import argparse
import json
import shutil
import tempfile
from pathlib import Path

from .inspection_dispatch import (
    resolve_inspect_dispatch_args,
    run_inspection_dispatch,
)


def _load_json_array_file(path, error_cls, error_context):
    """Load one JSON array file for merged multi-org inspection."""
    if not path.is_file():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise error_cls("Failed to read %s %s: %s" % (error_context, path, exc)) from exc
    except json.JSONDecodeError as exc:
        raise error_cls("Invalid JSON in %s %s: %s" % (error_context, path, exc)) from exc
    if not isinstance(raw, list):
        raise error_cls("%s must be a JSON array: %s" % (error_context, path))
    return raw


def _merge_multi_org_export_root(import_dir, temp_root, deps):
    """Materialize one merged raw inspect directory from a multi-org export root."""
    inspect_raw_dir = temp_root / "inspect-export-all-orgs" / deps["RAW_EXPORT_SUBDIR"]
    merged_index = []
    merged_folders = []
    merged_datasources = []
    dashboard_count = 0

    for org_raw_dir in deps["discover_org_raw_export_dirs"](import_dir):
        org_name = org_raw_dir.parent.name
        shutil.copytree(org_raw_dir, inspect_raw_dir / org_name, dirs_exist_ok=True)
        metadata = deps["load_export_metadata"](
            org_raw_dir, expected_variant=deps["RAW_EXPORT_SUBDIR"]
        )
        index_file = str((metadata or {}).get("indexFile") or "index.json")
        index_records = _load_json_array_file(
            org_raw_dir / index_file,
            deps["GrafanaError"],
            "Dashboard export index",
        )
        for item in index_records:
            if isinstance(item, dict):
                merged = dict(item)
                merged["path"] = "%s/%s" % (org_name, str(item.get("path") or ""))
                merged_index.append(merged)
        merged_folders.extend(
            _load_json_array_file(
                org_raw_dir / deps["FOLDER_INVENTORY_FILENAME"],
                deps["GrafanaError"],
                "Dashboard folder inventory",
            )
        )
        merged_datasources.extend(
            _load_json_array_file(
                org_raw_dir / deps["DATASOURCE_INVENTORY_FILENAME"],
                deps["GrafanaError"],
                "Dashboard datasource inventory",
            )
        )
        dashboard_count += int(
            (metadata or {}).get("dashboardCount") or len(index_records)
        )

    deps["write_json_document"](
        deps["build_export_metadata"](
            variant=deps["RAW_EXPORT_SUBDIR"],
            dashboard_count=dashboard_count,
            format_name="grafana-web-import-preserve-uid",
            folders_file=deps["FOLDER_INVENTORY_FILENAME"],
            datasources_file=deps["DATASOURCE_INVENTORY_FILENAME"],
        ),
        inspect_raw_dir / deps["EXPORT_METADATA_FILENAME"],
    )
    deps["write_json_document"](merged_index, inspect_raw_dir / "index.json")
    deps["write_json_document"](
        merged_folders, inspect_raw_dir / deps["FOLDER_INVENTORY_FILENAME"]
    )
    deps["write_json_document"](
        merged_datasources, inspect_raw_dir / deps["DATASOURCE_INVENTORY_FILENAME"]
    )
    (inspect_raw_dir / ".inspect-source-root").write_text(
        str(import_dir),
        encoding="utf-8",
    )
    return inspect_raw_dir


def materialize_live_inspection_export(client, page_size, raw_dir, deps):
    """Write one temporary raw-export-like directory for live dashboard inspection."""
    raw_dir.mkdir(parents=True, exist_ok=True)
    summaries = deps["attach_dashboard_org"](
        client, client.iter_dashboard_summaries(page_size)
    )
    org = client.fetch_current_org()
    folder_inventory = deps["collect_folder_inventory"](client, org, summaries)
    datasource_inventory = [
        deps["build_datasource_inventory_record"](item, org)
        for item in client.list_datasources()
    ]
    index_items = []
    for summary in summaries:
        uid = str(summary.get("uid") or "").strip()
        if not uid:
            continue
        payload = client.fetch_dashboard(uid)
        document = deps["build_preserved_web_import_document"](payload)
        output_path = deps["build_output_path"](raw_dir, summary, flat=False)
        deps["write_dashboard"](document, output_path, overwrite=True)
        item = deps["build_dashboard_index_item"](summary, uid)
        item["raw_path"] = str(output_path)
        index_items.append(item)

    raw_index = deps["build_variant_index"](
        index_items,
        "raw_path",
        "grafana-web-import-preserve-uid",
    )
    raw_metadata = deps["build_export_metadata"](
        variant=deps["RAW_EXPORT_SUBDIR"],
        dashboard_count=len(raw_index),
        format_name="grafana-web-import-preserve-uid",
        folders_file=deps["FOLDER_INVENTORY_FILENAME"],
        datasources_file=deps["DATASOURCE_INVENTORY_FILENAME"],
    )
    deps["write_json_document"](raw_index, raw_dir / "index.json")
    deps["write_json_document"](
        raw_metadata, raw_dir / deps["EXPORT_METADATA_FILENAME"]
    )
    deps["write_json_document"](
        folder_inventory, raw_dir / deps["FOLDER_INVENTORY_FILENAME"]
    )
    deps["write_json_document"](
        datasource_inventory, raw_dir / deps["DATASOURCE_INVENTORY_FILENAME"]
    )
    return raw_dir


def run_inspect_live(args, deps):
    """Inspect live Grafana dashboards by reusing the raw-export inspection pipeline."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 13, 87

    client = deps["build_client"](args)
    with tempfile.TemporaryDirectory(prefix="grafana-utils-inspect-live-") as tmpdir:
        raw_dir = materialize_live_inspection_export(
            client,
            page_size=int(args.page_size),
            raw_dir=Path(tmpdir) / deps["RAW_EXPORT_SUBDIR"],
            deps=deps,
        )
        inspect_args = argparse.Namespace(
            import_dir=str(raw_dir),
            report=getattr(args, "report", None),
            output_format=getattr(args, "output_format", None),
            report_columns=getattr(args, "report_columns", None),
            report_filter_datasource=getattr(args, "report_filter_datasource", None),
            report_filter_panel_id=getattr(args, "report_filter_panel_id", None),
            json=bool(getattr(args, "json", False)),
            table=bool(getattr(args, "table", False)),
            no_header=bool(getattr(args, "no_header", False)),
        )
        return run_inspect_export(inspect_args, deps)


def run_inspect_export(args, deps):
    """Inspect one raw export directory and summarize dashboards, folders, and datasources."""
    # Call graph: see callers/callees.
    #   Upstream callers: 63
    #   Downstream callees: 無

    import_dir = Path(args.import_dir)
    settings = resolve_inspect_dispatch_args(
        args,
        deps,
        deps["GrafanaError"],
    )
    metadata = deps["load_export_metadata"](import_dir, expected_variant=None)
    if isinstance(metadata, dict) and str(metadata.get("variant") or "") == "root":
        with tempfile.TemporaryDirectory(prefix="grafana-utils-inspect-export-") as tmpdir:
            merged_raw_dir = _merge_multi_org_export_root(
                import_dir,
                Path(tmpdir),
                deps,
            )
            return run_inspection_dispatch(merged_raw_dir, deps, settings)
    return run_inspection_dispatch(import_dir, deps, settings)
