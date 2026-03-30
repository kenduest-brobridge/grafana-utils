//! Overview section and view projection helpers.

use crate::common::Result;

use super::{
    overview_kind::parse_overview_artifact_kind,
    overview_section_rows::{
        build_bundle_views, build_fact_breakdown_view, build_input_view, build_rich_section_view,
    },
    overview_summary_projection::section_summary_facts,
    OverviewArtifact, OverviewSection, OverviewSectionFact, OverviewSectionItem,
    OverviewSectionView, OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
};

fn summary_facts_to_line(facts: &[OverviewSectionFact]) -> String {
    format!("Summary: {}", summary_facts_to_meta(facts))
}

pub(crate) fn summary_facts_to_meta(facts: &[OverviewSectionFact]) -> String {
    facts
        .iter()
        .map(|fact| format!("{}={}", fact.label, fact.value))
        .collect::<Vec<String>>()
        .join(" ")
}

fn section_detail_lines(
    artifact: &OverviewArtifact,
    summary_facts: &[OverviewSectionFact],
) -> Vec<String> {
    let mut lines = Vec::new();
    if !artifact.inputs.is_empty() {
        lines.push(format!(
            "Inputs: {}",
            artifact
                .inputs
                .iter()
                .map(|item| format!("{}={}", item.name, item.value))
                .collect::<Vec<String>>()
                .join(" ")
        ));
    }
    lines.push(summary_facts_to_line(summary_facts));
    lines
}

pub(crate) fn build_overview_summary_item(
    artifact: &OverviewArtifact,
) -> Result<OverviewSectionItem> {
    let summary_facts = section_summary_facts(artifact)?;
    let meta = summary_facts_to_meta(&summary_facts);
    Ok(OverviewSectionItem {
        kind: parse_overview_artifact_kind(&artifact.kind)?
            .item_kind()
            .to_string(),
        title: artifact.title.clone(),
        meta,
        facts: summary_facts.clone(),
        details: section_detail_lines(artifact, &summary_facts),
    })
}

fn build_overview_section(
    artifact: &OverviewArtifact,
    artifact_index: usize,
) -> Result<OverviewSection> {
    let summary_item = build_overview_summary_item(artifact)?;
    let summary_facts = summary_item.facts.clone();
    let meta = summary_item.meta.clone();
    let mut views = vec![OverviewSectionView {
        label: "Summary".to_string(),
        items: vec![summary_item],
    }];
    if let Some(view) = build_fact_breakdown_view(artifact, &summary_facts) {
        views.push(view);
    }
    if artifact.kind == OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND {
        views.extend(build_bundle_views(artifact));
    }
    if let Some(view) = build_rich_section_view(artifact) {
        views.push(view);
    }
    if let Some(view) = build_input_view(artifact) {
        views.push(view);
    }
    Ok(OverviewSection {
        artifact_index,
        kind: artifact.kind.clone(),
        label: artifact.title.clone(),
        subtitle: meta,
        views,
    })
}

pub(crate) fn build_overview_sections(
    artifacts: &[OverviewArtifact],
) -> Result<Vec<OverviewSection>> {
    artifacts
        .iter()
        .enumerate()
        .map(|(index, artifact)| build_overview_section(artifact, index))
        .collect()
}
