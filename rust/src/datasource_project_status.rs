//! Shared datasource domain-status producer.
//!
//! Maintainer note:
//! - This module derives one datasource-owned domain-status row from the staged
//!   datasource export document.
//! - Keep it document-driven and conservative so the overview layer can reuse
//!   the same contract without duplicating staged-summary logic.

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

const DATASOURCE_DOMAIN_ID: &str = "datasource";
const DATASOURCE_SCOPE: &str = "staged";
const DATASOURCE_MODE: &str = "artifact-summary";
const DATASOURCE_REASON_READY: &str = PROJECT_STATUS_READY;
const DATASOURCE_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const DATASOURCE_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const DATASOURCE_SOURCE_KINDS: &[&str] = &["datasource-export"];
const DATASOURCE_SIGNAL_KEYS: &[&str] = &[
    "summary.datasourceCount",
    "summary.orgCount",
    "summary.defaultCount",
    "summary.typeCount",
    "summary.wouldCreate",
    "summary.wouldUpdate",
    "summary.wouldSkip",
    "summary.wouldBlock",
    "summary.wouldCreateOrgCount",
];

const DATASOURCE_WARNING_MISSING_DEFAULT: &str = "missing-default";
const DATASOURCE_WARNING_MULTIPLE_DEFAULTS: &str = "multiple-defaults";
const DATASOURCE_WARNING_DIFF_DRIFT_CHANGED_FIELDS: &str = "diff-drift-changed-fields";
const DATASOURCE_WARNING_DIFF_DRIFT_MISSING_LIVE: &str = "diff-drift-missing-live";
const DATASOURCE_WARNING_DIFF_DRIFT_MISSING_EXPORT: &str = "diff-drift-missing-export";
const DATASOURCE_WARNING_DIFF_DRIFT_AMBIGUOUS: &str = "diff-drift-ambiguous";
const DATASOURCE_WARNING_DIFF_DRIFT_SUMMARY: &str = "diff-drift-summary";
const DATASOURCE_WARNING_SECRET_REFERENCE_READY: &str = "secret-reference-ready";
const DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_CREATE: &str = "import-preview-would-create";
const DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_UPDATE: &str = "import-preview-would-update";
const DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_SKIP: &str = "import-preview-would-skip";
const DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_CREATE_ORG: &str = "import-preview-would-create-org";
const DATASOURCE_BLOCKER_IMPORT_PREVIEW_WOULD_BLOCK: &str = "import-preview-would-block";

const DATASOURCE_EXPORT_AT_LEAST_ONE_ACTIONS: &[&str] = &["export at least one datasource"];
const DATASOURCE_MARK_DEFAULT_ACTIONS: &[&str] = &["mark a default datasource if none is set"];
const DATASOURCE_KEEP_SINGLE_DEFAULT_ACTIONS: &[&str] =
    &["keep exactly one datasource marked as the default"];
const DATASOURCE_REVIEW_DIFF_DRIFT_ACTIONS: &[&str] =
    &["review datasource diff drift before import or sync"];
const DATASOURCE_REVIEW_SECRET_REFERENCE_ACTIONS: &[&str] =
    &["review datasource secret references before import or sync"];
const DATASOURCE_REVIEW_IMPORT_PREVIEW_ACTIONS: &[&str] =
    &["review datasource import preview before import or sync"];
const DATASOURCE_REVIEW_IMPORT_ORG_CREATION_ACTIONS: &[&str] =
    &["review datasource org creation before import or sync"];
const DATASOURCE_RESOLVE_IMPORT_BLOCKERS_ACTIONS: &[&str] =
    &["resolve datasource import preview blockers before import or sync"];

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn summary_number_first(document: &Value, keys: &[&'static str]) -> (usize, Option<&'static str>) {
    for key in keys {
        let count = summary_number(document, key);
        if count > 0 {
            return (count, Some(*key));
        }
    }
    (0, None)
}

fn append_signal_key(signal_keys: &mut Vec<String>, source: &str) {
    if !signal_keys.iter().any(|item| item == source) {
        signal_keys.push(source.to_string());
    }
}

fn summary_source_label(source: Option<&str>, fallback: &str) -> String {
    format!("summary.{}", source.unwrap_or(fallback))
}

fn push_warning(
    warnings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    signal_keys: &mut Vec<String>,
    kind: &str,
    count: usize,
    source: &str,
) {
    if count == 0 {
        return;
    }
    warnings.push(status_finding(kind, count, source));
    append_signal_key(signal_keys, source);
}

pub(crate) fn build_datasource_domain_status(
    summary_document: Option<&Value>,
) -> Option<ProjectDomainStatus> {
    let document = summary_document?;
    let datasources = summary_number(document, "datasourceCount");
    let _orgs = summary_number(document, "orgCount");
    let defaults = summary_number(document, "defaultCount");
    let _types = summary_number(document, "typeCount");

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut signal_keys = DATASOURCE_SIGNAL_KEYS
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<String>>();

    if datasources == 0 {
        return Some(ProjectDomainStatus {
            id: DATASOURCE_DOMAIN_ID.to_string(),
            scope: DATASOURCE_SCOPE.to_string(),
            mode: DATASOURCE_MODE.to_string(),
            status: PROJECT_STATUS_PARTIAL.to_string(),
            reason_code: DATASOURCE_REASON_PARTIAL_NO_DATA.to_string(),
            primary_count: datasources,
            blocker_count: 0,
            warning_count: 0,
            source_kinds: DATASOURCE_SOURCE_KINDS
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
            signal_keys,
            blockers: Vec::new(),
            warnings,
            next_actions: DATASOURCE_EXPORT_AT_LEAST_ONE_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
            freshness: Default::default(),
        });
    }

    let mut next_actions = if defaults == 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_MISSING_DEFAULT,
            1,
            "summary.defaultCount",
        );
        DATASOURCE_MARK_DEFAULT_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else if defaults > 1 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_MULTIPLE_DEFAULTS,
            defaults - 1,
            "summary.defaultCount",
        );
        DATASOURCE_KEEP_SINGLE_DEFAULT_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else {
        Vec::new()
    };

    let (changed_fields, changed_fields_source) =
        summary_number_first(document, &["differentCount", "different_count"]);
    let (missing_live, missing_live_source) =
        summary_number_first(document, &["missingLiveCount", "missing_in_live_count"]);
    let (missing_export, missing_export_source) = summary_number_first(
        document,
        &[
            "extraLiveCount",
            "missingInExportCount",
            "missing_in_export_count",
        ],
    );
    let (ambiguous, ambiguous_source) = summary_number_first(
        document,
        &[
            "ambiguousCount",
            "ambiguousLiveMatchCount",
            "ambiguous_live_match_count",
        ],
    );
    let (summary_diff_drift, summary_diff_drift_source) =
        summary_number_first(document, &["diffCount", "diff_count"]);
    let (secret_reference_ready, secret_reference_source) =
        summary_number_first(document, &["secretVisibilityCount", "secretReferenceCount"]);
    let (would_create, would_create_source) =
        summary_number_first(document, &["wouldCreate", "would_create"]);
    let (would_update, would_update_source) =
        summary_number_first(document, &["wouldUpdate", "would_update"]);
    let (would_block, would_block_source) =
        summary_number_first(document, &["wouldBlock", "would_block"]);
    let (would_skip, would_skip_source) =
        summary_number_first(document, &["wouldSkip", "would_skip"]);
    let (would_create_org, would_create_org_source) =
        summary_number_first(document, &["wouldCreateOrgCount", "would_create_org_count"]);

    let mut diff_drift_review_required = false;
    let mut granular_diff_drift_found = false;
    if changed_fields > 0 {
        diff_drift_review_required = true;
        granular_diff_drift_found = true;
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_DIFF_DRIFT_CHANGED_FIELDS,
            changed_fields,
            &summary_source_label(changed_fields_source, "differentCount"),
        );
    }
    if missing_live > 0 {
        diff_drift_review_required = true;
        granular_diff_drift_found = true;
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_DIFF_DRIFT_MISSING_LIVE,
            missing_live,
            &summary_source_label(missing_live_source, "missingLiveCount"),
        );
    }
    if missing_export > 0 {
        diff_drift_review_required = true;
        granular_diff_drift_found = true;
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_DIFF_DRIFT_MISSING_EXPORT,
            missing_export,
            &summary_source_label(missing_export_source, "missingInExportCount"),
        );
    }
    if ambiguous > 0 {
        diff_drift_review_required = true;
        granular_diff_drift_found = true;
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_DIFF_DRIFT_AMBIGUOUS,
            ambiguous,
            &summary_source_label(ambiguous_source, "ambiguousCount"),
        );
    }
    if !granular_diff_drift_found && summary_diff_drift > 0 {
        diff_drift_review_required = true;
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_DIFF_DRIFT_SUMMARY,
            summary_diff_drift,
            &summary_source_label(summary_diff_drift_source, "diffCount"),
        );
    }
    if diff_drift_review_required {
        next_actions.extend(
            DATASOURCE_REVIEW_DIFF_DRIFT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    if secret_reference_ready > 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_SECRET_REFERENCE_READY,
            secret_reference_ready,
            &summary_source_label(secret_reference_source, "secretVisibilityCount"),
        );
        next_actions.extend(
            DATASOURCE_REVIEW_SECRET_REFERENCE_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    if would_create > 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_CREATE,
            would_create,
            &summary_source_label(would_create_source, "wouldCreate"),
        );
    }
    if would_update > 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_UPDATE,
            would_update,
            &summary_source_label(would_update_source, "wouldUpdate"),
        );
    }
    if would_block > 0 {
        let source = summary_source_label(would_block_source, "wouldBlock");
        blockers.push(status_finding(
            DATASOURCE_BLOCKER_IMPORT_PREVIEW_WOULD_BLOCK,
            would_block,
            &source,
        ));
        append_signal_key(&mut signal_keys, &source);
    }
    if would_skip > 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_SKIP,
            would_skip,
            &summary_source_label(would_skip_source, "wouldSkip"),
        );
    }
    if would_create_org > 0 {
        push_warning(
            &mut warnings,
            &mut signal_keys,
            DATASOURCE_WARNING_IMPORT_PREVIEW_WOULD_CREATE_ORG,
            would_create_org,
            &summary_source_label(would_create_org_source, "wouldCreateOrgCount"),
        );
        next_actions.extend(
            DATASOURCE_REVIEW_IMPORT_ORG_CREATION_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    if would_create > 0 || would_update > 0 || would_block > 0 || would_skip > 0 {
        next_actions.extend(
            DATASOURCE_REVIEW_IMPORT_PREVIEW_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    if !blockers.is_empty() {
        next_actions.splice(
            0..0,
            DATASOURCE_RESOLVE_IMPORT_BLOCKERS_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    let (status, reason_code) = if !blockers.is_empty() {
        (
            PROJECT_STATUS_BLOCKED,
            DATASOURCE_REASON_BLOCKED_BY_BLOCKERS,
        )
    } else {
        (PROJECT_STATUS_READY, DATASOURCE_REASON_READY)
    };

    Some(ProjectDomainStatus {
        id: DATASOURCE_DOMAIN_ID.to_string(),
        scope: DATASOURCE_SCOPE.to_string(),
        mode: DATASOURCE_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: datasources,
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds: DATASOURCE_SOURCE_KINDS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        signal_keys,
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::build_datasource_domain_status;
    use serde_json::json;

    #[test]
    fn build_datasource_domain_status_tracks_staged_summary_fields() {
        let document = json!({
            "summary": {
                "datasourceCount": 2,
                "orgCount": 2,
                "defaultCount": 0,
                "typeCount": 2,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["id"], json!("datasource"));
        assert_eq!(domain["scope"], json!("staged"));
        assert_eq!(domain["mode"], json!("artifact-summary"));
        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["reasonCode"], json!("ready"));
        assert_eq!(domain["primaryCount"], json!(2));
        assert_eq!(domain["blockerCount"], json!(0));
        assert_eq!(domain["warningCount"], json!(1));
        assert_eq!(domain["sourceKinds"], json!(["datasource-export"]));
        assert_eq!(
            domain["signalKeys"],
            json!([
                "summary.datasourceCount",
                "summary.orgCount",
                "summary.defaultCount",
                "summary.typeCount",
                "summary.wouldCreate",
                "summary.wouldUpdate",
                "summary.wouldSkip",
                "summary.wouldBlock",
                "summary.wouldCreateOrgCount",
            ])
        );
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "missing-default",
                    "count": 1,
                    "source": "summary.defaultCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["mark a default datasource if none is set"])
        );
    }

    #[test]
    fn build_datasource_domain_status_surfaces_diff_drift_and_secret_readiness() {
        let document = json!({
            "summary": {
                "datasourceCount": 3,
                "orgCount": 2,
                "defaultCount": 1,
                "typeCount": 2,
                "differentCount": 2,
                "missingLiveCount": 1,
                "extraLiveCount": 1,
                "ambiguousCount": 1,
                "secretVisibilityCount": 4,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["reasonCode"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(9));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "diff-drift-changed-fields",
                    "count": 2,
                    "source": "summary.differentCount",
                },
                {
                    "kind": "diff-drift-missing-live",
                    "count": 1,
                    "source": "summary.missingLiveCount",
                },
                {
                    "kind": "diff-drift-missing-export",
                    "count": 1,
                    "source": "summary.extraLiveCount",
                },
                {
                    "kind": "diff-drift-ambiguous",
                    "count": 1,
                    "source": "summary.ambiguousCount",
                },
                {
                    "kind": "secret-reference-ready",
                    "count": 4,
                    "source": "summary.secretVisibilityCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "review datasource diff drift before import or sync",
                "review datasource secret references before import or sync",
            ])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "summary.datasourceCount",
                "summary.orgCount",
                "summary.defaultCount",
                "summary.typeCount",
                "summary.wouldCreate",
                "summary.wouldUpdate",
                "summary.wouldSkip",
                "summary.wouldBlock",
                "summary.wouldCreateOrgCount",
                "summary.differentCount",
                "summary.missingLiveCount",
                "summary.extraLiveCount",
                "summary.ambiguousCount",
                "summary.secretVisibilityCount",
            ])
        );
    }

    #[test]
    fn build_datasource_domain_status_surfaces_import_preview_mutation_counts() {
        let document = json!({
            "summary": {
                "datasourceCount": 4,
                "orgCount": 2,
                "defaultCount": 1,
                "typeCount": 3,
                "would_create": 2,
                "would_update": 1,
                "would_skip": 1,
                "would_block": 3,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("blocked"));
        assert_eq!(domain["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(domain["blockerCount"], json!(3));
        assert_eq!(domain["warningCount"], json!(4));
        assert_eq!(
            domain["blockers"],
            json!([
                {
                    "kind": "import-preview-would-block",
                    "count": 3,
                    "source": "summary.would_block",
                }
            ])
        );
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "import-preview-would-create",
                    "count": 2,
                    "source": "summary.would_create",
                },
                {
                    "kind": "import-preview-would-update",
                    "count": 1,
                    "source": "summary.would_update",
                },
                {
                    "kind": "import-preview-would-skip",
                    "count": 1,
                    "source": "summary.would_skip",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "resolve datasource import preview blockers before import or sync",
                "review datasource import preview before import or sync"
            ])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "summary.datasourceCount",
                "summary.orgCount",
                "summary.defaultCount",
                "summary.typeCount",
                "summary.wouldCreate",
                "summary.wouldUpdate",
                "summary.wouldSkip",
                "summary.wouldBlock",
                "summary.wouldCreateOrgCount",
                "summary.would_create",
                "summary.would_update",
                "summary.would_block",
                "summary.would_skip",
            ])
        );
    }

    #[test]
    fn build_datasource_domain_status_surfaces_import_preview_skip_only() {
        let document = json!({
            "summary": {
                "datasourceCount": 2,
                "orgCount": 1,
                "defaultCount": 1,
                "typeCount": 1,
                "would_skip": 2,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["reasonCode"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(2));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "import-preview-would-skip",
                    "count": 2,
                    "source": "summary.would_skip",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["review datasource import preview before import or sync",])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "summary.datasourceCount",
                "summary.orgCount",
                "summary.defaultCount",
                "summary.typeCount",
                "summary.wouldCreate",
                "summary.wouldUpdate",
                "summary.wouldSkip",
                "summary.wouldBlock",
                "summary.wouldCreateOrgCount",
                "summary.would_skip",
            ])
        );
    }

    #[test]
    fn build_datasource_domain_status_surfaces_import_org_creation_readiness() {
        let document = json!({
            "summary": {
                "datasourceCount": 4,
                "orgCount": 3,
                "defaultCount": 1,
                "typeCount": 2,
                "wouldCreateOrgCount": 2,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("ready"));
        assert_eq!(domain["reasonCode"], json!("ready"));
        assert_eq!(domain["warningCount"], json!(2));
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "import-preview-would-create-org",
                    "count": 2,
                    "source": "summary.wouldCreateOrgCount",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["review datasource org creation before import or sync"])
        );
        assert_eq!(
            domain["signalKeys"],
            json!([
                "summary.datasourceCount",
                "summary.orgCount",
                "summary.defaultCount",
                "summary.typeCount",
                "summary.wouldCreate",
                "summary.wouldUpdate",
                "summary.wouldSkip",
                "summary.wouldBlock",
                "summary.wouldCreateOrgCount",
            ])
        );
    }

    #[test]
    fn build_datasource_domain_status_falls_back_to_snake_case_import_org_creation_count() {
        let document = json!({
            "summary": {
                "datasourceCount": 4,
                "orgCount": 3,
                "defaultCount": 1,
                "typeCount": 2,
                "would_create_org_count": 1,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "import-preview-would-create-org",
                    "count": 1,
                    "source": "summary.would_create_org_count",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!(["review datasource org creation before import or sync"])
        );
    }

    #[test]
    fn build_datasource_domain_status_keeps_diff_and_secret_signals_with_import_preview() {
        let document = json!({
            "summary": {
                "datasourceCount": 5,
                "orgCount": 2,
                "defaultCount": 1,
                "typeCount": 3,
                "differentCount": 2,
                "secretVisibilityCount": 4,
                "wouldCreate": 1,
                "wouldSkip": 1,
                "wouldBlock": 2,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("blocked"));
        assert_eq!(domain["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(domain["blockerCount"], json!(2));
        assert_eq!(domain["warningCount"], json!(8));
        assert_eq!(
            domain["blockers"],
            json!([
                {
                    "kind": "import-preview-would-block",
                    "count": 2,
                    "source": "summary.wouldBlock",
                }
            ])
        );
        assert_eq!(
            domain["warnings"],
            json!([
                {
                    "kind": "diff-drift-changed-fields",
                    "count": 2,
                    "source": "summary.differentCount",
                },
                {
                    "kind": "secret-reference-ready",
                    "count": 4,
                    "source": "summary.secretVisibilityCount",
                },
                {
                    "kind": "import-preview-would-create",
                    "count": 1,
                    "source": "summary.wouldCreate",
                },
                {
                    "kind": "import-preview-would-skip",
                    "count": 1,
                    "source": "summary.wouldSkip",
                }
            ])
        );
        assert_eq!(
            domain["nextActions"],
            json!([
                "resolve datasource import preview blockers before import or sync",
                "review datasource diff drift before import or sync",
                "review datasource secret references before import or sync",
                "review datasource import preview before import or sync",
            ])
        );
    }

    #[test]
    fn build_datasource_domain_status_is_partial_without_datasources() {
        let document = json!({
            "summary": {
                "datasourceCount": 0,
                "orgCount": 0,
                "defaultCount": 0,
                "typeCount": 0,
            }
        });

        let domain = build_datasource_domain_status(Some(&document)).unwrap();
        let domain = serde_json::to_value(domain).unwrap();

        assert_eq!(domain["status"], json!("partial"));
        assert_eq!(domain["reasonCode"], json!("partial-no-data"));
        assert_eq!(
            domain["nextActions"],
            json!(["export at least one datasource"])
        );
    }
}
