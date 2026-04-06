//! Shared source resolution for dashboard analysis commands.

use serde_json::Value;
use std::path::Path;

use crate::common::{load_json_object_file, message, Result};

use super::cli_defs::{CommonCliArgs, DashboardImportInputFormat, InspectExportInputType};
use super::export::export_dashboards_with_org_clients;
use super::inspect::{
    build_export_inspection_query_report_for_variant, build_export_inspection_summary_for_variant,
    resolve_inspect_export_import_dir,
};
use super::inspect_governance::build_export_inspection_governance_document;
use super::inspect_live::{
    build_analysis_live_export_args, prepare_live_analysis_import_dir, TempInspectDir,
};
use super::inspect_report::build_export_inspection_query_report_document;
use super::ExportArgs;

#[derive(Debug)]
pub(crate) struct DashboardAnalysisArtifacts {
    pub(crate) governance: Value,
    pub(crate) queries: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardAnalysisSourceArgs<'a> {
    pub(crate) common: &'a CommonCliArgs,
    pub(crate) page_size: usize,
    pub(crate) org_id: Option<i64>,
    pub(crate) all_orgs: bool,
    pub(crate) import_dir: Option<&'a Path>,
    pub(crate) input_format: DashboardImportInputFormat,
    pub(crate) input_type: Option<InspectExportInputType>,
    pub(crate) governance: Option<&'a Path>,
    pub(crate) queries: Option<&'a Path>,
    pub(crate) require_queries: bool,
}

fn load_object(path: &Path, label: &str) -> Result<Value> {
    load_json_object_file(path, label)
}

fn build_artifacts_from_export_dir(
    import_dir: &Path,
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
) -> Result<DashboardAnalysisArtifacts> {
    let temp_dir = TempInspectDir::new("dashboard-analysis-source")?;
    let resolved =
        resolve_inspect_export_import_dir(&temp_dir.path, import_dir, input_format, input_type, false)?;
    let summary =
        build_export_inspection_summary_for_variant(&resolved.import_dir, resolved.expected_variant)?;
    let report =
        build_export_inspection_query_report_for_variant(&resolved.import_dir, resolved.expected_variant)?;
    Ok(DashboardAnalysisArtifacts {
        governance: serde_json::to_value(build_export_inspection_governance_document(
            &summary, &report,
        ))?,
        queries: serde_json::to_value(build_export_inspection_query_report_document(&report))?,
    })
}

fn build_artifacts_from_live(source: &DashboardAnalysisSourceArgs<'_>) -> Result<DashboardAnalysisArtifacts> {
    let temp_dir = TempInspectDir::new("dashboard-analysis-live")?;
    let export_args: ExportArgs = build_analysis_live_export_args(
        source.common,
        temp_dir.path.clone(),
        source.page_size,
        source.org_id,
        source.all_orgs,
    );
    let _ = export_dashboards_with_org_clients(&export_args)?;
    let import_dir = prepare_live_analysis_import_dir(&temp_dir.path, source.all_orgs)?;
    build_artifacts_from_export_dir(
        &import_dir,
        DashboardImportInputFormat::Raw,
        Some(InspectExportInputType::Raw),
    )
}

pub(crate) fn resolve_dashboard_analysis_artifacts(
    source: &DashboardAnalysisSourceArgs<'_>,
) -> Result<DashboardAnalysisArtifacts> {
    if let Some(import_dir) = source.import_dir {
        if source.governance.is_some() || source.queries.is_some() {
            return Err(message(
                "--import-dir cannot be combined with --governance or --queries.",
            ));
        }
        return build_artifacts_from_export_dir(import_dir, source.input_format, source.input_type);
    }

    if source.governance.is_some() || source.queries.is_some() {
        let governance_path = source.governance.ok_or_else(|| {
            message("--governance is required when reusing prebuilt analysis artifacts.")
        })?;
        if source.require_queries && source.queries.is_none() {
            return Err(message(
                "--queries is required when reusing artifact inputs for governance-gate.",
            ));
        }
        let governance = load_object(governance_path, "Dashboard governance JSON")?;
        let queries = match source.queries {
            Some(path) => load_object(path, "Dashboard query report JSON")?,
            None => Value::Null,
        };
        return Ok(DashboardAnalysisArtifacts { governance, queries });
    }

    build_artifacts_from_live(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CliColorChoice;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn make_common_args() -> CommonCliArgs {
        CommonCliArgs {
            color: CliColorChoice::Never,
            profile: None,
            url: "http://127.0.0.1:3000".to_string(),
            api_token: None,
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        }
    }

    fn write_basic_raw_export(
        raw_dir: &Path,
        dashboard_uid: &str,
        dashboard_title: &str,
        datasource_uid: &str,
    ) {
        fs::create_dir_all(raw_dir).unwrap();
        fs::write(
            raw_dir.join("export-metadata.json"),
            serde_json::to_string_pretty(&json!({
                "kind": "grafana-utils-dashboard-export-index",
                "schemaVersion": 1,
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "format": "grafana-web-import-preserve-uid",
                "foldersFile": "folders.json",
                "datasourcesFile": "datasources.json",
                "org": "Main Org",
                "orgId": "1"
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("folders.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": "general",
                    "title": "General",
                    "path": "General",
                    "org": "Main Org",
                    "orgId": "1"
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("datasources.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": datasource_uid,
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://grafana.example.internal",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("index.json"),
            serde_json::to_string_pretty(&json!([
                {
                    "uid": dashboard_uid,
                    "title": dashboard_title,
                    "path": "dash.json",
                    "format": "grafana-web-import-preserve-uid",
                    "org": "Main Org",
                    "orgId": "1"
                }
            ]))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            raw_dir.join("dash.json"),
            serde_json::to_string_pretty(&json!({
                "dashboard": {
                    "id": null,
                    "uid": dashboard_uid,
                    "title": dashboard_title,
                    "schemaVersion": 38,
                    "templating": {
                        "list": [
                            { "name": "env", "type": "query", "datasource": { "uid": datasource_uid, "type": "prometheus" } }
                        ]
                    },
                    "panels": [{
                        "id": 7,
                        "title": "CPU",
                        "type": "timeseries",
                        "datasource": {"uid": datasource_uid, "type": "prometheus"},
                        "targets": [{
                            "refId": "A",
                            "expr": "sum(rate(cpu_seconds_total[5m]))"
                        }]
                    }]
                },
                "meta": {
                    "folderUid": "general",
                    "folderTitle": "General"
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn resolve_dashboard_analysis_artifacts_from_import_dir_builds_documents() {
        let temp = tempdir().unwrap();
        let raw_dir = temp.path().join("raw");
        write_basic_raw_export(&raw_dir, "cpu-main", "CPU Main", "prom-main");
        let common = make_common_args();

        let artifacts = resolve_dashboard_analysis_artifacts(&DashboardAnalysisSourceArgs {
            common: &common,
            page_size: 500,
            org_id: None,
            all_orgs: false,
            import_dir: Some(&raw_dir),
            input_format: DashboardImportInputFormat::Raw,
            input_type: Some(InspectExportInputType::Raw),
            governance: None,
            queries: None,
            require_queries: false,
        })
        .unwrap();

        assert_eq!(artifacts.governance["summary"]["dashboardCount"], json!(1));
        assert_eq!(artifacts.queries["summary"]["dashboardCount"], json!(1));
        assert_eq!(artifacts.queries["queries"][0]["dashboardUid"], json!("cpu-main"));
    }

    #[test]
    fn resolve_dashboard_analysis_artifacts_requires_queries_for_gate_artifacts() {
        let temp = tempdir().unwrap();
        let governance_path = temp.path().join("governance.json");
        fs::write(
            &governance_path,
            serde_json::to_string_pretty(&json!({"summary": {"dashboardCount": 1}})).unwrap(),
        )
        .unwrap();
        let common = make_common_args();

        let error = resolve_dashboard_analysis_artifacts(&DashboardAnalysisSourceArgs {
            common: &common,
            page_size: 500,
            org_id: None,
            all_orgs: false,
            import_dir: None,
            input_format: DashboardImportInputFormat::Raw,
            input_type: None,
            governance: Some(&governance_path),
            queries: None,
            require_queries: true,
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("--queries is required when reusing artifact inputs for governance-gate"));
    }
}
