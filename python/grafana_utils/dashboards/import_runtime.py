"""Dashboard import dependency assembly helpers."""

from .export_inventory import (
    discover_dashboard_files,
    discover_org_raw_export_dirs,
    resolve_export_org_id,
    resolve_export_org_name,
)
from .folder_path_match import (
    apply_folder_path_guard_to_action,
    build_folder_path_match_result,
    resolve_existing_dashboard_folder_path,
    resolve_source_dashboard_folder_path,
)
from .folder_support import (
    build_folder_inventory_lookup,
    ensure_folder_inventory,
    inspect_folder_inventory,
    resolve_dashboard_import_folder_path,
    resolve_folder_inventory_requirements,
)
from .import_support import (
    build_dashboard_import_dry_run_record,
    collect_dashboard_import_dependency_records,
    build_import_payload,
    describe_dashboard_import_mode,
    determine_dashboard_import_action,
    determine_import_folder_uid_override,
    extract_dashboard_object,
    fetch_dashboard_import_dependency_availability,
    load_export_metadata,
    load_json_file,
    render_dashboard_import_dry_run_json,
    render_dashboard_import_dry_run_table,
    render_folder_inventory_dry_run_table,
    validate_dashboard_import_dependencies,
)
from .progress import print_dashboard_import_progress


def build_import_workflow_deps(config):
    """Build import workflow deps implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return {
        "DEFAULT_UNKNOWN_UID": config["DEFAULT_UNKNOWN_UID"],
        "FOLDER_INVENTORY_FILENAME": config["FOLDER_INVENTORY_FILENAME"],
        "GrafanaError": config["GrafanaError"],
        "IMPORT_DRY_RUN_COLUMN_HEADERS": config["IMPORT_DRY_RUN_COLUMN_HEADERS"],
        "RAW_EXPORT_SUBDIR": config["RAW_EXPORT_SUBDIR"],
        "apply_folder_path_guard_to_action": apply_folder_path_guard_to_action,
        "build_client": config["build_client"],
        "build_dashboard_import_dry_run_record": build_dashboard_import_dry_run_record,
        "build_folder_inventory_lookup": build_folder_inventory_lookup,
        "build_folder_path_match_result": build_folder_path_match_result,
        "build_import_payload": build_import_payload,
        "collect_dashboard_import_dependency_records": (
            collect_dashboard_import_dependency_records
        ),
        "create_organization": (
            lambda client, org_name: client.create_organization({"name": org_name})
        ),
        "describe_dashboard_import_mode": describe_dashboard_import_mode,
        "determine_dashboard_import_action": determine_dashboard_import_action,
        "determine_import_folder_uid_override": determine_import_folder_uid_override,
        "discover_dashboard_files": (
            lambda import_dir: discover_dashboard_files(
                import_dir,
                config["RAW_EXPORT_SUBDIR"],
                config["PROMPT_EXPORT_SUBDIR"],
                config["EXPORT_METADATA_FILENAME"],
                config["FOLDER_INVENTORY_FILENAME"],
                config["DATASOURCE_INVENTORY_FILENAME"],
                config["DASHBOARD_PERMISSION_BUNDLE_FILENAME"],
            )
        ),
        "discover_org_raw_export_dirs": (
            lambda import_dir: discover_org_raw_export_dirs(
                import_dir,
                config["RAW_EXPORT_SUBDIR"],
            )
        ),
        "ensure_folder_inventory": ensure_folder_inventory,
        "extract_dashboard_object": extract_dashboard_object,
        "inspect_folder_inventory": inspect_folder_inventory,
        "input_reader": config["input_reader"],
        "is_tty": config["is_tty"],
        "load_export_metadata": (
            lambda import_dir, expected_variant=None: load_export_metadata(
                import_dir,
                config["EXPORT_METADATA_FILENAME"],
                config["ROOT_INDEX_KIND"],
                config["TOOL_SCHEMA_VERSION"],
                expected_variant=expected_variant,
            )
        ),
        "fetch_dashboard_import_dependency_availability": (
            fetch_dashboard_import_dependency_availability
        ),
        "load_json_file": load_json_file,
        "print_dashboard_import_progress": print_dashboard_import_progress,
        "output_writer": config["output_writer"],
        "render_dashboard_import_dry_run_json": render_dashboard_import_dry_run_json,
        "render_dashboard_import_dry_run_table": render_dashboard_import_dry_run_table,
        "render_folder_inventory_dry_run_table": render_folder_inventory_dry_run_table,
        "resolve_dashboard_import_folder_path": resolve_dashboard_import_folder_path,
        "resolve_existing_dashboard_folder_path": resolve_existing_dashboard_folder_path,
        "resolve_export_org_id": (
            lambda import_dir, metadata=None: resolve_export_org_id(
                import_dir,
                config["FOLDER_INVENTORY_FILENAME"],
                config["DATASOURCE_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "resolve_export_org_name": (
            lambda import_dir, metadata=None: resolve_export_org_name(
                import_dir,
                config["FOLDER_INVENTORY_FILENAME"],
                config["DATASOURCE_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "resolve_folder_inventory_requirements": (
            lambda args, import_dir, metadata: resolve_folder_inventory_requirements(
                args,
                import_dir,
                folder_inventory_filename=config["FOLDER_INVENTORY_FILENAME"],
                metadata=metadata,
            )
        ),
        "resolve_source_dashboard_folder_path": resolve_source_dashboard_folder_path,
        "validate_dashboard_import_dependencies": (
            validate_dashboard_import_dependencies
        ),
    }
