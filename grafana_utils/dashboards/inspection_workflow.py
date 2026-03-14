"""Dashboard inspection workflow orchestration helpers."""

import argparse
import json
import sys
import tempfile
from pathlib import Path


INSPECT_OUTPUT_FORMAT_TO_MODE = {
    "text": (None, False, False),
    "table": (None, False, True),
    "json": (None, True, False),
    "report-table": ("table", False, False),
    "report-csv": ("csv", False, False),
    "report-json": ("json", False, False),
    "report-tree": ("tree", False, False),
    "report-tree-table": ("tree-table", False, False),
    "governance": ("governance", False, False),
    "governance-json": ("governance-json", False, False),
}


def resolve_inspect_output_mode(args, grafana_error):
    """Normalize legacy inspect output flags and the new --output-format selector."""
    output_format = getattr(args, "output_format", None)
    report_format = getattr(args, "report", None)
    json_output = bool(getattr(args, "json", False))
    table_output = bool(getattr(args, "table", False))

    if output_format and (report_format or json_output or table_output):
        raise grafana_error(
            "--output-format cannot be combined with --json, --table, or --report."
        )
    if output_format:
        return INSPECT_OUTPUT_FORMAT_TO_MODE[output_format]
    return report_format, json_output, table_output


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
    import_dir = Path(args.import_dir)
    grafana_error = deps["GrafanaError"]
    report_format, json_output, table_output = resolve_inspect_output_mode(
        args, grafana_error
    )
    if report_format and (table_output or json_output):
        raise grafana_error("--report cannot be combined with --table or --json.")
    report_columns = deps["parse_report_columns"](
        getattr(args, "report_columns", None)
    )
    report_filter_datasource = getattr(args, "report_filter_datasource", None)
    report_filter_panel_id = getattr(args, "report_filter_panel_id", None)
    if table_output and json_output:
        raise grafana_error(
            "--table and --json are mutually exclusive for inspect-export."
        )
    if report_columns is not None and report_format is None:
        raise grafana_error(
            "--report-columns is only supported with --report or report-like --output-format."
        )
    if report_filter_datasource and report_format is None:
        raise grafana_error(
            "--report-filter-datasource is only supported with --report or report-like --output-format."
        )
    if report_filter_panel_id and report_format is None:
        raise grafana_error(
            "--report-filter-panel-id is only supported with --report or report-like --output-format."
        )
    if report_columns is not None and report_format in ("governance", "governance-json"):
        raise grafana_error(
            "--report-columns is not supported with governance output."
        )
    if report_columns is not None and report_format not in ("table", "csv", "tree-table"):
        raise grafana_error(
            "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes."
        )
    if getattr(args, "no_header", False) and not (
        table_output
        or report_format in ("table", "csv", "tree-table")
    ):
        raise grafana_error(
            "--no-header is only supported with --table, table-like --report, or compatible --output-format values."
        )
    if report_format == "json":
        document = deps["filter_export_inspection_report_document"](
            deps["build_export_inspection_report_document"](import_dir),
            datasource_label=report_filter_datasource,
            panel_id=report_filter_panel_id,
        )
        print(
            json.dumps(
                document,
                indent=2,
                sort_keys=False,
                ensure_ascii=False,
            )
        )
        return 0
    if report_format == "table":
        document = deps["filter_export_inspection_report_document"](
            deps["build_export_inspection_report_document"](import_dir),
            datasource_label=report_filter_datasource,
            panel_id=report_filter_panel_id,
        )
        for line in deps["render_export_inspection_report_tables"](
            document,
            import_dir,
            include_header=not bool(getattr(args, "no_header", False)),
            selected_columns=report_columns,
        ):
            print(line)
        return 0
    if report_format == "csv":
        document = deps["filter_export_inspection_report_document"](
            deps["build_export_inspection_report_document"](import_dir),
            datasource_label=report_filter_datasource,
            panel_id=report_filter_panel_id,
        )
        sys.stdout.write(
            deps["render_export_inspection_report_csv"](
                document,
                selected_columns=report_columns,
                include_header=not bool(getattr(args, "no_header", False)),
            )
        )
        return 0
    if report_format == "tree":
        document = deps["build_grouped_export_inspection_report_document"](
            deps["filter_export_inspection_report_document"](
                deps["build_export_inspection_report_document"](import_dir),
                datasource_label=report_filter_datasource,
                panel_id=report_filter_panel_id,
            )
        )
        for line in deps["render_export_inspection_grouped_report"](
            document,
            import_dir,
        ):
            print(line)
        return 0
    if report_format == "tree-table":
        document = deps["build_grouped_export_inspection_report_document"](
            deps["filter_export_inspection_report_document"](
                deps["build_export_inspection_report_document"](import_dir),
                datasource_label=report_filter_datasource,
                panel_id=report_filter_panel_id,
            )
        )
        for line in deps["render_export_inspection_tree_tables"](
            document,
            import_dir,
            include_header=not bool(getattr(args, "no_header", False)),
            selected_columns=report_columns,
        ):
            print(line)
        return 0
    if report_format in ("governance", "governance-json"):
        report_document = deps["filter_export_inspection_report_document"](
            deps["build_export_inspection_report_document"](import_dir),
            datasource_label=report_filter_datasource,
            panel_id=report_filter_panel_id,
        )
        document = deps["build_export_inspection_governance_document"](
            deps["build_export_inspection_document"](import_dir),
            report_document,
        )
        if report_format == "governance-json":
            print(
                json.dumps(
                    document,
                    indent=2,
                    sort_keys=False,
                    ensure_ascii=False,
                )
            )
            return 0
        for line in deps["render_export_inspection_governance_tables"](
            document,
            import_dir,
        ):
            print(line)
        return 0
    document = deps["build_export_inspection_document"](import_dir)
    if json_output:
        print(
            json.dumps(
                document, indent=2, sort_keys=False, ensure_ascii=False
            )
        )
        return 0
    if table_output:
        for line in deps["render_export_inspection_tables"](
            document,
            import_dir,
            include_header=not bool(getattr(args, "no_header", False)),
        ):
            print(line)
        return 0
    for line in deps["render_export_inspection_summary"](document, import_dir):
        print(line)
    return 0
