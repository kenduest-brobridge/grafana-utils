use serde_json::Value;

use super::super::string_field;
use crate::common::{message, Result};
use crate::dashboard::governance_gate::DashboardGovernanceGateFinding;

pub(super) fn build_query_violation(
    code: &str,
    message_text: String,
    query: &Value,
) -> DashboardGovernanceGateFinding {
    DashboardGovernanceGateFinding {
        severity: "error".to_string(),
        code: code.to_string(),
        message: message_text,
        dashboard_uid: string_field(query, "dashboardUid"),
        dashboard_title: string_field(query, "dashboardTitle"),
        panel_id: string_field(query, "panelId"),
        panel_title: string_field(query, "panelTitle"),
        ref_id: string_field(query, "refId"),
        datasource: string_field(query, "datasource"),
        datasource_uid: string_field(query, "datasourceUid"),
        datasource_family: string_field(query, "datasourceFamily"),
        risk_kind: String::new(),
    }
}

pub(super) fn build_dashboard_violation(
    code: &str,
    message_text: String,
    dashboard: &Value,
) -> DashboardGovernanceGateFinding {
    DashboardGovernanceGateFinding {
        severity: "error".to_string(),
        code: code.to_string(),
        message: message_text,
        dashboard_uid: string_field(dashboard, "dashboardUid"),
        dashboard_title: string_field(dashboard, "dashboardTitle"),
        panel_id: String::new(),
        panel_title: String::new(),
        ref_id: String::new(),
        datasource: String::new(),
        datasource_uid: String::new(),
        datasource_family: String::new(),
        risk_kind: String::new(),
    }
}

pub(super) fn build_dashboard_violation_from_fields(
    code: &str,
    message_text: String,
    dashboard_uid: String,
    dashboard_title: String,
) -> DashboardGovernanceGateFinding {
    DashboardGovernanceGateFinding {
        severity: "error".to_string(),
        code: code.to_string(),
        message: message_text,
        dashboard_uid,
        dashboard_title,
        panel_id: String::new(),
        panel_title: String::new(),
        ref_id: String::new(),
        datasource: String::new(),
        datasource_uid: String::new(),
        datasource_family: String::new(),
        risk_kind: String::new(),
    }
}

pub(super) fn array_of_objects<'a>(document: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    document.get(key).and_then(Value::as_array).ok_or_else(|| {
        message(format!(
            "Dashboard governance JSON must contain a {key} array."
        ))
    })
}
