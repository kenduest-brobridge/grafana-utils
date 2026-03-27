//! Dashboard inspect extraction facade.
//!
//! Keeps datasource reference helpers and query-text extraction helpers in focused sibling
//! modules while preserving the existing `inspect.rs` re-export surface.
#[path = "inspect_extract_datasource.rs"]
mod inspect_extract_datasource;
#[path = "inspect_extract_query.rs"]
mod inspect_extract_query;

pub(crate) use inspect_extract_datasource::{
    resolve_datasource_inventory_item, resolve_query_analyzer_family, summarize_datasource_name,
    summarize_datasource_ref, summarize_datasource_type, summarize_datasource_uid,
    summarize_panel_datasource_key,
};
pub(crate) use inspect_extract_query::{
    extract_flux_pipeline_functions, extract_influxql_select_functions,
    extract_influxql_select_metrics, extract_influxql_time_windows, extract_metric_names,
    extract_prometheus_functions, extract_prometheus_metric_names,
    extract_prometheus_range_windows, extract_query_buckets, extract_query_field_and_text,
    extract_query_measurements, extract_sql_query_shape_hints, extract_sql_source_references,
};
