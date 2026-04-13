const DASHBOARD_DIFF_SCHEMA_HELP_TEXT: &str =
    include_str!("../../../../schemas/help/diff/dashboard.txt");
const ALERT_DIFF_SCHEMA_HELP_TEXT: &str = include_str!("../../../../schemas/help/diff/alert.txt");
const DATASOURCE_DIFF_SCHEMA_HELP_TEXT: &str =
    include_str!("../../../../schemas/help/diff/datasource.txt");

pub(crate) fn render_workspace_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/root.help.txt"
            ))
            .to_string(),
        ),
        Some("summary") | Some("scan") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/summary.help.txt"
            ))
            .to_string(),
        ),
        Some("plan") | Some("preview") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/plan.help.txt"
            ))
            .to_string(),
        ),
        Some("review") | Some("mark-reviewed") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/review.help.txt"
            ))
            .to_string(),
        ),
        Some("apply") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/apply.help.txt"
            ))
            .to_string(),
        ),
        Some("audit") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/audit.help.txt"
            ))
            .to_string(),
        ),
        Some("preflight") | Some("test") | Some("input-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("assess-alerts") | Some("alert-readiness") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/assess-alerts.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle-preflight") | Some("package-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("promotion-preflight") | Some("promote-test") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/promotion-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle") | Some("package") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

pub(crate) fn render_dashboard_history_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/root.help.txt"
            ))
            .to_string(),
        ),
        Some("list") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/list.help.txt"
            ))
            .to_string(),
        ),
        Some("restore") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/restore.help.txt"
            ))
            .to_string(),
        ),
        Some("diff") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/diff.help.txt"
            ))
            .to_string(),
        ),
        Some("export") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/export.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

pub(crate) fn render_diff_schema_help(domain: &str) -> Option<String> {
    match domain {
        "dashboard" => Some(DASHBOARD_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "alert" => Some(ALERT_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "datasource" => Some(DATASOURCE_DIFF_SCHEMA_HELP_TEXT.to_string()),
        _ => None,
    }
}

pub(crate) fn render_status_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(include_str!("../../../../schemas/help/status/root.txt").to_string()),
        Some("staged") => {
            Some(include_str!("../../../../schemas/help/status/staged.txt").to_string())
        }
        Some("live") => Some(include_str!("../../../../schemas/help/status/live.txt").to_string()),
        _ => None,
    }
}
