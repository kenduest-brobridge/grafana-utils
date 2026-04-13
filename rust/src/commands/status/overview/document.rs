//! Overview document assembly and text rendering.

use super::{
    overview_kind::parse_overview_artifact_kind, overview_sections::build_overview_summary_item,
    overview_summary_projection::build_overview_summary, OverviewArtifact, OverviewDocument,
    OVERVIEW_KIND, OVERVIEW_SCHEMA_VERSION,
};
use crate::common::{message, tool_version, Result};
use crate::project_status::{
    render_domain_finding_summary, render_project_status_decision_order,
    render_project_status_signal_summary,
};
use crate::project_status_staged::build_staged_project_status;
use crate::sync::render_discovery_summary_from_value;
use serde_json::Value;

pub(crate) fn build_overview_document(
    artifacts: Vec<OverviewArtifact>,
) -> Result<OverviewDocument> {
    if artifacts.is_empty() {
        return Err(message("Overview requires at least one input artifact."));
    }
    for artifact in &artifacts {
        if artifact.title.trim().is_empty() {
            return Err(message("Overview artifacts require a title."));
        }
        parse_overview_artifact_kind(&artifact.kind)?;
    }
    let sections = super::overview_sections::build_overview_sections(&artifacts)?;
    Ok(OverviewDocument {
        kind: OVERVIEW_KIND.to_string(),
        schema_version: OVERVIEW_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        discovery: None,
        summary: build_overview_summary(&artifacts)?,
        project_status: build_staged_project_status(&artifacts),
        artifacts,
        selected_section_index: 0,
        sections,
    })
}

pub(crate) fn render_overview_text(document: &OverviewDocument) -> Result<Vec<String>> {
    if document.kind != OVERVIEW_KIND {
        return Err(message("Overview document kind is not supported."));
    }
    let mut lines = vec![
        "Project overview".to_string(),
        format!(
            "Status: {} domains={} present={} blocked={} blockers={} warnings={} freshness={} oldestAge={}s",
            document.project_status.overall.status,
            document.project_status.overall.domain_count,
            document.project_status.overall.present_count,
            document.project_status.overall.blocked_count,
            document.project_status.overall.blocker_count,
            document.project_status.overall.warning_count,
            document.project_status.overall.freshness.status,
            document
                .project_status
                .overall
                .freshness
                .oldest_age_seconds
                .unwrap_or(0),
        ),
        format!(
            "Artifacts: {} total, {} dashboard export, {} datasource export, {} alert export, {} access user export, {} access team export, {} access org export, {} access service-account export, {} sync summary, {} bundle preflight, {} promotion preflight",
            document.summary.artifact_count,
            document.summary.dashboard_export_count,
            document.summary.datasource_export_count,
            document.summary.alert_export_count,
            document.summary.access_user_export_count,
            document.summary.access_team_export_count,
            document.summary.access_org_export_count,
            document.summary.access_service_account_export_count,
            document.summary.sync_summary_count,
            document.summary.bundle_preflight_count,
            document.summary.promotion_preflight_count,
        ),
    ];
    if let Some(discovery) = document.discovery.as_ref().and_then(Value::as_object) {
        if let Some(summary) = render_discovery_summary_from_value(discovery) {
            lines.push(summary);
        }
    }
    if let Some(summary) = render_project_status_signal_summary(&document.project_status) {
        lines.push(summary);
    }
    if let Some(order) = render_project_status_decision_order(&document.project_status) {
        lines.push("Decision order:".to_string());
        lines.extend(order);
    }
    if !document.project_status.domains.is_empty() {
        lines.push("Domain status:".to_string());
        for domain in &document.project_status.domains {
            let mut line = format!(
                "- {} status={} reason={} primary={} blockers={} warnings={} freshness={}",
                domain.id,
                domain.status,
                domain.reason_code,
                domain.primary_count,
                domain.blocker_count,
                domain.warning_count,
                domain.freshness.status,
            );
            if let Some(action) = domain.next_actions.first() {
                line.push_str(&format!(" next={action}"));
            }
            if let Some(summary) = render_domain_finding_summary(&domain.blockers) {
                line.push_str(&format!(" blockerKinds={summary}"));
            }
            if let Some(summary) = render_domain_finding_summary(&domain.warnings) {
                line.push_str(&format!(" warningKinds={summary}"));
            }
            lines.push(line);
        }
    }
    if !document.project_status.top_blockers.is_empty() {
        lines.push("Top blockers:".to_string());
        for blocker in document.project_status.top_blockers.iter().take(5) {
            lines.push(format!(
                "- {} {} count={} source={}",
                blocker.domain, blocker.kind, blocker.count, blocker.source
            ));
        }
    }
    if !document.project_status.next_actions.is_empty() {
        lines.push("Next actions:".to_string());
        for action in document.project_status.next_actions.iter().take(5) {
            lines.push(format!(
                "- {} reason={} action={}",
                action.domain, action.reason_code, action.action
            ));
        }
    }
    for artifact in &document.artifacts {
        let item = build_overview_summary_item(artifact)?;
        lines.push(String::new());
        lines.push(format!("# {}", artifact.title));
        lines.extend(item.details);
    }
    Ok(lines)
}
