"""Dashboard inspection dependency assembly helpers."""

from .export_inventory import discover_dashboard_files, load_export_metadata
from .folder_support import (
    build_folder_inventory_lookup,
    collect_folder_inventory,
    load_datasource_inventory,
    load_folder_inventory,
    resolve_folder_inventory_record_for_dashboard,
)
from .import_support import extract_dashboard_object, load_json_file
from .inspection_governance import build_export_inspection_governance_document
from .inspection_governance_render import render_export_inspection_governance_tables
from .inspection_report import (
    build_export_inspection_report_document,
    build_grouped_export_inspection_report_document,
    filter_export_inspection_report_document,
    parse_report_columns,
)
from .inspection_render import (
    render_export_inspection_grouped_report,
    render_export_inspection_report_csv,
    render_export_inspection_report_tables,
    render_export_inspection_tree_tables,
)
from .inspection_summary import (
    build_export_inspection_document,
    render_export_inspection_summary,
    render_export_inspection_tables,
)
from .listing import attach_dashboard_org, build_datasource_inventory_record
from .output_support import (
    build_dashboard_index_item,
    build_export_metadata,
    build_output_path,
    build_variant_index,
    write_dashboard,
    write_json_document,
)
from .transformer import (
    build_datasource_catalog,
    build_preserved_web_import_document,
    collect_datasource_refs,
)


def iter_dashboard_panels(panels):
    """Flatten Grafana panels, including nested row/library panel layouts."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    flattened = []
    if not isinstance(panels, list):
        return flattened
    for panel in panels:
        if not isinstance(panel, dict):
            continue
        flattened.append(panel)
        nested_panels = panel.get("panels")
        if isinstance(nested_panels, list):
            flattened.extend(iter_dashboard_panels(nested_panels))
    return flattened


def build_inspection_workflow_deps(config):
    """Build inspection workflow deps implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    raw_document_deps = {
        "RAW_EXPORT_SUBDIR": config["RAW_EXPORT_SUBDIR"],
        "build_folder_inventory_lookup": build_folder_inventory_lookup,
        "discover_dashboard_files": (
            lambda import_dir: discover_dashboard_files(
                import_dir,
                config["RAW_EXPORT_SUBDIR"],
                config["PROMPT_EXPORT_SUBDIR"],
                config["EXPORT_METADATA_FILENAME"],
                config["FOLDER_INVENTORY_FILENAME"],
                config["DATASOURCE_INVENTORY_FILENAME"],
            )
        ),
        "extract_dashboard_object": extract_dashboard_object,
        "iter_dashboard_panels": iter_dashboard_panels,
        "load_datasource_inventory": (
            lambda import_dir, metadata=None: load_datasource_inventory(
                import_dir,
                config["DATASOURCE_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "load_export_metadata": (
            lambda import_dir, expected_variant=None: load_export_metadata(
                import_dir,
                config["EXPORT_METADATA_FILENAME"],
                config["ROOT_INDEX_KIND"],
                config["TOOL_SCHEMA_VERSION"],
                expected_variant=expected_variant,
            )
        ),
        "load_folder_inventory": (
            lambda import_dir, metadata=None: load_folder_inventory(
                import_dir,
                config["FOLDER_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "load_json_file": load_json_file,
        "resolve_folder_inventory_record_for_dashboard": (
            resolve_folder_inventory_record_for_dashboard
        ),
    }
    summary_document_deps = dict(
        raw_document_deps,
        build_datasource_catalog=build_datasource_catalog,
        collect_datasource_refs=collect_datasource_refs,
    )
    return {
        "GrafanaError": config["GrafanaError"],
        "DATASOURCE_INVENTORY_FILENAME": config["DATASOURCE_INVENTORY_FILENAME"],
        "EXPORT_METADATA_FILENAME": config["EXPORT_METADATA_FILENAME"],
        "FOLDER_INVENTORY_FILENAME": config["FOLDER_INVENTORY_FILENAME"],
        "RAW_EXPORT_SUBDIR": config["RAW_EXPORT_SUBDIR"],
        "load_datasource_inventory": (
            lambda import_dir, metadata=None: load_datasource_inventory(
                import_dir,
                config["DATASOURCE_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "load_export_metadata": (
            lambda import_dir, expected_variant=None: load_export_metadata(
                import_dir,
                config["EXPORT_METADATA_FILENAME"],
                config["ROOT_INDEX_KIND"],
                config["TOOL_SCHEMA_VERSION"],
                expected_variant=expected_variant,
            )
        ),
        "attach_dashboard_org": attach_dashboard_org,
        "build_client": config["build_client"],
        "build_dashboard_index_item": (
            lambda summary, uid: build_dashboard_index_item(
                summary,
                uid,
                default_org_name=config["DEFAULT_ORG_NAME"],
                default_org_id=config["DEFAULT_ORG_ID"],
            )
        ),
        "build_datasource_inventory_record": build_datasource_inventory_record,
        "build_export_inspection_document": (
            lambda import_dir: build_export_inspection_document(
                import_dir,
                summary_document_deps,
            )
        ),
        "build_export_inspection_governance_document": (
            build_export_inspection_governance_document
        ),
        "build_export_inspection_report_document": (
            lambda import_dir: build_export_inspection_report_document(
                import_dir,
                raw_document_deps,
            )
        ),
        "build_export_metadata": (
            lambda variant, dashboard_count, format_name=None, folders_file=None, datasources_file=None: build_export_metadata(
                variant,
                dashboard_count,
                tool_schema_version=config["TOOL_SCHEMA_VERSION"],
                root_index_kind=config["ROOT_INDEX_KIND"],
                format_name=format_name,
                folders_file=folders_file,
                datasources_file=datasources_file,
            )
        ),
        "build_grouped_export_inspection_report_document": (
            build_grouped_export_inspection_report_document
        ),
        "build_output_path": (
            lambda output_dir, summary, flat: build_output_path(
                output_dir,
                summary,
                flat,
                default_folder_title=config["DEFAULT_FOLDER_TITLE"],
                default_dashboard_title=config["DEFAULT_DASHBOARD_TITLE"],
                default_unknown_uid=config["DEFAULT_UNKNOWN_UID"],
            )
        ),
        "build_preserved_web_import_document": build_preserved_web_import_document,
        "build_variant_index": build_variant_index,
        "collect_folder_inventory": collect_folder_inventory,
        "filter_export_inspection_report_document": (
            filter_export_inspection_report_document
        ),
        "parse_report_columns": parse_report_columns,
        "render_export_inspection_governance_tables": (
            render_export_inspection_governance_tables
        ),
        "render_export_inspection_grouped_report": (
            render_export_inspection_grouped_report
        ),
        "render_export_inspection_report_csv": render_export_inspection_report_csv,
        "render_export_inspection_report_tables": (
            render_export_inspection_report_tables
        ),
        "render_export_inspection_summary": render_export_inspection_summary,
        "render_export_inspection_tables": render_export_inspection_tables,
        "render_export_inspection_tree_tables": render_export_inspection_tree_tables,
        "write_dashboard": (
            lambda payload, output_path, overwrite: write_dashboard(
                payload,
                output_path,
                overwrite,
                error_cls=config["GrafanaError"],
            )
        ),
        "write_json_document": write_json_document,
    }
