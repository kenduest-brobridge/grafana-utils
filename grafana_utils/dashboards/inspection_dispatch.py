"""Shared inspect output validation and rendering helpers."""

import json
import sys


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
    "graph-json": ("graph-json", False, False),
    "graph-dot": ("graph-dot", False, False),
    "graph-governance": ("graph-governance", False, False),
    "graph-governance-json": ("graph-governance-json", False, False),
}

REPORT_FORMATS_WITH_COLUMN_SUPPORT = ("table", "csv", "tree-table")
GOVERNANCE_REPORT_FORMATS = ("governance", "governance-json")
GRAPH_REPORT_FORMATS = (
    "graph-json",
    "graph-dot",
    "graph-governance",
    "graph-governance-json",
)


class InspectionDispatchSettings(object):
    """Normalized inspect dispatch settings."""

    def __init__(
        self,
        json_output,
        no_header,
        report_columns,
        report_filter_datasource,
        report_filter_panel_id,
        report_format,
        table_output,
    ):
        self.json_output = json_output
        self.no_header = no_header
        self.report_columns = report_columns
        self.report_filter_datasource = report_filter_datasource
        self.report_filter_panel_id = report_filter_panel_id
        self.report_format = report_format
        self.table_output = table_output


def resolve_inspect_output_mode(args, grafana_error):
    """Normalize legacy inspect output flags and the newer --output-format selector."""
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


def resolve_inspect_dispatch_args(args, deps, grafana_error):
    """Validate inspect output arguments and normalize dispatch settings."""
    report_format, json_output, table_output = resolve_inspect_output_mode(
        args, grafana_error
    )
    if report_format and (table_output or json_output):
        raise grafana_error("--report cannot be combined with --table or --json.")
    if table_output and json_output:
        raise grafana_error(
            "--table and --json are mutually exclusive for inspect-export."
        )

    report_columns = deps.parse_report_columns(getattr(args, "report_columns", None))
    report_filter_datasource = getattr(args, "report_filter_datasource", None)
    report_filter_panel_id = getattr(args, "report_filter_panel_id", None)
    no_header = bool(getattr(args, "no_header", False))

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
    if report_columns is not None and report_format in GOVERNANCE_REPORT_FORMATS:
        raise grafana_error("--report-columns is not supported with governance output.")
    if (
        report_columns is not None
        and report_format not in REPORT_FORMATS_WITH_COLUMN_SUPPORT
    ):
        raise grafana_error(
            "--report-columns is only supported with report-table, report-csv, report-tree-table, or the equivalent --report modes."
        )
    if no_header and not (
        table_output or report_format in REPORT_FORMATS_WITH_COLUMN_SUPPORT
    ):
        raise grafana_error(
            "--no-header is only supported with --table, table-like --report, or compatible --output-format values."
        )

    return InspectionDispatchSettings(
        json_output=json_output,
        no_header=no_header,
        report_columns=report_columns,
        report_filter_datasource=report_filter_datasource,
        report_filter_panel_id=report_filter_panel_id,
        report_format=report_format,
        table_output=table_output,
    )


def build_filtered_report_document(import_dir, deps, settings):
    """Build one filtered flat query report document for inspect output."""
    return deps.filter_export_inspection_report_document(
        deps.build_export_inspection_report_document(import_dir),
        datasource_label=settings.report_filter_datasource,
        panel_id=settings.report_filter_panel_id,
    )


def _render_report_output(import_dir, deps, settings):
    report_format = settings.report_format
    report_columns = settings.report_columns
    include_header = not settings.no_header

    if report_format == "json":
        print(
            json.dumps(
                build_filtered_report_document(import_dir, deps, settings),
                indent=2,
                sort_keys=False,
                ensure_ascii=False,
            )
        )
        return 0

    if report_format == "table":
        for line in deps.render_export_inspection_report_tables(
            build_filtered_report_document(import_dir, deps, settings),
            import_dir,
            include_header=include_header,
            selected_columns=report_columns,
        ):
            print(line)
        return 0

    if report_format == "csv":
        sys.stdout.write(
            deps.render_export_inspection_report_csv(
                build_filtered_report_document(import_dir, deps, settings),
                selected_columns=report_columns,
                include_header=include_header,
            )
        )
        return 0

    if report_format in ("tree", "tree-table"):
        grouped_document = deps.build_grouped_export_inspection_report_document(
            build_filtered_report_document(import_dir, deps, settings)
        )
        if report_format == "tree":
            for line in deps.render_export_inspection_grouped_report(
                grouped_document,
                import_dir,
            ):
                print(line)
            return 0
        for line in deps.render_export_inspection_tree_tables(
            grouped_document,
            import_dir,
            include_header=include_header,
            selected_columns=report_columns,
        ):
            print(line)
        return 0

    report_document = build_filtered_report_document(import_dir, deps, settings)
    if report_format in GRAPH_REPORT_FORMATS:
        graph_document = deps.build_dependency_graph_document(
            deps.build_export_inspection_document(import_dir),
            report_document,
        )
        if report_format == "graph-json":
            print(
                json.dumps(
                    graph_document, indent=2, sort_keys=False, ensure_ascii=False
                )
            )
            return 0
        if report_format == "graph-dot":
            print(deps.render_dependency_graph_dot(graph_document))
            return 0
        if report_format == "graph-governance-json":
            print(
                json.dumps(
                    deps.build_dependency_graph_governance_summary(graph_document),
                    indent=2,
                    sort_keys=False,
                    ensure_ascii=False,
                )
            )
            return 0
        for line in deps.render_dependency_graph_governance_text(graph_document):
            print(line)
        return 0
    governance_document = deps.build_export_inspection_governance_document(
        deps.build_export_inspection_document(import_dir),
        report_document,
    )
    if report_format == "governance-json":
        print(
            json.dumps(
                governance_document,
                indent=2,
                sort_keys=False,
                ensure_ascii=False,
            )
        )
        return 0
    for line in deps.render_export_inspection_governance_tables(
        governance_document,
        import_dir,
    ):
        print(line)
    return 0


def _render_summary_output(import_dir, deps, settings):
    document = deps.build_export_inspection_document(import_dir)
    if settings.json_output:
        print(json.dumps(document, indent=2, sort_keys=False, ensure_ascii=False))
        return 0
    if settings.table_output:
        for line in deps.render_export_inspection_tables(
            document,
            import_dir,
            include_header=not settings.no_header,
        ):
            print(line)
        return 0
    for line in deps.render_export_inspection_summary(document, import_dir):
        print(line)
    return 0


def run_inspection_dispatch(import_dir, deps, settings):
    """Render inspect output using the normalized dispatch settings."""
    if settings.report_format is not None:
        return _render_report_output(import_dir, deps, settings)
    return _render_summary_output(import_dir, deps, settings)
