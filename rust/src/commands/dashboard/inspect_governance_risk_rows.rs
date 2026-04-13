//! Risk and audit row builders for dashboard inspect governance output.
//! Facade over the query-family heuristics and row assembly helpers.

#[path = "inspect_governance_risk_rows_builders.rs"]
mod inspect_governance_risk_rows_builders;
#[path = "inspect_governance_risk_rows_query_helpers.rs"]
mod inspect_governance_risk_rows_query_helpers;

pub(crate) use inspect_governance_risk_rows_builders::{
    build_dashboard_audit_rows, build_governance_risk_rows, build_query_audit_rows,
};
pub(crate) use inspect_governance_risk_rows_query_helpers::find_broad_loki_selector;
