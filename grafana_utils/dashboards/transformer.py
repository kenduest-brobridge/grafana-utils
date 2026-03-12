"""Dashboard prompt-export datasource rewrite helpers."""

import copy
import re
from typing import Any, Dict, List, Optional, Set, Tuple

from .common import (
    BUILTIN_DATASOURCE_NAMES,
    BUILTIN_DATASOURCE_TYPES,
    DATASOURCE_TYPE_ALIASES,
    GrafanaError,
)


def build_datasource_catalog(
    datasources: List[Dict[str, Any]],
) -> Tuple[Dict[str, Dict[str, Any]], Dict[str, Dict[str, Any]]]:
    """Index datasources by both uid and name because dashboards use either form."""
    by_uid: Dict[str, Dict[str, Any]] = {}
    by_name: Dict[str, Dict[str, Any]] = {}
    for datasource in datasources:
        uid = datasource.get("uid")
        name = datasource.get("name")
        if isinstance(uid, str) and uid:
            by_uid[uid] = datasource
        if isinstance(name, str) and name:
            by_name[name] = datasource
    return by_uid, by_name


def is_placeholder_string(value: str) -> bool:
    return value.startswith("$")


def extract_placeholder_name(value: str) -> str:
    if value.startswith("${") and value.endswith("}") and len(value) > 3:
        return value[2:-1]
    if value.startswith("$") and len(value) > 1:
        return value[1:]
    return value


def is_generated_input_placeholder(value: str) -> bool:
    return extract_placeholder_name(value).startswith("DS_")


def is_builtin_datasource_ref(value: Any) -> bool:
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


def collect_datasource_refs(node: Any, refs: List[Any]) -> None:
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
    normalized = re.sub(r"[^A-Z0-9]+", "_", label.upper()).strip("_")
    normalized = re.sub(r"_+", "_", normalized)
    return "DS_%s" % (normalized or "DATASOURCE")


def make_type_input_base(datasource_type: str) -> str:
    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    return make_input_name(alias)


def format_plugin_name(datasource_type: str) -> str:
    alias = DATASOURCE_TYPE_ALIASES.get(datasource_type.lower(), datasource_type)
    return alias.replace("-", " ").replace("_", " ").title()


def make_input_label(datasource_type: str, index: int) -> str:
    title = format_plugin_name(datasource_type)
    if index == 1:
        return "%s datasource" % title
    return "%s datasource %s" % (title, index)


def build_resolved_datasource(
    key: str,
    label: str,
    ds_type: str,
    input_label: Optional[str] = None,
) -> Dict[str, str]:
    """Create the normalized datasource descriptor used by prompt export helpers."""
    resolved = {"key": key, "label": label, "type": ds_type}
    if isinstance(input_label, str) and input_label:
        resolved["input_label"] = input_label
    return resolved


def lookup_datasource(
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
    uid: Optional[str] = None,
    name: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
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
    datasources_by_uid: Dict[str, Dict[str, Any]],
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
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Dict[str, str]:
    """Resolve string datasource references stored as names, UIDs, or type aliases."""
    datasource = lookup_datasource(
        datasources_by_uid,
        datasources_by_name,
        uid=ref,
        name=ref,
    )
    if datasource is None:
        datasource_type = resolve_datasource_type_alias(ref, datasources_by_uid)
        if datasource_type is not None:
            return build_resolved_datasource(
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
    return build_resolved_datasource(
        "uid:%s" % uid,
        label,
        ds_type,
        input_label=label,
    )


def resolve_placeholder_object_ref(
    uid: Any,
    name: Any,
    ds_type: Any,
) -> Optional[Dict[str, str]]:
    """Resolve object refs that already point at a datasource placeholder token."""
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
    return build_resolved_datasource(
        "var:%s:%s" % (ds_type, token),
        token,
        ds_type,
        input_label=format_plugin_name(ds_type),
    )


def resolve_object_datasource_ref(
    ref: Dict[str, Any],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[Dict[str, str]]:
    """Resolve object datasource refs stored as {'type': ..., 'uid': ...}."""
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

    return build_resolved_datasource(
        "uid:%s" % resolved_uid,
        resolved_label,
        resolved_type,
        input_label=resolved_label,
    )


def resolve_datasource_ref(
    ref: Any,
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Optional[Dict[str, str]]:
    """Normalize Grafana datasource references into stable keys for __inputs generation."""
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
    ref_mapping: Dict[str, Dict[str, str]],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
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
                    input_name = ref_mapping[resolved["key"]]["input_name"]
                    placeholder = "${%s}" % input_name
                    if isinstance(value, dict):
                        replacement = {"uid": placeholder}
                        ds_type = resolved.get("type")
                        if isinstance(ds_type, str) and ds_type:
                            replacement["type"] = ds_type
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
    dashboard: Dict[str, Any],
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
    panels: List[Dict[str, Any]],
    placeholder_names: Set[str],
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
    resolved: Dict[str, str],
    ref_mapping: Dict[str, Dict[str, str]],
    type_counts: Dict[str, int],
    key: Optional[str] = None,
) -> Dict[str, str]:
    """Create or reuse one __inputs mapping entry for a resolved datasource ref."""
    mapping_key = key or resolved["key"]
    mapping = ref_mapping.get(mapping_key)
    if mapping is not None:
        return mapping

    ds_type = resolved["type"]
    base_label = resolved.get("input_label") or format_plugin_name(ds_type)
    input_base = make_input_name(base_label)
    index = type_counts.get(input_base, 0) + 1
    type_counts[input_base] = index
    mapping = {
        "input_name": input_base if index == 1 else "%s_%s" % (input_base, index),
        "label": resolved.get("input_label") or make_input_label(ds_type, index),
        "type": ds_type,
        "plugin_name": format_plugin_name(ds_type),
    }
    ref_mapping[mapping_key] = mapping
    return mapping


def rewrite_template_variable_query(
    variable: Dict[str, Any],
    mapping: Dict[str, str],
    datasource_var_types: Dict[str, str],
    datasource_var_placeholders: Set[str],
) -> None:
    """Rewrite one datasource template variable into importer-friendly prompt form."""
    var_name = variable.get("name")
    if isinstance(var_name, str) and var_name:
        datasource_var_types[var_name] = mapping["type"]
        datasource_var_placeholders.add("$%s" % var_name)
        datasource_var_placeholders.add("${%s}" % var_name)

    variable["current"] = {}
    variable["options"] = []
    variable["query"] = mapping["type"]
    variable["refresh"] = 1
    variable["regex"] = variable.get("regex", "")
    if variable.get("hide") == 0:
        variable.pop("hide", None)


def rewrite_template_variable_datasource(
    variable: Dict[str, Any],
    datasource_var_types: Dict[str, str],
    datasource_var_placeholders: Set[str],
    datasource_var_input_names: Dict[str, str],
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
    dashboard: Dict[str, Any],
    ref_mapping: Dict[str, Dict[str, str]],
    type_counts: Dict[str, int],
    datasources_by_uid: Dict[str, Dict[str, Any]],
    datasources_by_name: Dict[str, Dict[str, Any]],
) -> Set[str]:
    """Rewrite datasource template variables so exported dashboards prompt on import."""
    templating = dashboard.get("templating")
    if not isinstance(templating, dict):
        return set()
    variables = templating.get("list")
    if not isinstance(variables, list):
        return set()

    datasource_var_types: Dict[str, str] = {}
    datasource_var_placeholders: Set[str] = set()
    datasource_var_input_names: Dict[str, str] = {}

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
            key="templating:%s" % (variable.get("name") or resolved["key"]),
        )
        rewrite_template_variable_query(
            variable,
            mapping,
            datasource_var_types,
            datasource_var_placeholders,
        )
        if isinstance(variable.get("name"), str) and variable.get("name"):
            datasource_var_input_names[str(variable["name"])] = mapping["input_name"]

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


def collect_panel_types(panels: List[Dict[str, Any]], panel_types: Set[str]) -> None:
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
    ref_mapping: Dict[str, Dict[str, str]],
) -> List[Dict[str, str]]:
    """Build Grafana's __inputs block from the resolved datasource mapping table."""
    return [
        {
            "name": mapping["input_name"],
            "label": mapping["label"],
            "description": "",
            "type": "datasource",
            "pluginId": mapping["type"],
            "pluginName": mapping["plugin_name"],
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1]["input_name"])
    ]


def build_requires_block(
    ref_mapping: Dict[str, Dict[str, str]],
    panel_types: Set[str],
) -> List[Dict[str, str]]:
    """Build Grafana's __requires block for Grafana itself, datasources, and panels."""
    requires = [{"type": "grafana", "id": "grafana", "name": "Grafana", "version": ""}]
    requires.extend(
        {
            "type": "datasource",
            "id": mapping["type"],
            "name": mapping["type"],
            "version": "",
        }
        for _, mapping in sorted(ref_mapping.items(), key=lambda item: item[1]["input_name"])
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


def build_preserved_web_import_document(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Keep the dashboard JSON Grafana expects for web import, but clear the numeric id."""
    dashboard = payload.get("dashboard", payload)
    if not isinstance(dashboard, dict):
        raise GrafanaError("Unexpected dashboard payload from Grafana.")
    document = copy.deepcopy(dashboard)
    document["id"] = None
    return document


def build_external_export_document(
    payload: Dict[str, Any],
    datasource_catalog: Tuple[Dict[str, Dict[str, Any]], Dict[str, Dict[str, Any]]],
) -> Dict[str, Any]:
    """Convert a fetched dashboard into Grafana's web-import prompt format."""
    dashboard = build_preserved_web_import_document(payload)

    datasources_by_uid, datasources_by_name = datasource_catalog
    refs: List[Any] = []
    collect_datasource_refs(dashboard, refs)

    ref_mapping: Dict[str, Dict[str, str]] = {}
    type_counts: Dict[str, int] = {}
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
        if resolved is None or resolved["key"] in ref_mapping:
            continue
        allocate_input_mapping(resolved, ref_mapping, type_counts)

    replace_datasource_refs_in_dashboard(
        dashboard,
        ref_mapping=ref_mapping,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )

    datasource_types = sorted({mapping["type"] for mapping in ref_mapping.values()})
    if len(datasource_types) == 1 and len(ref_mapping) == 1:
        ensure_datasource_template_variable(dashboard, datasource_types[0])
        placeholder_names = {
            "${%s}" % mapping["input_name"] for mapping in ref_mapping.values()
        }
        panels = dashboard.get("panels")
        if isinstance(panels, list):
            rewrite_panel_datasources_to_template_variable(
                [item for item in panels if isinstance(item, dict)],
                placeholder_names,
            )

    dashboard["__inputs"] = build_input_definitions(ref_mapping)

    panel_types: Set[str] = set()
    panels = dashboard.get("panels")
    if isinstance(panels, list):
        collect_panel_types(
            [item for item in panels if isinstance(item, dict)],
            panel_types,
        )
    dashboard["__requires"] = build_requires_block(ref_mapping, panel_types)
    dashboard["__elements"] = {}
    return dashboard
