"""Richer offline inspection models and extraction helpers for dependency work.

This module defines typed rows for dependency-oriented outputs and the extraction
logic needed for future report modes without forcing current inspectors to change
their execution flow.
"""

from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass, field
import re
from typing import Any, Iterable, Mapping

from .reference_models import DatasourceReference, collect_datasource_reference_index


_PROMQL_METRIC_RE = re.compile(r"\b([a-zA-Z_:][a-zA-Z0-9_:]*)\s*(?:\(|\[)")
_SQL_FROM_RE = re.compile(r"(?i)\bfrom\s+([`\"'\\[]?[a-zA-Z0-9_\\.:-]+[`\"'\\]]?)")
_SQL_JOIN_RE = re.compile(r"(?i)\bjoin\s+([`\"'\\[]?[a-zA-Z0-9_\\.:-]+[`\"'\\]]?)")
_LOKI_SELECTOR_RE = re.compile(r"\{([^{}]+)\}")
_LOKI_PIPE_RE = re.compile(r"\|\s*([a-zA-Z0-9_]+)")
_FLUX_FROM_RE = re.compile(r"(?i)\bfrom\(\s*([^)]+)\s*\)")
_FLUX_RANGE_RE = re.compile(r"(?i)\b(?:range|window)\s*\(\s*([^)]+)\)")


def _coerce_text(value: Any) -> str:
    """Convert generic input into a normalized text value."""
    if value is None:
        return ""
    text = str(value).strip()
    return text


def _coerce_list(values: Any) -> list[str]:
    """Normalize list-like input into unique, sorted string entries."""
    if not isinstance(values, (list, tuple)):
        text = _coerce_text(values)
        return [text] if text else []
    output = []
    seen = set()
    for value in values:
        normalized = _coerce_text(value)
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        output.append(normalized)
    return sorted(output)


def _normalize_family(datasource_type: str, datasource_family: str = "") -> str:
    """Prefer explicit family metadata and normalize to stable families."""
    if datasource_family:
        return _coerce_text(datasource_family).lower()
    text = _coerce_text(datasource_type).lower()
    aliases = {
        "grafana-prometheus-datasource": "prometheus",
        "grafana-loki-datasource": "loki",
        "grafana-influxdb-flux-datasource": "flux",
        "grafana-influxdb-datasource": "influxdb",
        "grafana-mysql-datasource": "mysql",
        "grafana-postgresql-datasource": "postgresql",
    }
    return aliases.get(text, text or "unknown")


def _extract_prometheus_features(query: str) -> dict[str, list[str]]:
    """Extract a minimal feature bundle from PromQL-like query text."""
    metrics = _coerce_list(_PROMQL_METRIC_RE.findall(query))
    if not metrics:
        metrics = _coerce_list(re.findall(r"\b([a-zA-Z_:][a-zA-Z0-9_:]*)", query))
    buckets = []
    lower = query.lower()
    for token in ("sum", "avg", "rate", "histogram_quantile", "count_over_time"):
        if f"{token}(" in lower:
            buckets.append(token)
    return {
        "metrics": metrics,
        "measurements": [],
        "buckets": sorted(set(buckets)),
        "labels": [],
        "aggregations": [],
    }


def _extract_loki_features(query: str) -> dict[str, list[str]]:
    """Extract stream selectors, filters, and pipeline stages from Loki query text."""
    selectors = _coerce_list(_LOKI_SELECTOR_RE.findall(query))
    pipes = []
    for stage in _LOKI_PIPE_RE.findall(query):
        stage_norm = _coerce_text(stage)
        if stage_norm and stage_norm not in pipes:
            pipes.append(stage_norm)
    labels = []
    for selector in selectors:
        for part in selector.split(","):
            bit = part.strip()
            if bit:
                labels.append(bit)
    return {
        "metrics": [],
        "measurements": [],
        "buckets": [],
        "labels": sorted(set(labels)),
        "aggregations": pipes,
    }


def _extract_flux_features(query: str) -> dict[str, list[str]]:
    """Extract high-signal Flux tokens (bucket-like windows and source selectors)."""
    measurements = []
    for match in _FLUX_FROM_RE.findall(query):
        text = _coerce_text(match)
        if text:
            measurements.append(text)
    buckets = []
    for match in _FLUX_RANGE_RE.findall(query):
        text = _coerce_text(match)
        if text:
            buckets.append(text)
    return {
        "metrics": [],
        "measurements": sorted(set(measurements)),
        "buckets": sorted(set(buckets)),
        "labels": [],
        "aggregations": [],
    }


def _extract_sql_features(query: str) -> dict[str, list[str]]:
    """Extract SQL table/relation references from FROM/JOIN clauses."""
    measurements = []
    froms = [match.strip("`\"'[]") for match in _SQL_FROM_RE.findall(query)]
    joins = [match.strip("`\"'[]") for match in _SQL_JOIN_RE.findall(query)]
    for table in list(froms) + list(joins):
        text = _coerce_text(table)
        if text:
            measurements.append(text)
    return {
        "metrics": [],
        "measurements": sorted(set(measurements)),
        "buckets": [],
        "labels": [],
        "aggregations": [],
    }


def _extract_features(datasource_family: str, query: str) -> dict[str, list[str]]:
    """Choose a parser strategy by datasource family."""
    # Call graph: see callers/callees.
    #   Upstream callers: 184
    #   Downstream callees: 109, 130, 51, 67, 86

    family = _normalize_family("", datasource_family)
    if family in ("prometheus", "graphite", "victoriametrics"):
        return _extract_prometheus_features(query)
    if family == "loki":
        return _extract_loki_features(query)
    if family == "flux":
        return _extract_flux_features(query)
    if family in ("sql", "mysql", "postgresql", "postgres"):
        return _extract_sql_features(query)
    return {
        "metrics": [],
        "measurements": [],
        "buckets": [],
        "labels": [],
        "aggregations": [],
    }


def _coerce_record_text(value: Mapping[str, Any], key: str) -> str:
    """Read a required string field from a generic mapping safely."""
    return _coerce_text(value.get(key))


@dataclass(frozen=True)
class QueryFeatureSet:
    """Per-query feature extraction result."""

    metrics: list[str] = field(default_factory=list)
    measurements: list[str] = field(default_factory=list)
    buckets: list[str] = field(default_factory=list)
    labels: list[str] = field(default_factory=list)
    aggregations: list[str] = field(default_factory=list)

    @classmethod
    def from_query(cls, family: str, query: str) -> "QueryFeatureSet":
        """Build one feature set from query text and family hints."""
        return cls(**_extract_features(family, query))

    def as_dict(self) -> dict[str, list[str]]:
        """Render feature fields in report-style mapping form."""
        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        return {
            "metrics": list(self.metrics),
            "measurements": list(self.measurements),
            "buckets": list(self.buckets),
            "labels": list(self.labels),
            "aggregations": list(self.aggregations),
        }


@dataclass(frozen=True)
class DependencyQueryRecord:
    """One enriched query-row candidate for dependency reporting."""

    dashboard_uid: str
    dashboard_title: str
    folder_path: str
    panel_id: str
    panel_title: str
    panel_type: str
    ref_id: str
    datasource_identity: str
    datasource_uid: str
    datasource_type: str
    datasource_family: str
    file: str
    query_field: str
    query: str
    features: QueryFeatureSet

    def as_dict(self) -> dict[str, Any]:
        """Serialize one dependency row in a stable dict shape."""
        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        return {
            "dashboardUid": self.dashboard_uid,
            "dashboardTitle": self.dashboard_title,
            "folderPath": self.folder_path,
            "panelId": self.panel_id,
            "panelTitle": self.panel_title,
            "panelType": self.panel_type,
            "refId": self.ref_id,
            "datasource": self.datasource_identity,
            "datasourceUid": self.datasource_uid,
            "datasourceType": self.datasource_type,
            "datasourceFamily": self.datasource_family,
            "file": self.file,
            "queryField": self.query_field,
            "query": self.query,
            "analysis": self.features.as_dict(),
        }


@dataclass(frozen=True)
class DatasourceUsageSummary:
    """Roll-up usage summary per datasource identity."""

    datasource_identity: str
    family: str
    reference_count: int
    query_count: int
    dashboard_count: int
    query_fields: list[str]

    def as_dict(self) -> dict[str, Any]:
        """Serialize one usage summary row."""
        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        return {
            "datasource": self.datasource_identity,
            "family": self.family,
            "referenceCount": self.reference_count,
            "queryCount": self.query_count,
            "dashboardCount": self.dashboard_count,
            "queryFields": list(self.query_fields),
        }


@dataclass(frozen=True)
class OfflineDependencyReport:
    """Bundle for richer dependency inspection outputs."""

    summary: dict[str, int]
    queries: list[DependencyQueryRecord]
    usage: list[DatasourceUsageSummary]
    orphaned: list[DatasourceReference]

    def to_dict(self) -> dict[str, Any]:
        """Serialize the report as plain JSON-compatible mappings."""
        # Call graph: see callers/callees.
        #   Upstream callers: 無
        #   Downstream callees: 無

        return {
            "summary": dict(self.summary),
            "queries": [item.as_dict() for item in self.queries],
            "datasourceUsage": [item.as_dict() for item in self.usage],
            "orphanedDatasources": [item.as_dict() for item in self.orphaned],
        }


def build_dependency_rows_from_query_report(
    query_rows: Iterable[Mapping[str, Any]],
    datasource_inventory: Iterable[Mapping[str, Any]] = (),
) -> OfflineDependencyReport:
    """Build a richer dependency report from raw query rows and inventory rows."""
    # Call graph: see callers/callees.
    #   Upstream callers: 無
    #   Downstream callees: 168, 184, 51

    datasource_map = collect_datasource_reference_index(list(datasource_inventory))
    query_records = []
    usage: OrderedDict[str, dict[str, Any]] = OrderedDict()
    used_datasources = set()

    for raw in query_rows:
        dashboard_uid = _coerce_record_text(raw, "dashboardUid")
        dashboard_title = _coerce_record_text(raw, "dashboardTitle")
        folder_path = _coerce_record_text(raw, "folderPath")
        panel_id = _coerce_record_text(raw, "panelId")
        panel_title = _coerce_record_text(raw, "panelTitle")
        panel_type = _coerce_record_text(raw, "panelType")
        ref_id = _coerce_record_text(raw, "refId")
        datasource_label = _coerce_record_text(raw, "datasource")
        datasource_uid = _coerce_record_text(raw, "datasourceUid")
        identity = datasource_uid or datasource_label or "unknown"
        datasource_type = _coerce_record_text(raw, "datasourceType")
        datasource_family = _coerce_record_text(raw, "datasourceFamily")
        query_field = _coerce_record_text(raw, "queryField")
        query = _coerce_record_text(raw, "query")

        features = QueryFeatureSet.from_query(
            _normalize_family(datasource_type, datasource_family),
            query,
        )
        reference_record = DependencyQueryRecord(
            dashboard_uid=dashboard_uid or "unknown",
            dashboard_title=dashboard_title or dashboard_uid or "unknown",
            folder_path=folder_path,
            panel_id=panel_id or "unknown",
            panel_title=panel_title,
            panel_type=panel_type,
            ref_id=ref_id,
            datasource_identity=identity,
            datasource_uid=datasource_uid,
            datasource_type=datasource_type,
            datasource_family=_normalize_family(datasource_type, datasource_family),
            file=_coerce_record_text(raw, "file"),
            query_field=query_field,
            query=query,
            features=features,
        )
        query_records.append(reference_record)
        usage_entry = usage.setdefault(
            identity,
            {
                "identity": identity,
                "family": _normalize_family(datasource_type, datasource_family),
                "reference_count": 0,
                "query_count": 0,
                "dashboards": set(),
                "query_fields": set(),
            },
        )
        usage_entry["query_count"] += 1
        usage_entry["reference_count"] += 1
        usage_entry["query_fields"].add(query_field)
        usage_entry["dashboards"].add(dashboard_uid or "unknown")
        used_datasources.add(identity)

    usage_records = []
    for identity, values in usage.items():
        usage_records.append(
            DatasourceUsageSummary(
                datasource_identity=identity,
                family=str(values["family"]),
                reference_count=int(values["reference_count"]),
                query_count=int(values["query_count"]),
                dashboard_count=len(values["dashboards"]),
                query_fields=sorted(values["query_fields"]),
            )
        )
    orphaned = [
        item
        for key, item in sorted(datasource_map.items())
        if key not in used_datasources
    ]

    summary = OrderedDict(
        [
            ("queryCount", len(query_records)),
            ("datasourceCount", len(usage_records)),
            ("orphanedCount", len(orphaned)),
            ("dashboardCount", len({item.dashboard_uid for item in query_records})),
        ]
    )
    return OfflineDependencyReport(
        summary=dict(summary),
        queries=query_records,
        usage=usage_records,
        orphaned=orphaned,
    )
