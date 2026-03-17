"""Dashboard prompt-export datasource rewrite helpers."""

import copy
import re
from dataclasses import dataclass
from typing import Any, Optional

from .common import (
    BUILTIN_DATASOURCE_NAMES,
    BUILTIN_DATASOURCE_TYPES,
    DATASOURCE_TYPE_ALIASES,
    GrafanaError,
)


@dataclass(frozen=True)
class ResolvedDatasource:
    key: str
    label: str
    ds_type: str
    input_label: str = ""
    plugin_version: str = ""


@dataclass(frozen=True)
class InputMapping:
    input_name: str
    label: str
    ds_type: str
    plugin_name: str
    plugin_version: str = ""


def build_datasource_catalog(
    datasources: list[dict[str, Any]],
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]]:
    """Index datasources by both uid and name because dashboards use either form."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    by_uid: dict[str, dict[str, Any]] = {}
    by_name: dict[str, dict[str, Any]] = {}
    for datasource in datasources:
        uid = datasource.get("uid")
        name = datasource.get("name")
        if isinstance(uid, str) and uid:
            by_uid[uid] = datasource
        if isinstance(name, str) and name:
            by_name[name] = datasource
    return by_uid, by_name


def is_placeholder_string(value: str) -> bool:
    """Is placeholder string implementation."""
    return value.startswith("$")


def extract_placeholder_name(value: str) -> str:
    """Extract placeholder name implementation."""
    if value.startswith("${") and value.endswith("}") and len(value) > 3:
        return value[2:-1]
    if value.startswith("$") and len(value) > 1:
        return value[1:]
    return value


def is_generated_input_placeholder(value: str) -> bool:
    """Is generated input placeholder implementation."""
    return extract_placeholder_name(value).startswith("DS_")


def is_builtin_datasource_ref(value: Any) -> bool:
    """Is builtin datasource ref implementation."""
    if isinstance(value, str):
        return value in BUILTIN_DATASOURCE_NAMES or is_generated_input_placeholder(value)
    if isinstance(value, dict):
        uid = value.get("uid")
        name = value.get("name")
        ds_type = value.get("type")
        if isinstance(uid, str) and is_generated_input_placeholder(uid):
            return True
        if isinstance(name, str) and is_generated_input_placeholder(name):
            return True
        if uid in BUILTIN_DATASOURCE_NAMES or name in BUILTIN_DATASOURCE_NAMES:
            return True
        if uid in BUILTIN_DATASOURCE_TYPES or ds_type in BUILTIN_DATASOURCE_TYPES:
            return True
    return False


def collect_datasource_refs(node: Any, refs: list[Any]) -> None:
    """Walk the full dashboard tree and collect every datasource reference in place."""
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "datasource":
                refs.append(value)
            collect_datasource_refs(value, refs)
        return
    if isinstance(node, list):
        for item in node:
            collect_datasource_refs(item, refs)


def make_input_name(label: str) -> str:
    """Make input name implementation."""
    normalized = re.sub(r"[^A-Z0-9]+", "_", label.upper()).strip("_")
    normalized = re.sub(r"_+", "_", normalized)
    return "DS_%s" % (normalized or "DATASOURCE")


def make_type_input_base(datasource_type: str) -> str:
    """Make type input base implementation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 101

    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    return make_input_name(alias)


def format_plugin_name(datasource_type: str) -> str:
    """Format plugin name implementation."""
    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    return alias.replace("-", " ").replace("_", " ").title()


def make_input_label(datasource_type: str, index: int) -> str:
    """Make input label implementation."""
    title = format_plugin_name(datasource_type)
    if index == 1:
        return "%s datasource" % title
    return "%s datasource %s" % (title, index)


def build_resolved_datasource(
    key: str,
    label: str,
    ds_type: str,
    input_label: Optional[str] = None,
) -> ResolvedDatasource:
    """Create the normalized datasource descriptor used by prompt export helpers."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 無

    return ResolvedDatasource(
        key=key,
        label=label,
        ds_type=ds_type,
        input_label=input_label or "",
    )


def datasource_plugin_version(datasource: Optional[dict[str, Any]]) -> str:
    """Datasource plugin version implementation."""
    if not isinstance(datasource, dict):
        return ""
    plugin_version = datasource.get("pluginVersion")
    if isinstance(plugin_version, str) and plugin_version:
        return plugin_version
    version = datasource.get("version")
    if isinstance(version, str) and version:
        return version
    meta = datasource.get("meta")
    if isinstance(meta, dict):
        info = meta.get("info")
        if isinstance(info, dict):
            version = info.get("version")
            if isinstance(version, str) and version:
                return version
    return ""


def lookup_datasource(
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
    uid: Optional[str] = None,
    name: Optional[str] = None,
) -> Optional[dict[str, Any]]:
    """Resolve a datasource by UID first, then by datasource name."""
    if isinstance(uid, str) and uid:
        datasource = datasources_by_uid.get(uid)
        if datasource is not None:
            return datasource
    if isinstance(name, str) and name:
        return datasources_by_name.get(name)
    return None


def resolve_datasource_type_alias(
    ref: str,
    datasources_by_uid: dict[str, dict[str, Any]],
) -> Optional[str]:
    """Resolve datasource plugin aliases such as 'prometheus' or 'prom'."""
    ref_lower = ref.lower()
    datasource_type = DATASOURCE_TYPE_ALIASES.get(ref_lower)
    if datasource_type is not None:
        return datasource_type

    for candidate in datasources_by_uid.values():
        candidate_type = candidate.get("type")
        if isinstance(candidate_type, str) and candidate_type.lower() == ref_lower:
            return candidate_type
    return None


def resolve_string_datasource_ref(
    ref: str,
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> ResolvedDatasource:
    """Resolve string datasource references stored as names, UIDs, or type aliases."""
    # Call graph: see callers/callees.
    #   Upstream callers: 313
    #   Downstream callees: 114, 143, 163, 179

    datasource = lookup_datasource(
        datasources_by_uid,
        datasources_by_name,
        uid=ref,
        name=ref,
    )
    if datasource is None:
        datasource_type = resolve_datasource_type_alias(ref, datasources_by_uid)
        if datasource_type is not None:
            return ResolvedDatasource(
                "type:%s" % datasource_type,
                datasource_type,
                datasource_type,
                input_label=format_plugin_name(datasource_type),
            )
        raise GrafanaError(
            "Cannot resolve datasource name or uid %r for prompt export." % ref
        )

    uid = datasource.get("uid") or ref
    label = datasource.get("name") or ref
    ds_type = datasource.get("type")
    if not isinstance(ds_type, str) or not ds_type:
        raise GrafanaError("Datasource %r does not have a usable type." % ref)
    return ResolvedDatasource(
        key="uid:%s" % uid,
        label=label,
        ds_type=ds_type,
        input_label=label,
        plugin_version=datasource_plugin_version(datasource),
    )


def resolve_placeholder_object_ref(
    uid: Any,
    name: Any,
    ds_type: Any,
) -> Optional[ResolvedDatasource]:
    """Resolve object refs that already point at a datasource placeholder token."""
    # Call graph: see callers/callees.
    #   Upstream callers: 261
    #   Downstream callees: 114, 50, 55

    if not isinstance(ds_type, str) or not ds_type:
        return None

    placeholder_value = None
    if isinstance(uid, str) and is_placeholder_string(uid):
        placeholder_value = uid
    elif isinstance(name, str) and is_placeholder_string(name):
        placeholder_value = name
    if placeholder_value is None:
        return None

    token = extract_placeholder_name(placeholder_value)
    return ResolvedDatasource(
        key="var:%s:%s" % (ds_type, token),
        label=token,
        ds_type=ds_type,
        input_label=format_plugin_name(ds_type),
    )


def resolve_object_datasource_ref(
    ref: dict[str, Any],
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> Optional[ResolvedDatasource]:
    """Resolve object datasource refs stored as {'type': ..., 'uid': ...}."""
    # Call graph: see callers/callees.
    #   Upstream callers: 313
    #   Downstream callees: 143, 163, 235, 50

    uid = ref.get("uid")
    name = ref.get("name")
    ds_type = ref.get("type")
    has_placeholder = (
        isinstance(uid, str)
        and is_placeholder_string(uid)
        or isinstance(name, str)
        and is_placeholder_string(name)
    )

    resolved = resolve_placeholder_object_ref(uid, name, ds_type)
    if resolved is not None:
        return resolved
    if has_placeholder:
        return None

    datasource = lookup_datasource(
        datasources_by_uid,
        datasources_by_name,
        uid=uid,
        name=name,
    )
    resolved_type = ds_type
    resolved_label = name or uid
    resolved_uid = uid or name
    if datasource is not None:
        resolved_type = datasource.get("type") or resolved_type
        resolved_label = datasource.get("name") or resolved_label
        resolved_uid = datasource.get("uid") or resolved_uid

    if not isinstance(resolved_type, str) or not resolved_type:
        raise GrafanaError("Cannot resolve datasource type from reference %r." % ref)
    if not isinstance(resolved_label, str) or not resolved_label:
        resolved_label = resolved_type
    if not isinstance(resolved_uid, str) or not resolved_uid:
        resolved_uid = resolved_label

    return ResolvedDatasource(
        key="uid:%s" % resolved_uid,
        label=resolved_label,
        ds_type=resolved_type,
        input_label=resolved_label,
        plugin_version=datasource_plugin_version(datasource),
    )


def resolve_datasource_ref(
    ref: Any,
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> Optional[ResolvedDatasource]:
    """Normalize Grafana datasource references into stable keys for __inputs generation."""
    # Call graph: see callers/callees.
    #   Upstream callers: 341, 531, 673
    #   Downstream callees: 196, 261, 50, 69

    if ref is None or is_builtin_datasource_ref(ref):
        return None

    if isinstance(ref, str):
        if is_placeholder_string(ref):
            return None
        return resolve_string_datasource_ref(
            ref,
            datasources_by_uid,
            datasources_by_name,
        )

    if isinstance(ref, dict):
        return resolve_object_datasource_ref(
            ref,
            datasources_by_uid,
            datasources_by_name,
        )

    return None


def replace_datasource_refs_in_dashboard(
    node: Any,
    ref_mapping: dict[str, InputMapping],
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> None:
    """Replace resolved datasource references with the generated __inputs placeholders."""
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "datasource":
                resolved = resolve_datasource_ref(
                    value,
                    datasources_by_uid=datasources_by_uid,
                    datasources_by_name=datasources_by_name,
                )
                if resolved is not None:
                    input_name = ref_mapping[resolved.key].input_name
                    placeholder = "${%s}" % input_name
                    if isinstance(value, dict):
                        replacement = {"uid": placeholder}
                        if resolved.ds_type:
                            replacement["type"] = resolved.ds_type
                        node[key] = replacement
                    else:
                        node[key] = placeholder
            else:
                replace_datasource_refs_in_dashboard(
                    value,
                    ref_mapping=ref_mapping,
                    datasources_by_uid=datasources_by_uid,
                    datasources_by_name=datasources_by_name,
                )
        return
    if isinstance(node, list):
        for item in node:
            replace_datasource_refs_in_dashboard(
                item,
                ref_mapping=ref_mapping,
                datasources_by_uid=datasources_by_uid,
                datasources_by_name=datasources_by_name,
            )


def ensure_datasource_template_variable(
    dashboard: dict[str, Any],
    datasource_type: str,
) -> None:
    """Create Grafana's conventional $datasource variable if one does not already exist."""
    templating = dashboard.setdefault("templating", {})
    if not isinstance(templating, dict):
        return
    variables = templating.setdefault("list", [])
    if not isinstance(variables, list):
        return

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        if variable.get("type") == "datasource":
            return

    variables.insert(
        0,
        {
            "current": {},
            "label": "Data source",
            "name": "datasource",
            "options": [],
            "query": datasource_type,
            "refresh": 1,
            "regex": "",
            "type": "datasource",
        },
    )


def rewrite_panel_datasources_to_template_variable(
    panels: list[dict[str, Any]],
    placeholder_names: set[str],
) -> None:
    """Collapse panel datasource placeholders down to the shared $datasource variable."""
    for panel in panels:
        datasource = panel.get("datasource")
        if isinstance(datasource, str):
            if datasource in placeholder_names or datasource in {"$datasource", "${datasource}"}:
                panel["datasource"] = {"uid": "$datasource"}
        elif isinstance(datasource, dict):
            uid = datasource.get("uid")
            if isinstance(uid, str) and (
                uid in placeholder_names or uid in {"$datasource", "${datasource}"}
            ):
                panel["datasource"] = {"uid": "$datasource"}

        nested = panel.get("panels")
        if isinstance(nested, list):
            rewrite_panel_datasources_to_template_variable(
                [item for item in nested if isinstance(item, dict)],
                placeholder_names,
            )


def allocate_input_mapping(
    resolved: ResolvedDatasource,
    ref_mapping: dict[str, InputMapping],
    type_counts: dict[str, int],
    key: Optional[str] = None,
) -> InputMapping:
    """Create or reuse one __inputs mapping entry for a resolved datasource ref."""
    # Call graph: see callers/callees.
    #   Upstream callers: 531, 673
    #   Downstream callees: 101, 114, 120

    mapping_key = key or resolved.key
    mapping = ref_mapping.get(mapping_key)
    if mapping is not None:
        return mapping

    ds_type = resolved.ds_type
    base_label = resolved.input_label or format_plugin_name(ds_type)
    input_base = make_input_name(base_label)
    index = type_counts.get(input_base, 0) + 1
    type_counts[input_base] = index
    mapping = InputMapping(
        input_name=input_base if index == 1 else "%s_%s" % (input_base, index),
        label=resolved.input_label or make_input_label(ds_type, index),
        ds_type=ds_type,
        plugin_name=format_plugin_name(ds_type),
        plugin_version=resolved.plugin_version,
    )
    ref_mapping[mapping_key] = mapping
    return mapping


def rewrite_template_variable_query(
    variable: dict[str, Any],
    mapping: InputMapping,
    datasource_var_types: dict[str, str],
    datasource_var_placeholders: set[str],
) -> None:
    """Rewrite one datasource template variable into importer-friendly prompt form."""
    var_name = variable.get("name")
    if isinstance(var_name, str) and var_name:
        datasource_var_types[var_name] = mapping.ds_type
        datasource_var_placeholders.add("$%s" % var_name)
        datasource_var_placeholders.add("${%s}" % var_name)

    variable["current"] = {}
    variable["options"] = []
    variable["query"] = mapping.ds_type
    variable["refresh"] = 1
    variable["regex"] = variable.get("regex", "")
    if variable.get("hide") == 0:
        variable.pop("hide", None)


def rewrite_template_variable_datasource(
    variable: dict[str, Any],
    datasource_var_types: dict[str, str],
    datasource_var_placeholders: set[str],
    datasource_var_input_names: dict[str, str],
) -> None:
    """Rewrite datasource selectors that point at datasource template variables."""
    datasource = variable.get("datasource")
    placeholder_value = None
    if isinstance(datasource, str):
        placeholder_value = datasource
    elif isinstance(datasource, dict):
        uid = datasource.get("uid")
        if isinstance(uid, str):
            placeholder_value = uid

    if not isinstance(placeholder_value, str):
        return
    datasource_type = datasource_var_types.get(
        extract_placeholder_name(placeholder_value)
    )
    input_name = datasource_var_input_names.get(
        extract_placeholder_name(placeholder_value)
    )
    if (
        placeholder_value not in datasource_var_placeholders
        or not datasource_type
        or not input_name
    ):
        return

    variable["datasource"] = {
        "type": datasource_type,
        "uid": "${%s}" % input_name,
    }
    variable["current"] = {}
    variable["options"] = []


def prepare_templating_for_external_import(
    dashboard: dict[str, Any],
    ref_mapping: dict[str, InputMapping],
    type_counts: dict[str, int],
    datasources_by_uid: dict[str, dict[str, Any]],
    datasources_by_name: dict[str, dict[str, Any]],
) -> set[str]:
    """Rewrite datasource template variables so exported dashboards prompt on import."""
    # Call graph: see callers/callees.
    #   Upstream callers: 673
    #   Downstream callees: 313, 442, 470, 492

    templating = dashboard.get("templating")
    if not isinstance(templating, dict):
        return set()
    variables = templating.get("list")
    if not isinstance(variables, list):
        return set()

    datasource_var_types: dict[str, str] = {}
    datasource_var_placeholders: set[str] = set()
    datasource_var_input_names: dict[str, str] = {}

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        if variable.get("type") != "datasource":
            continue

        query = variable.get("query")
        ds_ref = query if isinstance(query, str) else None
        if not ds_ref:
            continue

        resolved = resolve_datasource_ref(
            ds_ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if resolved is None:
            continue

        mapping = allocate_input_mapping(
            resolved,
            ref_mapping,
            type_counts,
            key="templating:%s" % (variable.get("name") or resolved.key),
        )
        rewrite_template_variable_query(
            variable,
            mapping,
            datasource_var_types,
            datasource_var_placeholders,
        )
        if isinstance(variable.get("name"), str) and variable.get("name"):
            datasource_var_input_names[str(variable["name"])] = mapping.input_name

    for variable in variables:
        if not isinstance(variable, dict):
            continue
        rewrite_template_variable_datasource(
            variable,
            datasource_var_types,
            datasource_var_placeholders,
            datasource_var_input_names,
        )

    return set(datasource_var_types)


def collect_panel_types(panels: list[dict[str, Any]], panel_types: set[str]) -> None:
    """Gather panel plugin ids so __requires mirrors what Grafana exports."""
    for panel in panels:
        panel_type = panel.get("type")
        if isinstance(panel_type, str) and panel_type:
            panel_types.add(panel_type)
        nested = panel.get("panels")
        if isinstance(nested, list):
            collect_panel_types(
                [item for item in nested if isinstance(item, dict)],
                panel_types,
            )


def build_input_definitions(
    ref_mapping: dict[str, InputMapping],
) -> list[dict[str, str]]:
    """Build Grafana's __inputs block from the resolved datasource mapping table."""
    return [
        {
            "name": mapping.input_name,
            "label": mapping.label,
            "description": "",
            "type": "datasource",
            "pluginId": mapping.ds_type,
            "pluginName": mapping.plugin_name,
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1].input_name)
    ]


def build_requires_block(
    ref_mapping: dict[str, InputMapping],
    panel_types: set[str],
) -> list[dict[str, str]]:
    """Build Grafana's __requires block for Grafana itself, datasources, and panels."""
    requires = [{"type": "grafana", "id": "grafana", "name": "Grafana", "version": ""}]
    datasource_plugins: dict[str, tuple[str, str]] = {}
    for mapping in ref_mapping.values():
        existing = datasource_plugins.get(mapping.ds_type)
        if existing is None or (not existing[1] and mapping.plugin_version):
            datasource_plugins[mapping.ds_type] = (
                mapping.plugin_name,
                mapping.plugin_version,
            )
    requires.extend(
        {
            "type": "datasource",
            "id": plugin_id,
            "name": plugin_name,
            "version": plugin_version,
        }
        for plugin_id, (plugin_name, plugin_version) in sorted(datasource_plugins.items())
    )
    requires.extend(
        {
            "type": "panel",
            "id": panel_type,
            "name": panel_type,
            "version": "",
        }
        for panel_type in sorted(panel_types)
    )
    return requires


def build_preserved_web_import_document(payload: dict[str, Any]) -> dict[str, Any]:
    """Keep the dashboard JSON Grafana expects for web import, but clear the numeric id."""
    dashboard = payload.get("dashboard", payload)
    if not isinstance(dashboard, dict):
        raise GrafanaError("Unexpected dashboard payload from Grafana.")
    document = copy.deepcopy(dashboard)
    document["id"] = None
    return document


def build_external_export_document(
    payload: dict[str, Any],
    datasource_catalog: tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]]],
) -> dict[str, Any]:
    """Convert a fetched dashboard into Grafana's web-import prompt format."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 313, 341, 384, 417, 442, 531, 597, 611, 628, 663, 88

    dashboard = build_preserved_web_import_document(payload)

    datasources_by_uid, datasources_by_name = datasource_catalog
    refs: list[Any] = []
    collect_datasource_refs(dashboard, refs)

    ref_mapping: dict[str, InputMapping] = {}
    type_counts: dict[str, int] = {}
    prepare_templating_for_external_import(
        dashboard,
        ref_mapping=ref_mapping,
        type_counts=type_counts,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )
    for ref in refs:
        resolved = resolve_datasource_ref(
            ref,
            datasources_by_uid=datasources_by_uid,
            datasources_by_name=datasources_by_name,
        )
        if resolved is None or resolved.key in ref_mapping:
            continue
        allocate_input_mapping(resolved, ref_mapping, type_counts)

    replace_datasource_refs_in_dashboard(
        dashboard,
        ref_mapping=ref_mapping,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )

    datasource_types = sorted({mapping.ds_type for mapping in ref_mapping.values()})
    if len(datasource_types) == 1 and len(ref_mapping) == 1:
        ensure_datasource_template_variable(dashboard, datasource_types[0])
        placeholder_names = {
            "${%s}" % mapping.input_name for mapping in ref_mapping.values()
        }
        panels = dashboard.get("panels")
        if isinstance(panels, list):
            rewrite_panel_datasources_to_template_variable(
                [item for item in panels if isinstance(item, dict)],
                placeholder_names,
            )

    dashboard["__inputs"] = build_input_definitions(ref_mapping)

    panel_types: set[str] = set()
    panels = dashboard.get("panels")
    if isinstance(panels, list):
        collect_panel_types(
            [item for item in panels if isinstance(item, dict)],
            panel_types,
        )
    dashboard["__requires"] = build_requires_block(ref_mapping, panel_types)
    dashboard["__elements"] = {}
    return dashboard
