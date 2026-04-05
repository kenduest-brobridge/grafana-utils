//! Strict dashboard export validation.
//! Validates raw dashboard export files for schema migration, legacy properties, and custom
//! plugin usage before GitOps-style import or sync.
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

use crate::common::{
    message, render_json_value, should_print_stdout, string_field, value_as_object,
    write_plain_output_file, Result,
};

use super::{
    discover_dashboard_files, extract_dashboard_object, load_json_file, ValidateExportArgs,
};

const CORE_PANEL_TYPES: &[&str] = &[
    "alertlist",
    "bargauge",
    "candlestick",
    "dashlist",
    "gauge",
    "geomap",
    "graph",
    "heatmap",
    "histogram",
    "logs",
    "news",
    "nodeGraph",
    "piechart",
    "row",
    "state-timeline",
    "stat",
    "status-history",
    "table",
    "text",
    "timeseries",
    "traces",
    "trend",
    "xychart",
];

const CORE_DATASOURCE_TYPES: &[&str] = &[
    "alertmanager",
    "cloud-monitoring",
    "elasticsearch",
    "grafana",
    "graphite",
    "influxdb",
    "jaeger",
    "loki",
    "mysql",
    "opensearch",
    "postgres",
    "prometheus",
    "tempo",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardValidationIssue {
    pub(crate) severity: &'static str,
    pub(crate) code: &'static str,
    pub(crate) file: String,
    pub(crate) dashboard_uid: String,
    pub(crate) dashboard_title: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardValidationResult {
    pub(crate) dashboard_count: usize,
    pub(crate) error_count: usize,
    pub(crate) warning_count: usize,
    pub(crate) issues: Vec<DashboardValidationIssue>,
}

fn push_issue(
    issues: &mut Vec<DashboardValidationIssue>,
    severity: &'static str,
    code: &'static str,
    file: &Path,
    dashboard_uid: &str,
    dashboard_title: &str,
    message_text: impl Into<String>,
) {
    issues.push(DashboardValidationIssue {
        severity,
        code,
        file: file.display().to_string(),
        dashboard_uid: dashboard_uid.to_string(),
        dashboard_title: dashboard_title.to_string(),
        message: message_text.into(),
    });
}

fn walk_panels<'a>(panels: &'a [Value], collected: &mut Vec<&'a Map<String, Value>>) {
    for panel in panels {
        let Some(panel_object) = panel.as_object() else {
            continue;
        };
        collected.push(panel_object);
        if let Some(children) = panel_object.get("panels").and_then(Value::as_array) {
            walk_panels(children, collected);
        }
    }
}

fn collect_dashboard_panels(dashboard: &Map<String, Value>) -> Vec<&Map<String, Value>> {
    let mut panels = Vec::new();
    if let Some(items) = dashboard.get("panels").and_then(Value::as_array) {
        walk_panels(items, &mut panels);
    }
    panels
}

fn collect_dashboard_datasource_types(
    dashboard: &Map<String, Value>,
    panels: &[&Map<String, Value>],
) -> BTreeSet<String> {
    let mut types = BTreeSet::new();
    let collect_from_value = |value: Option<&Value>, collected: &mut BTreeSet<String>| {
        let Some(value) = value else {
            return;
        };
        match value {
            Value::Object(object) => {
                let datasource_type = string_field(object, "type", "");
                if !datasource_type.is_empty() {
                    collected.insert(datasource_type);
                }
            }
            Value::String(text) if !text.is_empty() => {
                collected.insert(text.clone());
            }
            _ => {}
        }
    };

    collect_from_value(dashboard.get("datasource"), &mut types);
    for panel in panels {
        collect_from_value(panel.get("datasource"), &mut types);
        if let Some(targets) = panel.get("targets").and_then(Value::as_array) {
            for target in targets {
                collect_from_value(target.get("datasource"), &mut types);
            }
        }
    }
    types
}

fn validate_dashboard_document(
    document: &Value,
    file: &Path,
    reject_custom_plugins: bool,
    reject_legacy_properties: bool,
    target_schema_version: Option<i64>,
) -> Result<Vec<DashboardValidationIssue>> {
    let document_object = value_as_object(document, "Dashboard payload must be a JSON object.")?;
    let dashboard = extract_dashboard_object(document_object)?;
    let dashboard_uid = string_field(dashboard, "uid", "");
    let dashboard_title = string_field(dashboard, "title", "");
    let dashboard_label = if dashboard_uid.is_empty() {
        dashboard_title.as_str()
    } else {
        dashboard_uid.as_str()
    };
    let mut issues = Vec::new();

    let schema_version = dashboard.get("schemaVersion").and_then(Value::as_i64);
    match schema_version {
        Some(version) => {
            if let Some(target_version) = target_schema_version {
                if version < target_version {
                    push_issue(
                        &mut issues,
                        "error",
                        "schema-migration-required",
                        file,
                        &dashboard_uid,
                        &dashboard_title,
                        format!(
                            "Dashboard {dashboard_label} schemaVersion {version} is below required target {target_version}."
                        ),
                    );
                }
            }
        }
        None => push_issue(
            &mut issues,
            "error",
            "missing-schema-version",
            file,
            &dashboard_uid,
            &dashboard_title,
            format!("Dashboard {dashboard_label} is missing dashboard.schemaVersion."),
        ),
    }

    if document_object.contains_key("__inputs") {
        push_issue(
            &mut issues,
            "error",
            "web-import-placeholders",
            file,
            &dashboard_uid,
            &dashboard_title,
            format!("Dashboard {dashboard_label} still contains Grafana web-import placeholders (__inputs)."),
        );
    }

    if reject_legacy_properties {
        if document_object.contains_key("__requires") {
            push_issue(
                &mut issues,
                "error",
                "legacy-web-import-requires",
                file,
                &dashboard_uid,
                &dashboard_title,
                format!("Dashboard {dashboard_label} still contains top-level __requires web-import scaffolding."),
            );
        }
        if dashboard.contains_key("rows") {
            push_issue(
                &mut issues,
                "error",
                "legacy-row-layout",
                file,
                &dashboard_uid,
                &dashboard_title,
                format!("Dashboard {dashboard_label} uses legacy rows layout. Re-save or migrate before sync."),
            );
        }
    }

    if reject_custom_plugins {
        let panels = collect_dashboard_panels(dashboard);
        for panel in &panels {
            let panel_type = string_field(panel, "type", "");
            if !panel_type.is_empty() && !CORE_PANEL_TYPES.contains(&panel_type.as_str()) {
                push_issue(
                    &mut issues,
                    "error",
                    "custom-panel-plugin",
                    file,
                    &dashboard_uid,
                    &dashboard_title,
                    format!("Dashboard {dashboard_label} uses unsupported custom panel plugin type {panel_type}."),
                );
            }
        }
        for datasource_type in collect_dashboard_datasource_types(dashboard, &panels) {
            if datasource_type.starts_with("-- ") || datasource_type == "__expr__" {
                continue;
            }
            if !CORE_DATASOURCE_TYPES.contains(&datasource_type.as_str()) {
                push_issue(
                    &mut issues,
                    "error",
                    "custom-datasource-plugin",
                    file,
                    &dashboard_uid,
                    &dashboard_title,
                    format!("Dashboard {dashboard_label} references unsupported datasource plugin type {datasource_type}."),
                );
            }
        }
    }

    Ok(issues)
}

pub(crate) fn validate_dashboard_export_dir(
    import_dir: &Path,
    reject_custom_plugins: bool,
    reject_legacy_properties: bool,
    target_schema_version: Option<i64>,
) -> Result<DashboardValidationResult> {
    let mut issues = Vec::new();
    let files = discover_dashboard_files(import_dir)?;
    for dashboard_file in &files {
        let document = load_json_file(dashboard_file)?;
        issues.extend(validate_dashboard_document(
            &document,
            dashboard_file,
            reject_custom_plugins,
            reject_legacy_properties,
            target_schema_version,
        )?);
    }
    let error_count = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warning_count = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();
    Ok(DashboardValidationResult {
        dashboard_count: files.len(),
        error_count,
        warning_count,
        issues,
    })
}

fn validation_result_document(result: &DashboardValidationResult) -> Value {
    let issues = result
        .issues
        .iter()
        .map(|issue| {
            serde_json::json!({
                "severity": issue.severity,
                "code": issue.code,
                "file": issue.file,
                "dashboardUid": issue.dashboard_uid,
                "dashboardTitle": issue.dashboard_title,
                "message": issue.message,
            })
        })
        .collect::<Vec<Value>>();
    serde_json::json!({
        "kind": "grafana-utils-dashboard-validation",
        "schemaVersion": 1,
        "summary": {
            "dashboardCount": result.dashboard_count,
            "errorCount": result.error_count,
            "warningCount": result.warning_count,
        },
        "issues": issues,
    })
}

fn render_validation_result_text(result: &DashboardValidationResult) -> Vec<String> {
    let mut lines = vec![
        "Dashboard validation".to_string(),
        format!("Dashboards: {}", result.dashboard_count),
        format!("Errors: {}", result.error_count),
        format!("Warnings: {}", result.warning_count),
    ];
    if !result.issues.is_empty() {
        lines.push(String::new());
        lines.push("# Issues".to_string());
        for issue in &result.issues {
            lines.push(format!(
                "[{}] {} {} {}",
                issue.severity.to_uppercase(),
                issue.code,
                issue.dashboard_uid,
                issue.message
            ));
        }
    }
    lines
}

pub(crate) fn render_validation_result_json(result: &DashboardValidationResult) -> Result<String> {
    Ok(format!(
        "{}\n",
        render_json_value(&validation_result_document(result))?
    ))
}

pub(crate) fn run_dashboard_validate_export(args: &ValidateExportArgs) -> Result<()> {
    let temp_dir = super::inspect::TempInspectDir::new("validate-export")?;
    let import_dir = super::inspect::resolve_inspect_export_import_dir(
        &temp_dir.path,
        &args.import_dir,
        args.input_format,
        None,
        false,
    )?;
    let result = validate_dashboard_export_dir(
        &import_dir.import_dir,
        args.reject_custom_plugins,
        args.reject_legacy_properties,
        args.target_schema_version,
    )?;
    let output = match args.output_format {
        super::ValidationOutputFormat::Text => {
            format!("{}\n", render_validation_result_text(&result).join("\n"))
        }
        super::ValidationOutputFormat::Json => render_validation_result_json(&result)?,
    };
    if let Some(path) = args.output_file.as_ref() {
        write_plain_output_file(path, &output)?;
    }
    if should_print_stdout(args.output_file.as_deref(), args.also_stdout) {
        print!("{output}");
    }
    if result.error_count > 0 {
        return Err(message(format!(
            "Dashboard validation found {} blocking issue(s).",
            result.error_count
        )));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::dashboard::{DashboardImportInputFormat, ValidationOutputFormat};
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn write_valid_dashboard(path: &Path, uid: &str, title: &str) {
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "uid": uid,
                "title": title,
                "schemaVersion": 39,
                "panels": []
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn run_dashboard_validate_export_supports_provisioning_root() {
        let temp = tempdir().unwrap();
        let provisioning_root = temp.path().join("provisioning");
        let dashboards_dir = provisioning_root.join("dashboards/team");
        fs::create_dir_all(&dashboards_dir).unwrap();
        write_valid_dashboard(
            &dashboards_dir.join("cpu-main.json"),
            "cpu-main",
            "CPU Main",
        );
        let output_file = temp.path().join("validation.json");

        run_dashboard_validate_export(&ValidateExportArgs {
            import_dir: provisioning_root,
            input_format: DashboardImportInputFormat::Provisioning,
            reject_custom_plugins: true,
            reject_legacy_properties: true,
            target_schema_version: Some(39),
            output_format: ValidationOutputFormat::Json,
            output_file: Some(output_file.clone()),
            also_stdout: false,
        })
        .unwrap();

        let report = fs::read_to_string(output_file).unwrap();
        assert!(report.contains("\"dashboardCount\": 1"));
        assert!(report.contains("\"errorCount\": 0"));
    }

    #[test]
    fn run_dashboard_validate_export_supports_provisioning_dashboards_dir() {
        let temp = tempdir().unwrap();
        let provisioning_root = temp.path().join("provisioning");
        let dashboards_dir = provisioning_root.join("dashboards");
        fs::create_dir_all(&dashboards_dir).unwrap();
        write_valid_dashboard(
            &dashboards_dir.join("ops-main.json"),
            "ops-main",
            "Ops Main",
        );
        let output_file = temp.path().join("validation.json");

        run_dashboard_validate_export(&ValidateExportArgs {
            import_dir: dashboards_dir,
            input_format: DashboardImportInputFormat::Provisioning,
            reject_custom_plugins: false,
            reject_legacy_properties: false,
            target_schema_version: None,
            output_format: ValidationOutputFormat::Json,
            output_file: Some(output_file.clone()),
            also_stdout: false,
        })
        .unwrap();

        let report = fs::read_to_string(output_file).unwrap();
        assert!(
            report.contains("\"dashboardUid\": \"ops-main\"") || report.contains("\"issues\": []")
        );
        assert!(report.contains("\"dashboardCount\": 1"));
    }
}

pub(crate) fn validate_dashboard_import_document(
    document: &Value,
    file: &Path,
    strict: bool,
    target_schema_version: Option<i64>,
) -> Result<()> {
    let issues =
        validate_dashboard_document(document, file, strict, strict, target_schema_version)?;
    let blocking = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .collect::<Vec<_>>();
    if blocking.is_empty() {
        return Ok(());
    }
    let first = blocking[0];
    Err(message(format!(
        "Refusing dashboard import because strict schema validation failed in {}: {} ({})",
        first.file, first.message, first.code
    )))
}
