"""Typed models for dashboard and datasource references.

This module intentionally contains only data-shape definitions and pure
normalization helpers so it can be wired in later without changing current
inspection execution paths.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterable, Mapping, Sequence


def _coerce_text(value: Any) -> str:
    """Return a trimmed string, or an empty string for null-ish values."""
    if value is None:
        return ""
    text = str(value).strip()
    return text


def _collect_unique(values: Iterable[Any]) -> list[str]:
    """Return stable deduplicated values while preserving first-seen ordering."""
    seen = set()
    result = []
    for value in values:
        value = _coerce_text(value)
        if not value:
            continue
        if value in seen:
            continue
        seen.add(value)
        result.append(value)
    return result


@dataclass(frozen=True)
class DatasourceReference:
    """Stable representation for one datasource reference."""

    uid: str
    name: str
    datasource_type: str = ""
    plugin_id: str = ""
    org: str = ""
    org_id: str = ""
    access: str = ""
    url: str = ""
    source_file: str = ""

    @property
    def stable_identity(self) -> str:
        """Prefer uid, then name for deterministic reference joins."""
        return self.uid or self.name or "unknown"

    @staticmethod
    def from_mapping(value: Mapping[str, Any]) -> "DatasourceReference":
        """Build one typed datasource reference from a raw JSON-like dict."""
        uid = _coerce_text(value.get("uid"))
        name = _coerce_text(value.get("name"))
        return DatasourceReference(
            uid=uid or name,
            name=name or uid,
            datasource_type=_coerce_text(value.get("type")),
            plugin_id=_coerce_text(value.get("pluginId") or value.get("type")),
            org=_coerce_text(value.get("org")),
            org_id=_coerce_text(value.get("orgId")),
            access=_coerce_text(value.get("access")),
            url=_coerce_text(value.get("url")),
            source_file=_coerce_text(value.get("sourcePath")),
        )

    def as_dict(self) -> dict[str, str]:
        """Render a stable JSON-friendly mapping for report consumers."""
        return {
            "uid": self.uid,
            "name": self.name,
            "type": self.datasource_type,
            "pluginId": self.plugin_id,
            "org": self.org,
            "orgId": self.org_id,
            "access": self.access,
            "url": self.url,
            "sourceFile": self.source_file,
        }


@dataclass(frozen=True)
class DashboardReference:
    """Stable representation for one dashboard artifact."""

    uid: str
    title: str
    folder_path: str = ""
    file: str = ""
    org: str = ""
    org_id: str = ""

    @staticmethod
    def from_mapping(value: Mapping[str, Any]) -> "DashboardReference":
        """Build one typed dashboard reference from a raw JSON-like dict."""
        uid = _coerce_text(value.get("uid"))
        title = _coerce_text(value.get("title"))
        return DashboardReference(
            uid=uid or "unknown",
            title=title or uid or "unknown",
            folder_path=_coerce_text(value.get("folderPath") or value.get("folder")),
            file=_coerce_text(value.get("file")),
            org=_coerce_text(value.get("org")),
            org_id=_coerce_text(value.get("orgId")),
        )


@dataclass(frozen=True)
class PanelReference:
    """Stable representation for one dashboard panel location."""

    dashboard_uid: str
    panel_id: str
    ref_id: str = ""
    panel_type: str = ""
    title: str = ""
    file: str = ""

    @staticmethod
    def from_mapping(value: Mapping[str, Any], dashboard_uid: str) -> "PanelReference":
        """Build one typed panel reference from panel content."""
        panel_id = _coerce_text(value.get("id") or value.get("panelId"))
        return PanelReference(
            dashboard_uid=dashboard_uid,
            panel_id=panel_id or "unknown",
            ref_id=_coerce_text(value.get("refId")),
            panel_type=_coerce_text(value.get("type")),
            title=_coerce_text(value.get("title")),
            file=_coerce_text(value.get("file")),
        )


@dataclass(frozen=True)
class DashboardQueryReference:
    """Stable representation for one dashboard query reference."""

    dashboard: DashboardReference
    panel: PanelReference
    datasource: DatasourceReference
    query_field: str = ""
    query: str = ""

    @property
    def dashboard_uid(self) -> str:
        """Shortcut for joining summaries grouped by dashboard uid."""
        return self.dashboard.uid

    @staticmethod
    def from_mappings(
        dashboard: Mapping[str, Any],
        panel: Mapping[str, Any],
        datasource: Mapping[str, Any],
        query_record: Mapping[str, Any],
    ) -> "DashboardQueryReference":
        """Build one query reference row using only explicit string fields."""
        dashboard_ref = DashboardReference.from_mapping(dashboard)
        panel_ref = PanelReference.from_mapping(
            {
                "id": query_record.get("panelId") or panel.get("id"),
                "refId": query_record.get("refId") or panel.get("refId"),
                "type": query_record.get("panelType") or panel.get("type"),
                "title": query_record.get("panelTitle") or panel.get("title"),
                "file": query_record.get("file"),
            },
            dashboard_uid=dashboard_ref.uid,
        )
        return DashboardQueryReference(
            dashboard=dashboard_ref,
            panel=panel_ref,
            datasource=DatasourceReference.from_mapping(datasource),
            query_field=_coerce_text(query_record.get("queryField")),
            query=_coerce_text(query_record.get("query")),
        )

    def as_dict(self) -> dict[str, str]:
        """Expose a merged dictionary that maps to current inspection consumers."""
        return {
            "dashboardUid": self.dashboard.uid,
            "dashboardTitle": self.dashboard.title,
            "folderPath": self.dashboard.folder_path,
            "panelId": self.panel.panel_id,
            "panelTitle": self.panel.title,
            "panelType": self.panel.panel_type,
            "refId": self.panel.ref_id,
            "file": self.panel.file,
            "datasourceUid": self.datasource.uid,
            "datasource": self.datasource.name,
            "datasourceType": self.datasource.datasource_type,
            "datasourceFamily": _coerce_text(self.datasource.datasource_type).lower(),
            "queryField": self.query_field,
            "query": self.query,
        }


def collect_datasource_reference_index(
    records: Sequence[Mapping[str, Any]],
) -> dict[str, DatasourceReference]:
    """Build an index by stable datasource identity for quick joins."""
    index: dict[str, DatasourceReference] = {}
    for record in records:
        reference = DatasourceReference.from_mapping(record)
        key = reference.stable_identity
        if key:
            index[key] = reference
    return index


def extract_dashboard_reference_sequence(
    records: Sequence[Mapping[str, Any]],
) -> list[DashboardReference]:
    """Normalize dashboard artifact rows to stable dashboard references."""
    return [DashboardReference.from_mapping(record) for record in records]


def dedupe_text_sequence(values: Sequence[Any]) -> list[str]:
    """Public dedupe helper for downstream report builders."""
    return _collect_unique(values)
