"""
Dispatcher for dashboard query analyzers by datasource family.
"""

from typing import Any, Iterable, Optional

from . import flux, generic, loki, prometheus, sql
from .contract import (
    DATASOURCE_FAMILY_FLUX,
    DATASOURCE_FAMILY_LOKI,
    DATASOURCE_FAMILY_PROMETHEUS,
    DATASOURCE_FAMILY_SQL,
    DATASOURCE_FAMILY_UNKNOWN,
    normalize_query_analysis,
)

FAMILY_ANALYZERS = {
    DATASOURCE_FAMILY_PROMETHEUS: prometheus.analyze_query,
    DATASOURCE_FAMILY_LOKI: loki.analyze_query,
    DATASOURCE_FAMILY_FLUX: flux.analyze_query,
    DATASOURCE_FAMILY_SQL: sql.analyze_query,
    DATASOURCE_FAMILY_UNKNOWN: generic.analyze_query,
}


def iter_datasource_ref_parts(ref: Any) -> Iterable[str]:
    """Yield normalized datasource reference parts from a target definition."""
    if isinstance(ref, str):
        text = ref.strip().lower()
        if text:
            yield text
        return
    if not isinstance(ref, dict):
        return
    for key in ("type", "uid", "name"):
        text = str(ref.get(key) or "").strip().lower()
        if text:
            yield text
def iter_inventory_datasource_parts(ref: Any, datasources_by_uid: Optional[dict[str, dict[str, str]]], datasources_by_name: Optional[dict[str, dict[str, str]]]) -> Iterable[str]:
    """Yield datasource attributes resolved from local inventory maps."""
    datasource = None
    if isinstance(ref, dict):
        uid = str(ref.get("uid") or "").strip()
        name = str(ref.get("name") or "").strip()
        if uid and datasources_by_uid:
            datasource = datasources_by_uid.get(uid)
        if datasource is None and name and datasources_by_name:
            datasource = datasources_by_name.get(name)
    elif isinstance(ref, str) and datasources_by_name:
        datasource = datasources_by_name.get(ref.strip())
    if datasource is not None:
        datasource_type = str(datasource.get("type") or "").strip().lower()
        if datasource_type:
            yield datasource_type
def resolve_query_analyzer_family(panel: dict[str, Any], target: dict[str, Any], query_field: str, query_text: str, datasources_by_uid: Optional[dict[str, dict[str, str]]] = None, datasources_by_name: Optional[dict[str, dict[str, str]]] = None) -> str:
    """Determine query family (prometheus/loki/flux/sql/unknown)."""
    hints = []
    for ref in (target.get("datasource"), panel.get("datasource")):
        hints.extend(list(iter_datasource_ref_parts(ref)))
        hints.extend(list(iter_inventory_datasource_parts(ref, datasources_by_uid, datasources_by_name)))
    field_hint = str(query_field or "").strip().lower()
    hints.append(field_hint)
    text = str(query_text or "").strip().lower()
    if "loki" in hints or field_hint == "logql":
        return DATASOURCE_FAMILY_LOKI
    if "prometheus" in hints or "prom" in hints or field_hint == "expr":
        return DATASOURCE_FAMILY_PROMETHEUS
    if "flux" in hints or "influxdb" in hints or "influx" in hints or "_measurement" in text or "from(bucket:" in text or "from (bucket:" in text:
        return DATASOURCE_FAMILY_FLUX
    if "mysql" in hints or "postgres" in hints or "postgresql" in hints or "mssql" in hints or field_hint in ("rawsql", "sql"):
        return DATASOURCE_FAMILY_SQL
    return DATASOURCE_FAMILY_UNKNOWN
def dispatch_query_analysis(
    panel: dict[str, Any],
    target: dict[str, Any],
    query_field: str,
    query_text: str,
    datasources_by_uid: Optional[dict[str, dict[str, str]]] = None,
    datasources_by_name: Optional[dict[str, dict[str, str]]] = None,
) -> dict[str, Any]:
    """Select an analyzer by family and normalize its query-analysis output."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 55

    family = resolve_query_analyzer_family(
        panel,
        target,
        query_field,
        query_text,
        datasources_by_uid=datasources_by_uid,
        datasources_by_name=datasources_by_name,
    )
    analyzer = FAMILY_ANALYZERS.get(family)
    if analyzer is None:
        analyzer = generic.analyze_query
    return normalize_query_analysis(
        analyzer(
            panel=panel,
            target=target,
            query_field=query_field,
            query_text=query_text,
        )
    )
