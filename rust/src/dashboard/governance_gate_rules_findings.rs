use serde_json::Value;

use crate::common::{message, Result};

use super::string_field;
use crate::dashboard::governance_gate::DashboardGovernanceGateFinding;

pub(crate) fn build_governance_warning_findings(
    governance_document: &Value,
) -> Result<Vec<DashboardGovernanceGateFinding>> {
    let risk_records = governance_document
        .get("riskRecords")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Dashboard governance JSON must contain a riskRecords array."))?;
    Ok(risk_records
        .iter()
        .map(|record| DashboardGovernanceGateFinding {
            severity: "warning".to_string(),
            code: string_field(record, "kind"),
            message: record
                .get("recommendation")
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    let detail = string_field(record, "detail");
                    if detail.is_empty() {
                        "Governance warning surfaced from inspect report.".to_string()
                    } else {
                        detail
                    }
                }),
            dashboard_uid: string_field(record, "dashboardUid"),
            dashboard_title: String::new(),
            panel_id: string_field(record, "panelId"),
            panel_title: String::new(),
            ref_id: String::new(),
            datasource: string_field(record, "datasource"),
            datasource_uid: String::new(),
            datasource_family: String::new(),
            risk_kind: string_field(record, "kind"),
        })
        .collect::<Vec<DashboardGovernanceGateFinding>>())
}
