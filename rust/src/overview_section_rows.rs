//! Overview section row builders for artifact-specific views.

use serde_json::{Map, Value};

use super::overview_kind::{parse_overview_artifact_kind, OverviewArtifactKind};
use super::overview_support::value_is_truthy;
use super::{OverviewArtifact, OverviewSectionFact, OverviewSectionItem, OverviewSectionView};

fn value_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(", "),
        _ => String::new(),
    }
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map(Vec::len).unwrap_or(0)
}

fn object_array<'a>(document: &'a Value, key: &str) -> Vec<&'a Map<String, Value>> {
    document
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .collect()
}

fn meta_value(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}

fn detail_line(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label}: {value}")
}

fn overview_item_kind(artifact_kind: &str) -> Option<&'static str> {
    parse_overview_artifact_kind(artifact_kind)
        .ok()
        .map(OverviewArtifactKind::item_kind)
}

fn fact_breakdown_view_label(artifact_kind: &str) -> Option<&'static str> {
    parse_overview_artifact_kind(artifact_kind)
        .ok()
        .map(OverviewArtifactKind::fact_breakdown_label)
}

fn access_view_label(kind: OverviewArtifactKind) -> &'static str {
    match kind {
        OverviewArtifactKind::AccessUserExport => "Users",
        OverviewArtifactKind::AccessTeamExport => "Teams",
        OverviewArtifactKind::AccessOrgExport => "Orgs",
        OverviewArtifactKind::AccessServiceAccountExport => "Service Accounts",
        _ => "Records",
    }
}

pub(super) fn build_input_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    if artifact.inputs.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: "Inputs".to_string(),
        items: artifact
            .inputs
            .iter()
            .map(|input| OverviewSectionItem {
                kind: "input".to_string(),
                title: input.name.clone(),
                meta: input.value.clone(),
                facts: Vec::new(),
                details: vec![
                    detail_line("Input", &input.name),
                    detail_line("Value", &input.value),
                ],
            })
            .collect(),
    })
}

fn build_datasource_inventory_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    let rows = object_array(&artifact.document, "datasources");
    if rows.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: "Datasource Inventory".to_string(),
        items: rows
            .into_iter()
            .map(|row| {
                let name = value_string(row.get("name"));
                let uid = value_string(row.get("uid"));
                let kind = value_string(row.get("type"));
                let org = value_string(row.get("org"));
                let org_id = value_string(row.get("orgId"));
                let access = value_string(row.get("access"));
                let url = value_string(row.get("url"));
                let is_default = value_is_truthy(row.get("isDefault"));
                OverviewSectionItem {
                    kind: "datasource".to_string(),
                    title: if name.is_empty() {
                        uid.clone()
                    } else {
                        name.clone()
                    },
                    meta: format!(
                        "type={} org={} default={}",
                        meta_value(&kind),
                        meta_value(&org),
                        is_default
                    ),
                    facts: vec![
                        OverviewSectionFact {
                            label: "uid".to_string(),
                            value: uid.clone(),
                        },
                        OverviewSectionFact {
                            label: "type".to_string(),
                            value: kind.clone(),
                        },
                        OverviewSectionFact {
                            label: "org".to_string(),
                            value: org.clone(),
                        },
                        OverviewSectionFact {
                            label: "org-id".to_string(),
                            value: org_id,
                        },
                    ],
                    details: vec![
                        detail_line("Datasource", meta_value(&name)),
                        detail_line("UID", &uid),
                        detail_line("Type", &kind),
                        detail_line("Org", &org),
                        detail_line("Access", meta_value(&access)),
                        detail_line("Default", is_default),
                        detail_line("URL", meta_value(&url)),
                    ],
                }
            })
            .collect(),
    })
}

fn build_alert_assets_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    let sources = [
        ("rules", "alert-rule"),
        ("contactPoints", "contact-point"),
        ("muteTimings", "mute-timing"),
        ("policies", "notification-policy"),
        ("templates", "template"),
    ];
    let mut items = Vec::new();
    for (key, kind) in sources {
        for row in object_array(&artifact.document, key) {
            let title = value_string(row.get("title"));
            let name = value_string(row.get("name"));
            let uid = value_string(row.get("uid"));
            let receiver = value_string(row.get("receiver"));
            let path = value_string(row.get("path"));
            let display = if !title.is_empty() {
                title.clone()
            } else if !name.is_empty() {
                name.clone()
            } else if !receiver.is_empty() {
                receiver.clone()
            } else if !uid.is_empty() {
                uid.clone()
            } else {
                kind.to_string()
            };
            items.push(OverviewSectionItem {
                kind: kind.to_string(),
                title: display,
                meta: format!("uid={} path={}", meta_value(&uid), meta_value(&path)),
                facts: vec![
                    OverviewSectionFact {
                        label: "uid".to_string(),
                        value: uid,
                    },
                    OverviewSectionFact {
                        label: "path".to_string(),
                        value: path.clone(),
                    },
                ],
                details: vec![
                    detail_line("Kind", &kind),
                    detail_line("Title", &title),
                    detail_line("Name", &name),
                    detail_line("Receiver", &receiver),
                    detail_line("Path", &path),
                ],
            });
        }
    }
    if items.is_empty() {
        None
    } else {
        Some(OverviewSectionView {
            label: "Asset Inventory".to_string(),
            items,
        })
    }
}

fn build_access_roster_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    let rows = object_array(&artifact.document, "records");
    if rows.is_empty() {
        return None;
    }
    let access_kind = parse_overview_artifact_kind(&artifact.kind).ok()?;
    let label = access_view_label(access_kind);
    let item_kind = overview_item_kind(&artifact.kind)
        .unwrap_or("overview")
        .to_string();
    Some(OverviewSectionView {
        label: label.to_string(),
        items: rows
            .into_iter()
            .map(|row| {
                let login = value_string(row.get("login"));
                let name = value_string(row.get("name"));
                let email = value_string(row.get("email"));
                let teams = value_string(row.get("teams"));
                let members = value_string(row.get("members"));
                let admins = value_string(row.get("admins"));
                let id = value_string(row.get("id"));
                let users = array_len(row.get("users"));
                let role = value_string(row.get("role"));
                let disabled = value_string(row.get("disabled"));
                let title = match access_kind {
                    OverviewArtifactKind::AccessUserExport => login.clone(),
                    OverviewArtifactKind::AccessTeamExport => name.clone(),
                    OverviewArtifactKind::AccessOrgExport => name.clone(),
                    OverviewArtifactKind::AccessServiceAccountExport => name.clone(),
                    _ => String::new(),
                };
                let meta = match access_kind {
                    OverviewArtifactKind::AccessUserExport => {
                        format!("email={} teams={}", &email, teams)
                    }
                    OverviewArtifactKind::AccessTeamExport => {
                        format!("email={} members={} admins={}", &email, members, admins)
                    }
                    OverviewArtifactKind::AccessOrgExport => {
                        format!("id={} users={}", &id, users)
                    }
                    OverviewArtifactKind::AccessServiceAccountExport => {
                        format!("role={} disabled={}", &role, &disabled)
                    }
                    _ => String::new(),
                };
                let details = match access_kind {
                    OverviewArtifactKind::AccessUserExport => vec![
                        detail_line("Login", &login),
                        detail_line("Name", &name),
                        detail_line("Email", &email),
                        detail_line("Teams", &teams),
                    ],
                    OverviewArtifactKind::AccessTeamExport => vec![
                        detail_line("Team", &name),
                        detail_line("Email", &email),
                        detail_line("Members", &members),
                        detail_line("Admins", &admins),
                    ],
                    OverviewArtifactKind::AccessOrgExport => vec![
                        detail_line("Org", &name),
                        detail_line("ID", &id),
                        detail_line("User count", users),
                    ],
                    OverviewArtifactKind::AccessServiceAccountExport => vec![
                        detail_line("Service account", &name),
                        detail_line("Role", &role),
                        detail_line("Disabled", &disabled),
                    ],
                    _ => vec!["Record".to_string()],
                };
                OverviewSectionItem {
                    kind: item_kind.clone(),
                    title,
                    meta,
                    facts: Vec::new(),
                    details,
                }
            })
            .collect(),
    })
}

fn build_sync_resources_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    let rows = object_array(&artifact.document, "resources");
    if rows.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: "Resources".to_string(),
        items: rows
            .into_iter()
            .map(|row| {
                let kind = value_string(row.get("kind"));
                let title = value_string(row.get("title"));
                let identity = value_string(row.get("identity"));
                let managed_fields = array_len(row.get("managedFields"));
                let body_field_count = value_string(row.get("bodyFieldCount"));
                let source_path = value_string(row.get("sourcePath"));
                OverviewSectionItem {
                    kind: if kind.is_empty() {
                        "sync".to_string()
                    } else {
                        kind.clone()
                    },
                    title: if title.is_empty() {
                        identity.clone()
                    } else {
                        title.clone()
                    },
                    meta: format!(
                        "kind={} fields={} source={}",
                        meta_value(&kind),
                        meta_value(&body_field_count),
                        meta_value(&source_path)
                    ),
                    facts: vec![OverviewSectionFact {
                        label: "managed-fields".to_string(),
                        value: managed_fields.to_string(),
                    }],
                    details: vec![
                        detail_line("Kind", &kind),
                        detail_line("Identity", &identity),
                        detail_line("Title", &title),
                        detail_line("Managed fields", managed_fields),
                        detail_line("Body field count", &body_field_count),
                        detail_line("Source path", &source_path),
                    ],
                }
            })
            .collect(),
    })
}

fn build_promotion_checks_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    let rows = object_array(&artifact.document, "checks");
    if rows.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: "Checks".to_string(),
        items: rows
            .into_iter()
            .map(|row| {
                let kind = value_string(row.get("kind"));
                let identity = value_string(row.get("identity"));
                let status = value_string(row.get("status"));
                let resolution = value_string(row.get("resolution"));
                let detail = value_string(row.get("detail"));
                let blocking = value_string(row.get("blocking"));
                OverviewSectionItem {
                    kind: "promotion-check".to_string(),
                    title: if identity.is_empty() {
                        kind.clone()
                    } else {
                        identity.clone()
                    },
                    meta: format!(
                        "kind={} status={} blocking={}",
                        meta_value(&kind),
                        meta_value(&status),
                        meta_value(&blocking)
                    ),
                    facts: vec![OverviewSectionFact {
                        label: "resolution".to_string(),
                        value: resolution.clone(),
                    }],
                    details: vec![
                        detail_line("Kind", &kind),
                        detail_line("Identity", &identity),
                        detail_line("Status", &status),
                        detail_line("Resolution", &resolution),
                        detail_line("Detail", &detail),
                        detail_line("Blocking", &blocking),
                    ],
                }
            })
            .collect(),
    })
}

fn build_bundle_assessment_checks_view(
    artifact: &OverviewArtifact,
    key: &str,
    label: &str,
) -> Option<OverviewSectionView> {
    let checks = artifact
        .document
        .get(key)
        .and_then(Value::as_object)
        .and_then(|assessment| assessment.get("checks"))
        .and_then(Value::as_array)?;
    if checks.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: label.to_string(),
        items: checks
            .iter()
            .filter_map(Value::as_object)
            .map(|row| {
                let kind = value_string(row.get("kind"));
                let identity = value_string(row.get("identity"));
                let title = value_string(row.get("title"));
                let datasource_name = value_string(row.get("datasourceName"));
                let provider_name = value_string(row.get("providerName"));
                let placeholder_name = value_string(row.get("placeholderName"));
                let source_path = value_string(row.get("sourcePath"));
                let status = value_string(row.get("status"));
                let blocking = value_string(row.get("blocking"));
                let detail = value_string(row.get("detail"));
                let display = if !title.is_empty() {
                    title.clone()
                } else if !identity.is_empty() {
                    identity.clone()
                } else if !datasource_name.is_empty() {
                    datasource_name.clone()
                } else {
                    kind.clone()
                };
                OverviewSectionItem {
                    kind: "bundle-check".to_string(),
                    title: display,
                    meta: format!(
                        "kind={} status={} blocking={}",
                        meta_value(&kind),
                        meta_value(&status),
                        meta_value(&blocking)
                    ),
                    facts: vec![
                        OverviewSectionFact {
                            label: "datasource".to_string(),
                            value: datasource_name.clone(),
                        },
                        OverviewSectionFact {
                            label: "source-path".to_string(),
                            value: source_path.clone(),
                        },
                    ],
                    details: vec![
                        detail_line("Kind", &kind),
                        detail_line("Identity", &identity),
                        detail_line("Title", &title),
                        detail_line("Datasource", &datasource_name),
                        detail_line("Provider", &provider_name),
                        detail_line("Placeholder", &placeholder_name),
                        detail_line("Source path", &source_path),
                        detail_line("Status", &status),
                        detail_line("Blocking", &blocking),
                        detail_line("Detail", &detail),
                    ],
                }
            })
            .collect(),
    })
}

pub(super) fn build_bundle_views(artifact: &OverviewArtifact) -> Vec<OverviewSectionView> {
    let mut views = Vec::new();
    if let Some(view) =
        build_bundle_assessment_checks_view(artifact, "syncPreflight", "Sync Checks")
    {
        views.push(view);
    }
    if let Some(view) =
        build_bundle_assessment_checks_view(artifact, "providerAssessment", "Secret Providers")
    {
        views.push(view);
    }
    if let Some(view) = build_bundle_assessment_checks_view(
        artifact,
        "secretPlaceholderAssessment",
        "Secret Placeholders",
    ) {
        views.push(view);
    }
    if let Some(view) = build_bundle_assessment_checks_view(
        artifact,
        "alertArtifactAssessment",
        "Alerting Artifacts",
    ) {
        views.push(view);
    }
    views
}

pub(super) fn build_rich_section_view(artifact: &OverviewArtifact) -> Option<OverviewSectionView> {
    match parse_overview_artifact_kind(&artifact.kind).ok()? {
        super::overview_kind::OverviewArtifactKind::DatasourceExport => {
            build_datasource_inventory_view(artifact)
        }
        super::overview_kind::OverviewArtifactKind::AlertExport => {
            build_alert_assets_view(artifact)
        }
        super::overview_kind::OverviewArtifactKind::AccessUserExport
        | super::overview_kind::OverviewArtifactKind::AccessTeamExport
        | super::overview_kind::OverviewArtifactKind::AccessOrgExport
        | super::overview_kind::OverviewArtifactKind::AccessServiceAccountExport => {
            build_access_roster_view(artifact)
        }
        super::overview_kind::OverviewArtifactKind::SyncSummary => {
            build_sync_resources_view(artifact)
        }
        super::overview_kind::OverviewArtifactKind::PromotionPreflight => {
            build_promotion_checks_view(artifact)
        }
        _ => None,
    }
}

pub(super) fn build_fact_breakdown_view(
    artifact: &OverviewArtifact,
    summary_facts: &[OverviewSectionFact],
) -> Option<OverviewSectionView> {
    if summary_facts.is_empty() {
        return None;
    }
    Some(OverviewSectionView {
        label: fact_breakdown_view_label(&artifact.kind)
            .unwrap_or("Facts")
            .to_string(),
        items: summary_facts
            .iter()
            .map(|fact| OverviewSectionItem {
                kind: overview_item_kind(&artifact.kind)
                    .unwrap_or("overview")
                    .to_string(),
                title: fact.label.clone(),
                meta: fact.value.clone(),
                facts: vec![fact.clone()],
                details: vec![
                    detail_line("Section", &artifact.title),
                    detail_line("Metric", &fact.label),
                    detail_line("Value", &fact.value),
                ],
            })
            .collect(),
    })
}
