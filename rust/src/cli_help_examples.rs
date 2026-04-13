//! Structured help/example text for the unified Rust CLI.

use crate::help_styles::{paint_argument, paint_command, paint_support};
pub(crate) use crate::help_styles::{paint_section, HELP_PALETTE};

macro_rules! help_item {
    (($label:literal, $summary:literal, $command:literal)) => {
        concat!("  ", $label, " ", $summary, "\n    ", $command)
    };
}

macro_rules! help_block {
    ($heading:literal, $first:tt $(, $rest:tt)+ $(,)?) => {
        concat!(
            $heading,
            "\n\n",
            help_item!($first),
            $(
                "\n\n",
                help_item!($rest),
            )+
        )
    };
    ($heading:literal, $only:tt $(,)?) => {
        concat!($heading, "\n\n", help_item!($only))
    };
}

fn is_cli_command_text(trimmed: &str) -> bool {
    trimmed == "grafana-util"
        || trimmed
            .strip_prefix("grafana-util")
            .is_some_and(|rest| rest.chars().next().is_some_and(char::is_whitespace))
}

fn paint_cli_command_line(indent: &str, trimmed: &str) -> Option<String> {
    if is_cli_command_text(trimmed) {
        Some(format!("{indent}{}", paint_command(trimmed)))
    } else {
        None
    }
}

fn paint_example_caption(text: &str) -> String {
    paint_argument(text)
}

fn is_option_entry_line(trimmed: &str) -> bool {
    let visible;
    let trimmed = if trimmed.contains('\x1b') {
        visible = crate::common::strip_ansi_codes(trimmed);
        visible.as_str()
    } else {
        trimmed
    };
    trimmed.starts_with("--")
        || trimmed
            .strip_prefix('-')
            .and_then(|rest| rest.chars().next())
            .is_some_and(|ch| ch.is_ascii_alphanumeric())
}

fn highlight_option_tokens(text: &str, resume_color: &str) -> String {
    let mut output = String::new();
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        if let Some(token_rest) = rest.strip_prefix("--") {
            if token_rest
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric())
            {
                let token_len = 2 + token_rest
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
                    .map(char::len_utf8)
                    .sum::<usize>();
                output.push_str(HELP_PALETTE.reset);
                output.push_str(HELP_PALETTE.argument);
                output.push_str(&text[index..index + token_len]);
                output.push_str(HELP_PALETTE.reset);
                output.push_str(resume_color);
                index += token_len;
                continue;
            }
        }
        let ch = rest
            .chars()
            .next()
            .expect("index should point to a valid character");
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}

fn paint_option_entry(text: &str) -> String {
    if text.contains('\x1b') {
        paint_argument(&crate::common::strip_ansi_codes(text))
    } else {
        paint_argument(text)
    }
}

fn paint_support_with_args(text: &str) -> String {
    if !text.contains("--") {
        return paint_support(text);
    }
    format!(
        "{}{}{}",
        HELP_PALETTE.support,
        highlight_option_tokens(text, HELP_PALETTE.support),
        HELP_PALETTE.reset
    )
}

pub(crate) const UNIFIED_HELP_TEXT: &str = concat!(
    help_block!(
        "Suggested flow:",
        (
            "[Version]",
            "Confirm the binary is installed and on PATH:",
            "grafana-util --version"
        ),
        (
            "[Status]",
            "Run one read-only live check before any mutation workflow:",
            "grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml"
        ),
        (
            "[Profile]",
            "Save the same connection for repeatable day-to-day commands:",
            "grafana-util config profile add dev --url http://localhost:3000 --basic-user admin --prompt-password"
        ),
    ),
    "\n\n",
    help_block!(
    "Next common commands:",
    (
        "[Overview]",
        "Open the human-oriented live overview after connectivity works:",
        "grafana-util status overview live --profile dev --output-format interactive"
    ),
    (
        "[Export]",
        "Back up dashboards through the task-first export surface:",
        "grafana-util export dashboard --profile dev --output-dir ./dashboards --overwrite"
    ),
    (
        "[Workspace]",
        "Review a local workspace before touching Grafana:",
        "grafana-util workspace preview ./grafana-oac-repo --fetch-live --profile dev"
    ),
    (
        "[Dashboard]",
        "Inspect local dashboard exports through the flat dashboard surface:",
        "grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table"
    ),
    (
        "[Alert]",
        "Export alerting resources through the task-first export surface:",
        "grafana-util export alert --profile dev --output-dir ./alerts --overwrite"
    ),
    (
        "[Datasource]",
        "List datasource inventory from a local bundle:",
        "grafana-util datasource list --input-dir ./datasources --table"
    ),
    (
        "[Access]",
        "Inspect exported access users without calling Grafana:",
        "grafana-util access user list --input-dir ./access-users --json"
    )
    )
);

pub(crate) const UNIFIED_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Status]",
        "Query generic Grafana resources through the canonical status surface:",
        "grafana-util status resource describe dashboards --output-format json"
    ),
    (
        "[Export]",
        "Export datasource inventory from a repo workspace or local bundle:",
        "grafana-util export datasource --output-dir ./datasources"
    ),
    (
        "[Export]",
        "Export access service accounts through the task-first export surface:",
        "grafana-util export access service-account --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-service-accounts"
    ),
    (
        "[Workspace]",
        "Review a local workspace before touching Grafana:",
        r#"grafana-util workspace preview ./grafana-oac-repo --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json"#
    ),
    (
        "[Dashboard]",
        "Analyze dashboards from a raw export tree through the flat dashboard surface:",
        "grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table --report-columns dashboard_uid,panel_title,datasource_uid,query"
    ),
    (
        "[Dashboard]",
        "Export dashboards into a backup tree through the flat dashboard surface:",
        "grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --include-history"
    ),
    (
        "[Alert]",
        "Preview alert routing through the flat alert surface:",
        "grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical"
    ),
    (
        "[Datasource]",
        "Dry-run datasource import with machine-readable output:",
        r#"grafana-util datasource import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input-dir ./datasources --dry-run --json"#
    ),
    (
        "[Access]",
        "Inspect exported access users through the flat access surface:",
        "grafana-util access user list --input-dir ./access-users --json"
    )
);

pub(crate) const ALERT_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Alert Export]",
        "Export alerting resources with overwrite enabled:",
        r#"grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite"#
    ),
    (
        "[Alert Import]",
        "Preview a replace-existing import before execution as structured JSON:",
        r#"grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json"#
    ),
    (
        "[Alert Diff]",
        "Compare a local export against Grafana as structured JSON:",
        r#"grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --output-format json"#
    ),
    (
        "[Alert Plan]",
        "Build a staged alert plan from desired files with linkage maps:",
        "grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output-format json"
    ),
    (
        "[Alert Apply]",
        "Apply a reviewed alert plan only after explicit approval:",
        "grafana-util alert apply --plan-file ./alert-plan-reviewed.json --approve"
    ),
    (
        "[Alert Delete]",
        "Delete one explicit alert resource and allow policy reset only when requested:",
        "grafana-util alert delete --kind policy-tree --identity default --allow-policy-reset"
    ),
    (
        "[Alert Add Rule]",
        "Dry-run a managed rule with routing, labels, and a simple threshold contract before writing files:",
        "grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high' --dry-run"
    ),
    (
        "[Alert Clone Rule]",
        "Dry-run a clone into a new target identity before writing files:",
        "grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform --dry-run"
    ),
    (
        "[Alert Add Contact Point]",
        "Dry-run a managed contact point entry before wiring routes:",
        "grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run"
    ),
    (
        "[Alert Set Route]",
        "Dry-run the tool-owned managed route that will be fully replaced on rerun instead of merged field-by-field:",
        "grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run"
    ),
    (
        "[Alert Preview Route]",
        "Preview route matching inputs from the staged desired tree under the same replace-not-merge managed-route model:",
        "grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical"
    ),
    (
        "[Alert Import]",
        "Re-map linked dashboards and panels during import:",
        "grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json"
    ),
    (
        "[Alert List]",
        "Render live alert rules as JSON:",
        r#"grafana-util alert list-rules --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json"#
    ),
    (
        "[Alert Init]",
        "Initialize a desired-state tree for staged alert management:",
        "grafana-util alert init --desired-dir ./alerts/desired"
    ),
    (
        "[Alert New Rule]",
        "Seed a low-level rule scaffold into the desired-state tree when the higher-level authoring surface is not enough:",
        "grafana-util alert new-rule --desired-dir ./alerts/desired --name cpu-main"
    ),
    (
        "[Alert New Contact Point]",
        "Seed a low-level contact point scaffold directly:",
        "grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary"
    )
);

pub(crate) const DATASOURCE_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Datasource Browse]",
        "Open a live datasource browser:",
        r#"grafana-util datasource browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN""#
    ),
    (
        "[Datasource List]",
        "Enumerate all visible org datasources as CSV:",
        r#"grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format csv"#
    ),
    (
        "[Datasource Add]",
        "Preview a new datasource contract as JSON:",
        r#"grafana-util datasource add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --json"#
    ),
    (
        "[Datasource Import]",
        "Import one exported org bundle with create-missing-orgs:",
        r#"grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json"#
    ),
    (
        "[Datasource Diff]",
        "Compare a local export directory with live Grafana:",
        r#"grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources"#
    ),
    (
        "[Datasource Diff]",
        "Compare a provisioning datasource tree against live Grafana:",
        r#"grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources/provisioning --input-format provisioning"#
    )
);

pub(crate) const ACCESS_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Access Local Inventory]",
        "Inspect exported access users without calling Grafana:",
        "grafana-util access user list --input-dir ./access-users --json"
    ),
    (
        "[Access User Diff]",
        "Compare exported users against the Grafana global scope:",
        "grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global"
    ),
    (
        "[Access Team Import]",
        "Preview a destructive team sync as a table:",
        "grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --dry-run --output-format table"
    ),
    (
        "[Access Org Delete]",
        "Delete one org by explicit org id:",
        "grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 7 --yes --json"
    ),
    (
        "[Access Token Add]",
        "Issue a short-lived service-account token:",
        r#"grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --service-account-id 7 --token-name nightly --seconds-to-live 3600"#
    )
);

pub(crate) const SYNC_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Workspace Summary]",
        "Render the staged workspace package as JSON:",
        "grafana-util workspace scan ./grafana-oac-repo --dashboard-export-dir ./dashboards/raw --output-format json"
    ),
    (
        "[Workspace Advanced Audit]",
        "Compare the current live state against a staged checksum lock:",
        r#"grafana-util workspace ci audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --fail-on-drift --output-format json"#
    ),
    (
        "[Workspace Bundle]",
        "Package exported dashboard and alert artifacts into one workspace package:",
        "grafana-util workspace package ./grafana-oac-repo --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json"
    ),
    (
        "[Workspace Bundle Preflight]",
        "Compare a workspace package against a target inventory snapshot:",
        "grafana-util workspace ci package-test --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output-format json"
    ),
    (
        "[Workspace Preview]",
        "Build a live-backed plan with prune candidates:",
        r#"grafana-util workspace preview --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --allow-prune --output-format json"#
    ),
    (
        "[Workspace Review]",
        "Stamp a reviewed plan with reviewer metadata:",
        "grafana-util workspace ci mark-reviewed --plan-file ./sync-plan.json --review-note 'peer-reviewed' --reviewed-by ops-user --output-format json"
    ),
    (
        "[Workspace Apply]",
        "Emit a reviewed local apply intent:",
        "grafana-util workspace apply --preview-file ./sync-plan-reviewed.json --approve"
    )
);

pub(crate) const HELP_EXAMPLE_LABELS: [(&str, &str); 42] = [
    ("[Config]", HELP_PALETTE.section),
    ("[Status]", HELP_PALETTE.section),
    ("[Export]", HELP_PALETTE.section),
    ("[Workspace]", HELP_PALETTE.section),
    ("[Dashboard]", HELP_PALETTE.section),
    ("[Alert]", HELP_PALETTE.section),
    ("[Datasource]", HELP_PALETTE.section),
    ("[Access]", HELP_PALETTE.section),
    ("[Dashboard import]", HELP_PALETTE.section),
    ("[Alert authoring]", HELP_PALETTE.section),
    ("[Datasource diff]", HELP_PALETTE.section),
    ("[Access administration]", HELP_PALETTE.section),
    ("[Dashboard Export]", HELP_PALETTE.section),
    ("[Dashboard Capture]", HELP_PALETTE.section),
    ("[Dashboard Analyze]", HELP_PALETTE.section),
    ("[Alert Export]", HELP_PALETTE.section),
    ("[Alert Import]", HELP_PALETTE.section),
    ("[Alert List]", HELP_PALETTE.section),
    ("[Datasource Inventory]", HELP_PALETTE.section),
    ("[Datasource List]", HELP_PALETTE.section),
    ("[Datasource Add]", HELP_PALETTE.section),
    ("[Datasource Import]", HELP_PALETTE.section),
    ("[Datasource Local Inventory]", HELP_PALETTE.section),
    ("[Datasource Diff]", HELP_PALETTE.section),
    ("[Access Local Inventory]", HELP_PALETTE.section),
    ("[Access User Diff]", HELP_PALETTE.section),
    ("[Access Team Import]", HELP_PALETTE.section),
    ("[Access Org Delete]", HELP_PALETTE.section),
    ("[Access Token Add]", HELP_PALETTE.section),
    ("[Profile Show]", HELP_PALETTE.section),
    ("[Profile Init]", HELP_PALETTE.section),
    ("[Profile Add]", HELP_PALETTE.section),
    ("[Profile Example]", HELP_PALETTE.section),
    ("[Workspace Planning]", HELP_PALETTE.section),
    ("[Workspace Summary]", HELP_PALETTE.section),
    ("[Workspace Plan]", HELP_PALETTE.section),
    ("[Workspace Review]", HELP_PALETTE.section),
    ("[Workspace Apply]", HELP_PALETTE.section),
    ("[Overview Staged]", HELP_PALETTE.section),
    ("[Overview Bundle]", HELP_PALETTE.section),
    ("[Project Status Staged]", HELP_PALETTE.section),
    ("[Project Status Live]", HELP_PALETTE.section),
];

pub(crate) fn colorize_help_examples(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let colored = match trimmed {
            "Suggested flow:"
            | "Next common commands:"
            | "Examples:"
            | "Extended Examples:"
            | "Notes:"
            | "More help:" => paint_section(trimmed),
            _ if !trimmed.is_empty() && indent.is_empty() && trimmed.ends_with(':') => {
                paint_section(trimmed)
            }
            _ if line.starts_with("Usage: ") => {
                let rest = line.trim_start_matches("Usage: ");
                format!("{} {}", paint_section("Usage:"), paint_command(rest))
            }
            _ if is_cli_command_text(trimmed) => {
                paint_cli_command_line(indent, trimmed).expect("matched CLI command text")
            }
            _ if is_option_entry_line(trimmed) => {
                format!("{indent}{}", paint_option_entry(trimmed))
            }
            _ if trimmed.starts_with("- ") => {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ if trimmed.starts_with('[') && trimmed.contains("--") => {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ if indent == "  " && trimmed.ends_with(':') => {
                format!("{indent}{}", paint_example_caption(trimmed))
            }
            _ if !trimmed.is_empty()
                && indent.len() >= 6
                && !trimmed.starts_with("--")
                && !trimmed.starts_with('[')
                && !is_cli_command_text(trimmed) =>
            {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ => line.to_string(),
        };
        lines.push(colored);
    }
    let mut colored = lines.join("\n");
    for (label, color) in HELP_EXAMPLE_LABELS {
        let colored_label = format!(
            "{reset}{color}{label}{reset}{argument}",
            reset = HELP_PALETTE.reset,
            argument = HELP_PALETTE.argument
        );
        colored = colored.replace(label, &colored_label);
    }
    colored
}

pub(crate) fn colorize_grouped_short_help(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let colored = if let Some(rest) = line.strip_prefix("Usage: ") {
            format!("{} {}", paint_section("Usage:"), paint_command(rest))
        } else if !trimmed.is_empty() && indent.is_empty() && trimmed.ends_with(':') {
            paint_section(trimmed)
        } else if is_cli_command_text(trimmed) {
            paint_cli_command_line(indent, trimmed).expect("matched CLI command text")
        } else if is_option_entry_line(trimmed) {
            format!("{indent}{}", paint_option_entry(trimmed))
        } else if indent == "  " && !trimmed.starts_with('-') {
            match trimmed.find(char::is_whitespace) {
                Some(split_at) => {
                    let (command, rest) = trimmed.split_at(split_at);
                    format!("{indent}{}{}", paint_command(command), rest)
                }
                None => format!("{indent}{}", paint_command(trimmed)),
            }
        } else {
            line.to_string()
        };
        lines.push(colored);
    }
    lines.join("\n")
}

pub(crate) fn colorize_dashboard_subcommand_help(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let colored = match line {
            "Options:" | "What it does:" | "When to use:" | "Related commands:" | "Examples:"
            | "Arguments:" | "More help:" | "Notes:" => paint_section(line),
            _ if !trimmed.is_empty() && indent.is_empty() && trimmed.ends_with(':') => {
                paint_section(trimmed)
            }
            _ if line.starts_with("Usage: ") => {
                let rest = line.trim_start_matches("Usage: ");
                format!("{} {}", paint_section("Usage:"), paint_command(rest))
            }
            _ if is_cli_command_text(trimmed) => {
                paint_cli_command_line(indent, trimmed).expect("matched CLI command text")
            }
            _ if is_option_entry_line(trimmed) => {
                format!("{indent}{}", paint_option_entry(trimmed))
            }
            _ if trimmed.starts_with("- ") => {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ if trimmed.starts_with('[') && trimmed.contains("--") => {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ if indent == "  " && trimmed.ends_with(':') => {
                format!("{indent}{}", paint_example_caption(trimmed))
            }
            _ if !trimmed.is_empty()
                && indent.len() >= 6
                && !trimmed.starts_with("--")
                && !trimmed.starts_with('[')
                && !is_cli_command_text(trimmed)
                && !trimmed.starts_with("- dashboard ") =>
            {
                format!("{indent}{}", paint_support_with_args(trimmed))
            }
            _ if trimmed.starts_with("- dashboard ") => {
                format!("{indent}{}", paint_command(trimmed))
            }
            _ => line.to_string(),
        };
        lines.push(colored);
    }
    lines.join("\n")
}
