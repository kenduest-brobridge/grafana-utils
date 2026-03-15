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
from ..roadmap_contracts import (
    build_dependency_graph_document,
    build_dependency_graph_governance_summary,
    render_dependency_graph_dot,
    render_dependency_graph_governance_text,
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


class _InspectionRawDocumentDeps(object):
    """Typed wrapper around the raw-document helper bundle."""

    def __init__(self, config):
        self.raw_export_subdir = config["RAW_EXPORT_SUBDIR"]
        self.prompt_export_subdir = config["PROMPT_EXPORT_SUBDIR"]
        self.export_metadata_filename = config["EXPORT_METADATA_FILENAME"]
        self.folder_inventory_filename = config["FOLDER_INVENTORY_FILENAME"]
        self.datasource_inventory_filename = config["DATASOURCE_INVENTORY_FILENAME"]
        self.root_index_kind = config["ROOT_INDEX_KIND"]
        self.tool_schema_version = config["TOOL_SCHEMA_VERSION"]
        self.build_folder_inventory_lookup = build_folder_inventory_lookup
        self.extract_dashboard_object = extract_dashboard_object
        self.iter_dashboard_panels = iter_dashboard_panels
        self.load_json_file = load_json_file
        self.resolve_folder_inventory_record_for_dashboard = (
            resolve_folder_inventory_record_for_dashboard
        )

    def discover_dashboard_files(self, import_dir):
        return discover_dashboard_files(
            import_dir,
            self.raw_export_subdir,
            self.prompt_export_subdir,
            self.export_metadata_filename,
            self.folder_inventory_filename,
            self.datasource_inventory_filename,
        )

    def load_datasource_inventory(self, import_dir, metadata=None):
        return load_datasource_inventory(
            import_dir,
            self.datasource_inventory_filename,
            metadata=metadata,
        )

    def load_export_metadata(self, import_dir, expected_variant=None):
        return load_export_metadata(
            import_dir,
            self.export_metadata_filename,
            self.root_index_kind,
            self.tool_schema_version,
            expected_variant=expected_variant,
        )

    def load_folder_inventory(self, import_dir, metadata=None):
        return load_folder_inventory(
            import_dir,
            self.folder_inventory_filename,
            metadata=metadata,
        )

    def __getitem__(self, key):
        if key == "RAW_EXPORT_SUBDIR":
            return self.raw_export_subdir
        return getattr(self, key)


class _InspectionSummaryDocumentDeps(_InspectionRawDocumentDeps):
    """Typed wrapper around the summary-document helper bundle."""

    def __init__(self, config):
        super(_InspectionSummaryDocumentDeps, self).__init__(config)
        self.build_datasource_catalog = build_datasource_catalog
        self.collect_datasource_refs = collect_datasource_refs


class InspectionWorkflowDeps(object):
    """Explicit inspection workflow wiring with compatibility accessors."""

    def __init__(self, config):
        self.grafana_error = config["GrafanaError"]
        self.datasource_inventory_filename = config["DATASOURCE_INVENTORY_FILENAME"]
        self.export_metadata_filename = config["EXPORT_METADATA_FILENAME"]
        self.folder_inventory_filename = config["FOLDER_INVENTORY_FILENAME"]
        self.raw_export_subdir = config["RAW_EXPORT_SUBDIR"]
        self.attach_dashboard_org = attach_dashboard_org
        self.build_client = config["build_client"]
        self.build_datasource_inventory_record = build_datasource_inventory_record
        self.build_export_inspection_governance_document = (
            build_export_inspection_governance_document
        )
        self.build_dependency_graph_document = build_dependency_graph_document
        self.build_dependency_graph_governance_summary = (
            build_dependency_graph_governance_summary
        )
        self.build_grouped_export_inspection_report_document = (
            build_grouped_export_inspection_report_document
        )
        self.build_preserved_web_import_document = build_preserved_web_import_document
        self.build_variant_index = build_variant_index
        self.collect_folder_inventory = collect_folder_inventory
        self.filter_export_inspection_report_document = (
            filter_export_inspection_report_document
        )
        self.parse_report_columns = parse_report_columns
        self.render_export_inspection_governance_tables = (
            render_export_inspection_governance_tables
        )
        self.render_dependency_graph_dot = render_dependency_graph_dot
        self.render_dependency_graph_governance_text = (
            render_dependency_graph_governance_text
        )
        self.render_export_inspection_grouped_report = (
            render_export_inspection_grouped_report
        )
        self.render_export_inspection_report_csv = render_export_inspection_report_csv
        self.render_export_inspection_report_tables = (
            render_export_inspection_report_tables
        )
        self.render_export_inspection_summary = render_export_inspection_summary
        self.render_export_inspection_tables = render_export_inspection_tables
        self.render_export_inspection_tree_tables = render_export_inspection_tree_tables
        self.write_json_document = write_json_document

        self._default_org_name = config["DEFAULT_ORG_NAME"]
        self._default_org_id = config["DEFAULT_ORG_ID"]
        self._default_folder_title = config["DEFAULT_FOLDER_TITLE"]
        self._default_dashboard_title = config["DEFAULT_DASHBOARD_TITLE"]
        self._default_unknown_uid = config["DEFAULT_UNKNOWN_UID"]
        self._root_index_kind = config["ROOT_INDEX_KIND"]
        self._tool_schema_version = config["TOOL_SCHEMA_VERSION"]
        self._raw_document_deps = _InspectionRawDocumentDeps(config)
        self._summary_document_deps = _InspectionSummaryDocumentDeps(config)

    def build_dashboard_index_item(self, summary, uid):
        return build_dashboard_index_item(
            summary,
            uid,
            default_org_name=self._default_org_name,
            default_org_id=self._default_org_id,
        )

    def build_export_inspection_document(self, import_dir):
        return build_export_inspection_document(
            import_dir,
            self._summary_document_deps,
        )

    def build_export_inspection_report_document(self, import_dir):
        return build_export_inspection_report_document(
            import_dir,
            self._raw_document_deps,
        )

    def build_export_metadata(
        self,
        variant,
        dashboard_count,
        format_name=None,
        folders_file=None,
        datasources_file=None,
    ):
        return build_export_metadata(
            variant,
            dashboard_count,
            tool_schema_version=self._tool_schema_version,
            root_index_kind=self._root_index_kind,
            format_name=format_name,
            folders_file=folders_file,
            datasources_file=datasources_file,
        )

    def build_output_path(self, output_dir, summary, flat):
        return build_output_path(
            output_dir,
            summary,
            flat,
            default_folder_title=self._default_folder_title,
            default_dashboard_title=self._default_dashboard_title,
            default_unknown_uid=self._default_unknown_uid,
        )

    def write_dashboard(self, payload, output_path, overwrite):
        return write_dashboard(
            payload,
            output_path,
            overwrite,
            error_cls=self.grafana_error,
        )

    def __getitem__(self, key):
        compatibility_map = {
            "GrafanaError": self.grafana_error,
            "DATASOURCE_INVENTORY_FILENAME": self.datasource_inventory_filename,
            "EXPORT_METADATA_FILENAME": self.export_metadata_filename,
            "FOLDER_INVENTORY_FILENAME": self.folder_inventory_filename,
            "RAW_EXPORT_SUBDIR": self.raw_export_subdir,
        }
        if key in compatibility_map:
            return compatibility_map[key]
        return getattr(self, key)


def iter_dashboard_panels(panels):
    """Flatten Grafana panels, including nested row/library panel layouts."""
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
    return InspectionWorkflowDeps(config)
