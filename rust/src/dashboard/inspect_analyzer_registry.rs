//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

use crate::dashboard::inspect::{
    extract_metric_names, extract_query_buckets, extract_query_measurements,
    resolve_query_analyzer_family,
};
use crate::dashboard::inspect_analyzer_flux;
use crate::dashboard::inspect_analyzer_loki;
use crate::dashboard::inspect_analyzer_prometheus;
use crate::dashboard::inspect_analyzer_search;
use crate::dashboard::inspect_analyzer_sql;
use crate::dashboard::prompt::datasource_type_alias;

pub(crate) const DATASOURCE_FAMILY_PROMETHEUS: &str = "prometheus";
pub(crate) const DATASOURCE_FAMILY_LOKI: &str = "loki";
pub(crate) const DATASOURCE_FAMILY_FLUX: &str = "flux";
pub(crate) const DATASOURCE_FAMILY_SQL: &str = "sql";
pub(crate) const DATASOURCE_FAMILY_SEARCH: &str = "search";
pub(crate) const DATASOURCE_FAMILY_TRACING: &str = "tracing";
pub(crate) const DATASOURCE_FAMILY_UNKNOWN: &str = "unknown";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueryAnalysis {
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

pub(crate) struct QueryExtractionContext<'a> {
    pub(crate) panel: &'a Map<String, Value>,
    pub(crate) target: &'a Map<String, Value>,
    pub(crate) query_field: &'a str,
    pub(crate) query_text: &'a str,
    pub(crate) resolved_datasource_type: &'a str,
}

pub(crate) fn ordered_unique_push(values: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }
    if !values.iter().any(|value| value == trimmed) {
        values.push(trimmed.to_string());
    }
}

fn canonicalize_query_analyzer_datasource_type(datasource_type: &str) -> &str {
    let datasource_type = datasource_type_alias(datasource_type);
    if let Some(normalized) = datasource_type
        .strip_prefix("grafana-")
        .and_then(|value| value.strip_suffix("-datasource"))
    {
        return normalized;
    }
    if let Some(normalized) = datasource_type.strip_suffix("-datasource") {
        return normalized;
    }
    datasource_type
}

pub(crate) fn resolve_query_analyzer_family_from_datasource_type(
    datasource_type: &str,
) -> Option<&'static str> {
    match canonicalize_query_analyzer_datasource_type(datasource_type) {
        "loki" => Some(DATASOURCE_FAMILY_LOKI),
        "prometheus" => Some(DATASOURCE_FAMILY_PROMETHEUS),
        "tempo" | "jaeger" | "zipkin" => Some(DATASOURCE_FAMILY_TRACING),
        "influxdb" | "flux" => Some(DATASOURCE_FAMILY_FLUX),
        "mysql" | "postgres" | "postgresql" | "mssql" => Some(DATASOURCE_FAMILY_SQL),
        "elasticsearch" | "opensearch" => Some(DATASOURCE_FAMILY_SEARCH),
        _ => None,
    }
}

pub(crate) fn resolve_query_analyzer_family_from_query_signature(
    query_field: &str,
    query_text: &str,
) -> Option<&'static str> {
    if matches!(query_field, "rawSql" | "sql") {
        return Some(DATASOURCE_FAMILY_SQL);
    }
    if query_field == "logql" {
        return Some(DATASOURCE_FAMILY_LOKI);
    }
    if query_field == "expr" {
        return Some(DATASOURCE_FAMILY_PROMETHEUS);
    }
    let trimmed = query_text.trim_start();
    if trimmed.starts_with("from(") || trimmed.starts_with("from (") || trimmed.contains("|>") {
        return Some(DATASOURCE_FAMILY_FLUX);
    }
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("select ")
        || lowered.starts_with("with ")
        || lowered.starts_with("insert ")
        || lowered.starts_with("update ")
        || lowered.starts_with("delete ")
    {
        return Some(DATASOURCE_FAMILY_SQL);
    }
    if inspect_analyzer_search::query_text_looks_like_search(query_text) {
        return Some(DATASOURCE_FAMILY_SEARCH);
    }
    None
}

fn tracing_field_hint_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)(?:^|[^A-Za-z0-9_.-])((?:service\.name|span\.name|resource\.service\.name|trace\.id|traceID|traceId))\s*(?:=|:)\s*(?:"(?:\\.|[^"\\])*"|\[[^\]]*\]|\{[^}]*\}|\([^\)]*\)|[A-Za-z0-9_*?.-]+)"#,
        )
        .expect("invalid hard-coded tracing field regex")
    })
}

fn extract_tracing_measurements(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for captures in tracing_field_hint_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

pub(crate) fn dispatch_query_analysis(context: &QueryExtractionContext<'_>) -> QueryAnalysis {
    // Dispatch order matters: an explicit datasource family wins first, then query
    // signature heuristics fill gaps for sparse or partially-typed dashboard targets.
    match resolve_query_analyzer_family(context) {
        DATASOURCE_FAMILY_PROMETHEUS => inspect_analyzer_prometheus::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_LOKI => inspect_analyzer_loki::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_FLUX => inspect_analyzer_flux::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_SQL => inspect_analyzer_sql::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_SEARCH => inspect_analyzer_search::analyze_query(
            context.panel,
            context.target,
            context.query_field,
            context.query_text,
        ),
        DATASOURCE_FAMILY_TRACING => QueryAnalysis {
            metrics: Vec::new(),
            functions: Vec::new(),
            measurements: extract_tracing_measurements(context.query_text),
            buckets: Vec::new(),
        },
        _ => QueryAnalysis {
            metrics: extract_metric_names(context.query_text),
            functions: Vec::new(),
            measurements: extract_query_measurements(context.target, context.query_text),
            buckets: extract_query_buckets(context.target, context.query_text),
        },
    }
}
