//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::{
    attach_dashboard_folder_paths_with_request, build_export_metadata, build_export_variant_dirs,
    build_external_export_document, build_folder_inventory_status, build_folder_path,
    build_governance_gate_tui_groups, build_governance_gate_tui_items, build_impact_browser_items,
    build_impact_document, build_impact_tui_groups, build_import_auth_context,
    build_import_payload, build_output_path, build_preserved_web_import_document,
    build_root_export_index, build_topology_document, build_topology_tui_groups,
    diff_dashboards_with_request, discover_dashboard_files, export_dashboards_with_request,
    extract_dashboard_variables, filter_impact_tui_items, filter_topology_tui_items,
    format_dashboard_summary_line, format_export_progress_line, format_export_verbose_line,
    format_folder_inventory_status_line, format_import_progress_line, format_import_verbose_line,
    import_dashboards_with_org_clients, import_dashboards_with_request,
    list_dashboards_with_request, parse_cli_from, render_dashboard_governance_gate_result,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
    render_impact_text, render_import_dry_run_json, render_import_dry_run_table,
    render_topology_dot, render_topology_mermaid, CommonCliArgs, DashboardCliArgs,
    DashboardCommand, DashboardGovernanceGateFinding, DashboardGovernanceGateResult,
    DashboardGovernanceGateSummary, DiffArgs, ExportArgs, FolderInventoryStatusKind,
    GovernanceGateArgs, GovernanceGateOutputFormat, ImpactAlertResource, ImpactDashboard,
    ImpactDocument, ImpactOutputFormat, ImpactSummary, ImportArgs, InspectExportArgs,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, ListArgs, SimpleOutputFormat,
    TopologyDocument, TopologyOutputFormat, ValidationOutputFormat,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use super::{
    assert_all_orgs_export_live_documents_match, assert_governance_documents_match,
    export_query_row, load_inspection_analyzer_cases, load_prompt_export_cases,
    make_basic_common_args, make_common_args, make_import_args, sample_topology_tui_document,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use crate::dashboard::inspect::{
    dispatch_query_analysis, extract_query_field_and_text, resolve_query_analyzer_family,
    QueryAnalysis, QueryExtractionContext,
};
use crate::dashboard::inspect_governance::governance_risk_spec;
use clap::{CommandFactory, Parser};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn resolve_query_analyzer_family_from_datasource_type_maps_supported_aliases_to_families() {
    let cases = [
        (
            "prometheus",
            Some(test_support::DATASOURCE_FAMILY_PROMETHEUS),
        ),
        (
            "grafana-prometheus-datasource",
            Some(test_support::DATASOURCE_FAMILY_PROMETHEUS),
        ),
        ("loki", Some(test_support::DATASOURCE_FAMILY_LOKI)),
        (
            "grafana-loki-datasource",
            Some(test_support::DATASOURCE_FAMILY_LOKI),
        ),
        ("tempo", Some(test_support::DATASOURCE_FAMILY_TRACING)),
        ("jaeger", Some(test_support::DATASOURCE_FAMILY_TRACING)),
        ("zipkin", Some(test_support::DATASOURCE_FAMILY_TRACING)),
        ("influxdb", Some(test_support::DATASOURCE_FAMILY_FLUX)),
        (
            "grafana-influxdb-datasource",
            Some(test_support::DATASOURCE_FAMILY_FLUX),
        ),
        ("flux", Some(test_support::DATASOURCE_FAMILY_FLUX)),
        ("mysql", Some(test_support::DATASOURCE_FAMILY_SQL)),
        (
            "grafana-mysql-datasource",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        ("postgres", Some(test_support::DATASOURCE_FAMILY_SQL)),
        ("postgresql", Some(test_support::DATASOURCE_FAMILY_SQL)),
        (
            "grafana-postgresql-datasource",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        ("mssql", Some(test_support::DATASOURCE_FAMILY_SQL)),
        (
            "elasticsearch",
            Some(test_support::DATASOURCE_FAMILY_SEARCH),
        ),
        (
            "grafana-opensearch-datasource",
            Some(test_support::DATASOURCE_FAMILY_SEARCH),
        ),
        ("opensearch", Some(test_support::DATASOURCE_FAMILY_SEARCH)),
        ("sqlite", None),
        ("custom", None),
    ];

    for (datasource_type, expected) in cases {
        assert_eq!(
            test_support::resolve_query_analyzer_family_from_datasource_type(datasource_type),
            expected,
            "unexpected family for datasource type {datasource_type}"
        );
    }
}

#[test]
fn resolve_query_analyzer_family_from_query_signature_maps_fallback_and_search_shapes() {
    let cases = [
        (
            "rawSql",
            "SELECT * FROM cpu",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        (
            "sql",
            "SELECT * FROM cpu",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        (
            "logql",
            "{job=\"grafana\"}",
            Some(test_support::DATASOURCE_FAMILY_LOKI),
        ),
        (
            "expr",
            "up",
            Some(test_support::DATASOURCE_FAMILY_PROMETHEUS),
        ),
        (
            "query",
            "from(bucket: \"prod\") |> range(start: -1h)",
            Some(test_support::DATASOURCE_FAMILY_FLUX),
        ),
        (
            "query",
            "SELECT mean(value) FROM cpu",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        (
            "query",
            "update cpu set value = 1",
            Some(test_support::DATASOURCE_FAMILY_SQL),
        ),
        (
            "query",
            "_exists_:host.name AND host.name:api AND response.status:404",
            Some(test_support::DATASOURCE_FAMILY_SEARCH),
        ),
        ("query", "service.name:checkout AND trace.id=abc123", None),
        ("query", "up", None),
    ];

    for (query_field, query_text, expected) in cases {
        assert_eq!(
            test_support::resolve_query_analyzer_family_from_query_signature(
                query_field,
                query_text
            ),
            expected,
            "unexpected family for query_field={query_field} query_text={query_text}"
        );
    }
}

#[test]
fn dispatch_query_analysis_extracts_flux_every_window_hints() {
    let panel = Map::new();
    let target = Map::new();
    let context = test_support::QueryExtractionContext {
        panel: &panel,
        target: &target,
        query_field: "query",
        query_text:
            "from(bucket: \"prod\") |> range(start: -1h) |> aggregateWindow(every: 5m, fn: mean)",
        resolved_datasource_type: "flux",
    };

    assert_eq!(
        test_support::resolve_query_analyzer_family(&context),
        test_support::DATASOURCE_FAMILY_FLUX
    );
    assert_eq!(
        test_support::dispatch_query_analysis(&context).buckets,
        vec!["prod".to_string(), "5m".to_string()]
    );
}

#[test]
fn dispatch_query_analysis_ignores_flux_every_outside_window_calls() {
    let panel = Map::new();
    let target = Map::new();
    let context = test_support::QueryExtractionContext {
        panel: &panel,
        target: &target,
        query_field: "query",
        query_text:
            "option task = {name: \"cpu\", every: 1h}\nfrom(bucket: \"prod\") |> range(start: -1h)",
        resolved_datasource_type: "flux",
    };

    assert_eq!(
        test_support::dispatch_query_analysis(&context).buckets,
        vec!["prod".to_string()]
    );
}

#[test]
fn resolve_query_analyzer_family_prefers_target_datasource_then_panel_datasource() {
    let resolve = |panel_type: Option<&str>,
                   target_type: Option<&str>,
                   query_field: &'static str,
                   query_text: &'static str| {
        let panel_value = match panel_type {
            Some(value) => json!({"datasource": {"type": value}}),
            None => json!({}),
        };
        let target_value = match target_type {
            Some(value) => json!({"datasource": {"type": value}}),
            None => json!({}),
        };
        let panel = panel_value.as_object().unwrap();
        let target = target_value.as_object().unwrap();
        test_support::resolve_query_analyzer_family(&test_support::QueryExtractionContext {
            panel,
            target,
            query_field,
            query_text,
            resolved_datasource_type: "",
        })
    };

    let cases = [
        (
            Some("loki"),
            Some("prometheus"),
            "rawSql",
            "SELECT 1",
            test_support::DATASOURCE_FAMILY_PROMETHEUS,
        ),
        (
            Some("mssql"),
            Some("custom"),
            "expr",
            "up",
            test_support::DATASOURCE_FAMILY_SQL,
        ),
        (
            Some("loki"),
            None,
            "expr",
            "up",
            test_support::DATASOURCE_FAMILY_LOKI,
        ),
        (
            None,
            Some("grafana-postgresql-datasource"),
            "query",
            "up",
            test_support::DATASOURCE_FAMILY_SQL,
        ),
        (
            None,
            Some("flux"),
            "query",
            "SELECT * FROM cpu",
            test_support::DATASOURCE_FAMILY_FLUX,
        ),
        (
            Some("zipkin"),
            None,
            "query",
            "service.name:checkout",
            test_support::DATASOURCE_FAMILY_TRACING,
        ),
    ];

    for (panel_type, target_type, query_field, query_text, expected) in cases {
        assert_eq!(
            resolve(panel_type, target_type, query_field, query_text),
            expected,
            "unexpected family for panel={panel_type:?} target={target_type:?} query_field={query_field} query_text={query_text}"
        );
    }
}

#[test]
fn resolve_query_analyzer_family_prefers_inventory_resolved_datasource_type() {
    let panel = Map::new();
    let target = Map::from_iter(vec![
        (
            "datasource".to_string(),
            Value::Object(Map::from_iter(vec![(
                "uid".to_string(),
                Value::String("prom-main".to_string()),
            )])),
        ),
        ("query".to_string(), Value::String("up".to_string())),
    ]);
    let context = test_support::QueryExtractionContext {
        panel: &panel,
        target: &target,
        query_field: "query",
        query_text: "up",
        resolved_datasource_type: "prometheus",
    };

    assert_eq!(
        test_support::resolve_query_analyzer_family(&context),
        test_support::DATASOURCE_FAMILY_PROMETHEUS
    );
    assert_eq!(
        test_support::dispatch_query_analysis(&context).metrics,
        vec!["up".to_string()]
    );
}

#[test]
fn dispatch_query_analysis_extracts_obvious_tracing_field_hints() {
    let panel_value = json!({
        "datasource": {
            "type": "tempo"
        }
    });
    let target_value = json!({});
    let panel = panel_value.as_object().unwrap();
    let target = target_value.as_object().unwrap();
    let context = test_support::QueryExtractionContext {
        panel,
        target,
        query_field: "query",
        query_text: "service.name:checkout AND traceID=abc123 AND span.name:\"GET /orders\"",
        resolved_datasource_type: "tempo",
    };

    assert_eq!(
        test_support::resolve_query_analyzer_family(&context),
        test_support::DATASOURCE_FAMILY_TRACING
    );
    assert_eq!(
        test_support::dispatch_query_analysis(&context),
        QueryAnalysis {
            metrics: Vec::new(),
            functions: Vec::new(),
            measurements: vec![
                "service.name".to_string(),
                "traceID".to_string(),
                "span.name".to_string(),
            ],
            buckets: Vec::new(),
        }
    );
}

#[test]
fn dispatch_query_analysis_extracts_loki_pipeline_field_hints() {
    let panel_value = json!({
        "datasource": {
            "type": "loki"
        }
    });
    let target_value = json!({});
    let panel = panel_value.as_object().unwrap();
    let target = target_value.as_object().unwrap();
    let context = test_support::QueryExtractionContext {
        panel,
        target,
        query_field: "expr",
        query_text: "sum by (level) (count_over_time({job=\"grafana\"} |= \"timeout\" | json status >= 500 | logfmt level = \"error\" | unwrap duration_ms [5m]))",
        resolved_datasource_type: "loki",
    };

    assert_eq!(
        test_support::resolve_query_analyzer_family(&context),
        test_support::DATASOURCE_FAMILY_LOKI
    );
    assert_eq!(
        test_support::dispatch_query_analysis(&context),
        QueryAnalysis {
            metrics: Vec::new(),
            functions: vec![
                "sum".to_string(),
                "count_over_time".to_string(),
                "json".to_string(),
                "logfmt".to_string(),
                "unwrap".to_string(),
                "line_filter_contains".to_string(),
                "line_filter_contains:timeout".to_string(),
            ],
            measurements: vec![
                "{job=\"grafana\"}".to_string(),
                "job=\"grafana\"".to_string(),
                "status".to_string(),
                "level".to_string(),
                "duration_ms".to_string(),
            ],
            buckets: vec!["5m".to_string()],
        }
    );
}

#[test]
fn dispatch_query_analysis_keeps_tracing_family_conservative_for_plain_text() {
    let panel_value = json!({
        "datasource": {
            "type": "zipkin"
        }
    });
    let target_value = json!({});
    let panel = panel_value.as_object().unwrap();
    let target = target_value.as_object().unwrap();
    let context = test_support::QueryExtractionContext {
        panel,
        target,
        query_field: "query",
        query_text: "trace workflow text with no obvious fields",
        resolved_datasource_type: "zipkin",
    };

    assert_eq!(
        test_support::resolve_query_analyzer_family(&context),
        test_support::DATASOURCE_FAMILY_TRACING
    );
    assert_eq!(
        test_support::dispatch_query_analysis(&context),
        QueryAnalysis::default()
    );
}

#[test]
fn resolve_query_analyzer_family_routes_elasticsearch_and_opensearch_to_search_family() {
    let cases = [
        ("elasticsearch", Some("prometheus"), Some("loki")),
        ("opensearch", Some("prometheus"), Some("loki")),
    ];

    for (resolved_datasource_type, panel_type, target_type) in cases {
        let panel_value = match panel_type {
            Some(value) => json!({"datasource": {"type": value}}),
            None => json!({}),
        };
        let target_value = match target_type {
            Some(value) => json!({"datasource": {"type": value}}),
            None => json!({}),
        };
        let panel = panel_value.as_object().unwrap();
        let target = target_value.as_object().unwrap();
        let context = test_support::QueryExtractionContext {
            panel,
            target,
            query_field: "query",
            query_text: "status:500",
            resolved_datasource_type,
        };

        assert_eq!(
            test_support::resolve_query_analyzer_family(&context),
            test_support::DATASOURCE_FAMILY_SEARCH,
            "unexpected family for resolved_datasource_type={resolved_datasource_type}"
        );
        assert_eq!(
            test_support::dispatch_query_analysis(&context),
            QueryAnalysis {
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: vec!["status".to_string()],
                buckets: Vec::new(),
            }
        );
    }
}

#[test]
fn dispatch_query_analysis_for_search_family_stays_conservative() {
    let cases = [
        (
            "elasticsearch",
            "status:500 AND status:500 AND _exists_:trace.id AND service.name:count AND category:rate",
            vec![
                "trace.id".to_string(),
                "status".to_string(),
                "service.name".to_string(),
                "category".to_string(),
            ],
        ),
        (
            "opensearch",
            "_exists_:host.name AND host.name:api AND response.status:404 AND response.status:404 AND level:error",
            vec![
                "host.name".to_string(),
                "response.status".to_string(),
                "level".to_string(),
            ],
        ),
    ];

    for (resolved_datasource_type, query_text, expected_measurements) in cases {
        let panel = Map::new();
        let target = Map::new();
        let context = test_support::QueryExtractionContext {
            panel: &panel,
            target: &target,
            query_field: "query",
            query_text,
            resolved_datasource_type,
        };

        assert_eq!(
            test_support::resolve_query_analyzer_family(&context),
            test_support::DATASOURCE_FAMILY_SEARCH,
            "unexpected family for resolved_datasource_type={resolved_datasource_type}"
        );
        assert_eq!(
            test_support::dispatch_query_analysis(&context),
            QueryAnalysis {
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: expected_measurements,
                buckets: Vec::new(),
            },
            "unexpected analysis for resolved_datasource_type={resolved_datasource_type}"
        );
    }
}

#[test]
fn dispatch_query_analysis_extracts_search_field_references_from_lucene_queries() {
    let panel_value = json!({
        "datasource": {
            "type": "elasticsearch"
        }
    });
    let target_value = json!({});
    let panel = panel_value.as_object().unwrap();
    let target = target_value.as_object().unwrap();

    let cases = [
        ("status:500", vec!["status"]),
        ("service.name:\"api\"", vec!["service.name"]),
        ("_exists_:traceId", vec!["traceId"]),
        (
            "@timestamp:[now-15m TO now] AND level:error",
            vec!["@timestamp", "level"],
        ),
        (
            "_exists_:@timestamp AND service.name:\"api\"",
            vec!["@timestamp", "service.name"],
        ),
        (
            "status:500 AND service.name:\"api\"",
            vec!["status", "service.name"],
        ),
    ];

    for (query_text, expected_measurements) in cases {
        let context = test_support::QueryExtractionContext {
            panel,
            target,
            query_field: "query",
            query_text,
            resolved_datasource_type: "elasticsearch",
        };

        assert_eq!(
            test_support::resolve_query_analyzer_family(&context),
            test_support::DATASOURCE_FAMILY_SEARCH,
            "unexpected family for query_text={query_text}"
        );
        assert_eq!(
            test_support::dispatch_query_analysis(&context),
            QueryAnalysis {
                metrics: Vec::new(),
                functions: Vec::new(),
                measurements: expected_measurements
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<String>>(),
                buckets: Vec::new(),
            },
            "unexpected analysis for query_text={query_text}"
        );
    }
}

#[test]
fn dispatch_query_analysis_keeps_search_family_conservative_for_non_lucene_shapes() {
    let panel_value = json!({
        "datasource": {
            "type": "elasticsearch"
        }
    });
    let target_value = json!({});
    let panel = panel_value.as_object().unwrap();
    let target = target_value.as_object().unwrap();

    let cases = [
        "{\"query\":{\"match\":{\"status\":\"500\"}}}",
        "source=logs | where status=500",
    ];

    for query_text in cases {
        let context = test_support::QueryExtractionContext {
            panel,
            target,
            query_field: "query",
            query_text,
            resolved_datasource_type: "elasticsearch",
        };

        assert_eq!(
            test_support::dispatch_query_analysis(&context),
            QueryAnalysis::default(),
            "unexpected analysis for query_text={query_text}"
        );
    }
}

#[test]
fn normalize_family_name_covers_core_family_aliases() {
    let cases = [
        ("prometheus", "prometheus"),
        ("grafana-prometheus-datasource", "prometheus"),
        ("loki", "loki"),
        ("grafana-loki-datasource", "loki"),
        ("influxdb", "flux"),
        ("grafana-influxdb-datasource", "flux"),
        ("flux", "flux"),
        ("mysql", "sql"),
        ("grafana-mysql-datasource", "sql"),
        ("postgres", "sql"),
        ("grafana-postgresql-datasource", "sql"),
        ("mssql", "sql"),
        ("elasticsearch", "search"),
        ("opensearch", "search"),
        ("grafana-opensearch-datasource", "search"),
        ("tempo", "tracing"),
        ("grafana-tempo-datasource", "tracing"),
        ("jaeger", "tracing"),
        ("zipkin", "tracing"),
        ("custom", "custom"),
    ];

    for (datasource_type, expected) in cases {
        assert_eq!(
            test_support::normalize_family_name(datasource_type),
            expected,
            "unexpected normalized family for datasource_type={datasource_type}"
        );
    }
}
