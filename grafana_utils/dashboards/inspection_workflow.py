"""Dashboard inspection workflow orchestration helpers."""

import argparse
import tempfile
from pathlib import Path

from .inspection_dispatch import (
    resolve_inspect_dispatch_args,
    run_inspection_dispatch,
)


def materialize_live_inspection_export(client, page_size, raw_dir, deps):
    """Write one temporary raw-export-like directory for live dashboard inspection."""
    raw_dir.mkdir(parents=True, exist_ok=True)
    summaries = deps.attach_dashboard_org(
        client, client.iter_dashboard_summaries(page_size)
    )
    org = client.fetch_current_org()
    folder_inventory = deps.collect_folder_inventory(client, org, summaries)
    datasource_inventory = [
        deps.build_datasource_inventory_record(item, org)
        for item in client.list_datasources()
    ]
    index_items = []
    for summary in summaries:
        uid = str(summary.get("uid") or "").strip()
        if not uid:
            continue
        payload = client.fetch_dashboard(uid)
        document = deps.build_preserved_web_import_document(payload)
        output_path = deps.build_output_path(raw_dir, summary, flat=False)
        deps.write_dashboard(document, output_path, overwrite=True)
        item = deps.build_dashboard_index_item(summary, uid)
        item["raw_path"] = str(output_path)
        index_items.append(item)

    raw_index = deps.build_variant_index(
        index_items,
        "raw_path",
        "grafana-web-import-preserve-uid",
    )
    raw_metadata = deps.build_export_metadata(
        variant=deps.raw_export_subdir,
        dashboard_count=len(raw_index),
        format_name="grafana-web-import-preserve-uid",
        folders_file=deps.folder_inventory_filename,
        datasources_file=deps.datasource_inventory_filename,
    )
    deps.write_json_document(raw_index, raw_dir / "index.json")
    deps.write_json_document(raw_metadata, raw_dir / deps.export_metadata_filename)
    deps.write_json_document(folder_inventory, raw_dir / deps.folder_inventory_filename)
    deps.write_json_document(
        datasource_inventory, raw_dir / deps.datasource_inventory_filename
    )
    return raw_dir


def run_inspect_live(args, deps):
    """Inspect live Grafana dashboards by reusing the raw-export inspection pipeline."""
    client = deps.build_client(args)
    with tempfile.TemporaryDirectory(prefix="grafana-utils-inspect-live-") as tmpdir:
        raw_dir = materialize_live_inspection_export(
            client,
            page_size=int(args.page_size),
            raw_dir=Path(tmpdir) / deps.raw_export_subdir,
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
    import_dir = Path(args.import_dir)
    settings = resolve_inspect_dispatch_args(
        args,
        deps,
        deps.grafana_error,
    )
    return run_inspection_dispatch(import_dir, deps, settings)
