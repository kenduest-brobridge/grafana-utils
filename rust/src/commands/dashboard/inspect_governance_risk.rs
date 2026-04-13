//! Risk and audit row builders for dashboard inspect governance output.
//! Keeps scoring, deduping, and risk metadata out of the facade module.

#[path = "inspect_governance_risk_rows.rs"]
mod inspect_governance_risk_rows;
#[path = "inspect_governance_risk_spec.rs"]
mod inspect_governance_risk_spec;

pub(crate) use inspect_governance_risk_rows::{
    build_dashboard_audit_rows, build_governance_risk_rows, build_query_audit_rows,
    find_broad_loki_selector,
};
#[cfg(test)]
pub(crate) use inspect_governance_risk_spec::governance_risk_spec;
