"""Dashboard export dependency assembly helpers."""

from .folder_support import collect_folder_inventory
from .listing import (
    attach_dashboard_org,
    build_datasource_inventory_record,
    resolve_dashboard_source_metadata,
)
from .output_support import (
    build_all_orgs_output_dir,
    build_dashboard_index_item,
    build_export_metadata,
    build_export_variant_dirs,
    build_output_path,
    build_root_export_index,
    ensure_dashboard_write_target,
    write_dashboard,
    write_json_document,
)
from .permission_support import collect_permission_export_documents
from .progress import (
    print_dashboard_export_progress,
    print_dashboard_export_progress_summary,
)
from .transformer import (
    build_datasource_catalog,
    build_external_export_document,
    build_preserved_web_import_document,
)


def build_export_workflow_deps(config):
    """Build export workflow deps implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return {
        "DASHBOARD_PERMISSION_BUNDLE_FILENAME": config[
            "DASHBOARD_PERMISSION_BUNDLE_FILENAME"
        ],
        "GrafanaError": config["GrafanaError"],
        "DATASOURCE_INVENTORY_FILENAME": config["DATASOURCE_INVENTORY_FILENAME"],
        "EXPORT_METADATA_FILENAME": config["EXPORT_METADATA_FILENAME"],
        "FOLDER_INVENTORY_FILENAME": config["FOLDER_INVENTORY_FILENAME"],
        "PROMPT_EXPORT_SUBDIR": config["PROMPT_EXPORT_SUBDIR"],
        "RAW_EXPORT_SUBDIR": config["RAW_EXPORT_SUBDIR"],
        "attach_dashboard_org": attach_dashboard_org,
        "build_all_orgs_output_dir": (
            lambda output_dir, org: build_all_orgs_output_dir(
                output_dir,
                org,
                default_unknown_uid=config["DEFAULT_UNKNOWN_UID"],
            )
        ),
        "build_client": config["build_client"],
        "build_dashboard_index_item": (
            lambda summary, uid: build_dashboard_index_item(
                summary,
                uid,
                default_org_name=config["DEFAULT_ORG_NAME"],
                default_org_id=config["DEFAULT_ORG_ID"],
            )
        ),
        "build_datasource_catalog": build_datasource_catalog,
        "build_datasource_inventory_record": build_datasource_inventory_record,
        "build_export_metadata": (
            lambda variant, dashboard_count, format_name=None, folders_file=None, datasources_file=None, permissions_file=None, org_name=None, org_id=None, orgs=None: build_export_metadata(
                variant,
                dashboard_count,
                tool_schema_version=config["TOOL_SCHEMA_VERSION"],
                root_index_kind=config["ROOT_INDEX_KIND"],
                format_name=format_name,
                folders_file=folders_file,
                datasources_file=datasources_file,
                permissions_file=permissions_file,
                org_name=org_name,
                org_id=org_id,
                orgs=orgs,
            )
        ),
        "build_export_variant_dirs": (
            lambda output_dir: build_export_variant_dirs(
                output_dir,
                raw_export_subdir=config["RAW_EXPORT_SUBDIR"],
                prompt_export_subdir=config["PROMPT_EXPORT_SUBDIR"],
            )
        ),
        "build_external_export_document": build_external_export_document,
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
        "extract_dashboard_object": config["extract_dashboard_object"],
        "resolve_dashboard_source_metadata": (
            lambda payload, datasources_by_uid, datasources_by_name: resolve_dashboard_source_metadata(
                payload,
                extract_dashboard_object=config["extract_dashboard_object"],
                datasource_error=config["GrafanaError"],
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )
        ),
        "build_root_export_index": (
            lambda index_items, raw_index_path, prompt_index_path: build_root_export_index(
                index_items,
                raw_index_path,
                prompt_index_path,
                tool_schema_version=config["TOOL_SCHEMA_VERSION"],
                root_index_kind=config["ROOT_INDEX_KIND"],
            )
        ),
        "build_variant_index": config["build_variant_index"],
        "collect_permission_export_documents": collect_permission_export_documents,
        "collect_folder_inventory": collect_folder_inventory,
        "ensure_dashboard_write_target": (
            lambda output_path, overwrite, create_parents=True: ensure_dashboard_write_target(
                output_path,
                overwrite,
                error_cls=config["GrafanaError"],
                create_parents=create_parents,
            )
        ),
        "print_dashboard_export_progress": print_dashboard_export_progress,
        "print_dashboard_export_progress_summary": (
            print_dashboard_export_progress_summary
        ),
        "sys": config["sys"],
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
