//! Unified CLI help examples and rendering helpers.
//!
//! Keeping the large example blocks and help rendering here lets `cli.rs`
//! stay focused on command topology and dispatch.

use clap::{ColorChoice, CommandFactory};

use crate::access::root_command as access_root_command;
use crate::alert::root_command as alert_root_command;
use crate::alert_sync::ALERT_SYNC_KIND;
use crate::cli::CliArgs;
use crate::cli_help_examples::{
    colorize_help_examples, inject_help_full_hint, ACCESS_HELP_FULL_TEXT, ALERT_HELP_FULL_TEXT,
    DATASOURCE_HELP_FULL_TEXT, OVERVIEW_HELP_FULL_TEXT, PROJECT_STATUS_HELP_FULL_TEXT,
    SYNC_HELP_FULL_TEXT, UNIFIED_HELP_FULL_TEXT, UNIFIED_HELP_TEXT,
};
use crate::datasource::root_command as datasource_root_command;
use crate::overview::OverviewCliArgs;
use crate::profile_cli::root_command as profile_root_command;
use crate::project_status_command::ProjectStatusCliArgs;
use crate::snapshot::root_command as snapshot_root_command;
use crate::sync::SyncCliArgs;

pub(crate) const UNIFIED_DASHBOARD_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard get --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n  grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n  grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution exact\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n  grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n  grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table";
pub(crate) const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource inspect-export --input-dir ./datasources --json\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --json";
pub(crate) const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util change summary --desired-file ./desired.json\n  grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
pub(crate) const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";
pub(crate) const UNIFIED_PROFILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util profile list\n  grafana-util profile show --profile prod --output-format yaml\n  grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file\n  grafana-util profile example --mode basic\n  grafana-util profile example --mode full\n  grafana-util profile init --overwrite";
pub(crate) const DASHBOARD_BROWSE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs";
pub(crate) const DASHBOARD_GET_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard get --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n  grafana-util dashboard get --profile prod --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json";
pub(crate) const DASHBOARD_CLONE_LIVE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n  grafana-util dashboard clone-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json";
pub(crate) const DASHBOARD_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util dashboard list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const DASHBOARD_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n  grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite";
pub(crate) const DASHBOARD_RAW_TO_PROMPT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n  grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution strict --output-format json\n  grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite";
pub(crate) const DASHBOARD_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing\n  grafana-util dashboard import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./dashboards/raw --dry-run --table\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --interactive --replace-existing";
pub(crate) const DASHBOARD_PATCH_FILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n  grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'";
pub(crate) const DASHBOARD_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard review --input ./drafts/cpu-main.json\n  grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml";
pub(crate) const DASHBOARD_PUBLISH_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table";
pub(crate) const DASHBOARD_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n  grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n  grafana-util dashboard delete --url http://localhost:3000 --interactive";
pub(crate) const DASHBOARD_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --import-dir ./dashboards/raw --json";
pub(crate) const DASHBOARD_INSPECT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --table\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --interactive\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --report governance-json\n  grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report tree-table";
pub(crate) const DASHBOARD_INSPECT_LIVE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n  grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive";
pub(crate) const DASHBOARD_INSPECT_VARS_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format table\n  grafana-util dashboard inspect-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token \"$GRAFANA_API_TOKEN\" --output-format json";
pub(crate) const DASHBOARD_GOVERNANCE_GATE_HELP_TEXT: &str = "Examples:\n\n  Build artifacts first with inspect-live or inspect-export, then evaluate governance policy:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json --output-file ./governance.json\n    grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json\n\n  Use the builtin policy and emit a machine-readable report:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json";
pub(crate) const DASHBOARD_TOPOLOGY_HELP_TEXT: &str = "Examples:\n\n  Build artifacts first with inspect-live or inspect-export, then render topology as Mermaid:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json --output-file ./governance.json\n    grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid\n\n  Render Graphviz DOT with the graph alias:\n    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot";
pub(crate) const DASHBOARD_SCREENSHOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n  grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'";
pub(crate) const SNAPSHOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./snapshot --overwrite\n  grafana-util snapshot review --input-dir ./snapshot --output-format text\n  grafana-util snapshot review --input-dir ./snapshot --output-format json\n  grafana-util snapshot review --input-dir ./snapshot --interactive";

const OVERVIEW_HELP_SHAPE_NOTE: &str =
    "\nStaged overview is the default. Use `grafana-util overview live` to route into shared live status.\n";

const CHANGE_SCHEMA_ROOT_HELP_TEXT: &str = "Change JSON schema guide\n\nUse this help surface when CI, scripts, or external tooling need to understand which JSON document each `change` subcommand emits.\n\nGeneral rule:\n- Branch on `kind` plus `schemaVersion` before reading nested fields.\n- `--output-format json` is the machine-readable path.\n- `change bundle` writes a bundle contract through `--output-file`; it does not use `--output-format`.\n\nSubcommand contracts:\n- summary -> grafana-utils-sync-summary
  top-level keys: kind, schemaVersion, toolVersion, summary, resources
- plan -> grafana-utils-sync-plan
  top-level keys: kind, schemaVersion, toolVersion, dryRun, reviewRequired, reviewed, allowPrune, summary, alertAssessment, operations, traceId, stage, stepIndex, parentTraceId
- review -> grafana-utils-sync-plan
  same base shape as plan, plus reviewedBy, reviewedAt, reviewNote; lineage moves to stage=review
- apply -> grafana-utils-sync-apply-intent
  top-level keys: kind, schemaVersion, toolVersion, mode, reviewed, reviewRequired, allowPrune, approved, summary, alertAssessment, operations, optional preflightSummary, optional bundlePreflightSummary, appliedBy, appliedAt, approvalReason, applyNote, traceId, stage, stepIndex, parentTraceId
- apply --execute-live -> live apply result
  top-level keys: mode, appliedCount, results
- audit -> grafana-utils-sync-audit
  top-level keys: kind, schemaVersion, toolVersion, summary, currentLock, baselineLock, drifts
- preflight -> grafana-utils-sync-preflight
  top-level keys: kind, schemaVersion, toolVersion, summary, checks
- assess-alerts -> ";
const CHANGE_SCHEMA_ROOT_HELP_TAIL: &str = "
  top-level keys: kind, schemaVersion, toolVersion, summary, alerts
- bundle-preflight -> grafana-utils-sync-bundle-preflight
  top-level keys: kind, schemaVersion, summary, syncPreflight, alertArtifactAssessment, secretPlaceholderAssessment, providerAssessment
- promotion-preflight -> grafana-utils-sync-promotion-preflight
  top-level keys: kind, schemaVersion, toolVersion, summary, bundlePreflight, mappingSummary, checkSummary, handoffSummary, continuationSummary, checks, resolvedChecks, blockingChecks

Quick lookups:
- grafana-util change --help-schema
- grafana-util change plan --help-schema
- grafana-util change apply --help-schema";
const CHANGE_SUMMARY_SCHEMA_HELP_TEXT: &str = "Change summary JSON schema\n\nCommand:\n  grafana-util change summary --desired-file ./desired.json --output-format json\n\nkind:\n  grafana-utils-sync-summary\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
  - resourceCount
  - dashboardCount
  - datasourceCount
  - folderCount
  - alertCount
- resources[]
  - kind
  - identity
  - title
  - managedFields
  - bodyFieldCount
  - sourcePath";
const CHANGE_PLAN_SCHEMA_HELP_TEXT: &str = "Change plan JSON schema\n\nCommand:\n  grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod --output-format json\n\nkind:\n  grafana-utils-sync-plan\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- dryRun
- reviewRequired
- reviewed
- allowPrune
- traceId
- stage
- stepIndex
- parentTraceId
- summary
  - would_create
  - would_update
  - would_delete
  - noop
  - unmanaged
  - alert_candidate
  - alert_plan_only
  - alert_blocked
- alertAssessment
- operations[]
  - kind
  - identity
  - title
  - action
  - reason
  - changedFields
  - managedFields
  - desired
  - live
  - sourcePath";
const CHANGE_REVIEW_SCHEMA_HELP_TEXT: &str = "Change review JSON schema\n\nCommand:\n  grafana-util change review --plan-file ./sync-plan.json --output-format json\n\nkind:\n  grafana-utils-sync-plan\n\nBase shape:
- same JSON envelope as `change plan`
\nReview additions:
- reviewed = true
- stage = review
- stepIndex = 2
- reviewedBy
- reviewedAt
- reviewNote";
const CHANGE_APPLY_SCHEMA_HELP_TEXT: &str = "Change apply JSON schema\n\nCommands:\n  grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --output-format json
  grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --profile prod --output-format json\n\nDefault apply-intent kind:
  grafana-utils-sync-apply-intent
\nApply-intent top-level keys:
- kind
- schemaVersion
- toolVersion
- mode
- reviewed
- reviewRequired
- allowPrune
- approved
- summary
- alertAssessment
- operations
- optional preflightSummary
- optional bundlePreflightSummary
- appliedBy
- appliedAt
- approvalReason
- applyNote
- traceId
- stage
- stepIndex
- parentTraceId
\nLive execute shape (`--execute-live`):
- mode = live-apply
- appliedCount
- results[]
  - kind
  - identity
  - action
  - response";
const CHANGE_AUDIT_SCHEMA_HELP_TEXT: &str = "Change audit JSON schema\n\nCommand:\n  grafana-util change audit --lock-file ./sync-lock.json --fetch-live --profile prod --output-format json\n\nkind:\n  grafana-utils-sync-audit\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
  - managedCount
  - baselineCount
  - currentPresentCount
  - currentMissingCount
  - inSyncCount
  - driftCount
  - missingLockCount
  - missingLiveCount
- currentLock
- baselineLock
- drifts[]
  - kind
  - identity
  - title
  - status
  - baselineStatus
  - currentStatus
  - baselineChecksum
  - currentChecksum
  - driftedFields
  - sourcePath";
const CHANGE_PREFLIGHT_SCHEMA_HELP_TEXT: &str = "Change preflight JSON schema\n\nCommand:\n  grafana-util change preflight --desired-file ./desired.json --fetch-live --profile prod --output-format json\n\nkind:\n  grafana-utils-sync-preflight\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
  - checkCount
  - okCount
  - blockingCount
- checks[]
  - kind
  - identity
  - status
  - detail
  - blocking";
const CHANGE_ASSESS_ALERTS_SCHEMA_HELP_TEXT_HEAD: &str = "Change assess-alerts JSON schema\n\nCommand:\n  grafana-util change assess-alerts --alerts-file ./alerts.json --output-format json\n\nkind:\n  ";
const CHANGE_ASSESS_ALERTS_SCHEMA_HELP_TEXT_TAIL: &str = "\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
- alerts[]";
const CHANGE_BUNDLE_PREFLIGHT_SCHEMA_HELP_TEXT: &str = "Change bundle-preflight JSON schema\n\nCommand:\n  grafana-util change bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --output-format json\n\nkind:\n  grafana-utils-sync-bundle-preflight\n\nTop-level keys:
- kind
- schemaVersion
- summary
- syncPreflight
- alertArtifactAssessment
- secretPlaceholderAssessment
- providerAssessment";
const CHANGE_PROMOTION_PREFLIGHT_SCHEMA_HELP_TEXT: &str = "Change promotion-preflight JSON schema\n\nCommand:\n  grafana-util change promotion-preflight --source-bundle ./bundle.json --target-inventory ./target.json --mapping-file ./promotion-map.json --output-format json\n\nkind:\n  grafana-utils-sync-promotion-preflight\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
- bundlePreflight
- mappingSummary
- checkSummary
- handoffSummary
- continuationSummary
- checks[]
- resolvedChecks[]
- blockingChecks[]";
const CHANGE_BUNDLE_SCHEMA_HELP_TEXT: &str = "Change bundle JSON contract\n\nCommand:\n  grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n\nNotes:
- `change bundle` writes the contract to `--output-file`.
- It does not switch render modes with `--output-format`.
- Add `--also-stdout` only when you intentionally want both file output and stdout.\n\nkind:
  grafana-utils-sync-source-bundle\n\nTop-level keys:
- kind
- schemaVersion
- toolVersion
- summary
- resources
- providers
- secretPlaceholders
- alertArtifacts";

fn render_long_help_with_color_choice(command: &mut clap::Command, colorize: bool) -> String {
    let configured = std::mem::take(command).color(if colorize {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    });
    *command = configured;
    let rendered = command.render_long_help();
    if colorize {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    }
}

fn render_domain_help_text(mut command: clap::Command, colorize: bool) -> String {
    inject_help_full_hint(render_long_help_with_color_choice(&mut command, colorize))
}

fn render_domain_help_full_text(
    mut command: clap::Command,
    extended_examples: &str,
    colorize: bool,
) -> String {
    let mut help = render_long_help_with_color_choice(&mut command, colorize);
    if colorize {
        help.push_str(&colorize_help_examples(extended_examples));
    } else {
        help.push_str(extended_examples);
    }
    help
}

fn render_overview_help_text(colorize: bool) -> String {
    let mut help = render_domain_help_text(OverviewCliArgs::command(), colorize);
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

fn render_overview_help_full_text(colorize: bool) -> String {
    let mut help = render_domain_help_full_text(
        OverviewCliArgs::command(),
        OVERVIEW_HELP_FULL_TEXT,
        colorize,
    );
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

pub fn render_unified_help_text(colorize: bool) -> String {
    let mut command = CliArgs::command();
    let help = inject_help_full_hint(render_long_help_with_color_choice(&mut command, colorize));
    let mut help = if colorize {
        help.replace(
            UNIFIED_HELP_TEXT,
            &colorize_help_examples(UNIFIED_HELP_TEXT),
        )
    } else {
        help
    };
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

pub fn render_unified_help_full_text(colorize: bool) -> String {
    let mut help = render_unified_help_text(colorize);
    if colorize {
        help.push_str(&colorize_help_examples(UNIFIED_HELP_FULL_TEXT));
    } else {
        help.push_str(UNIFIED_HELP_FULL_TEXT);
    }
    help
}

pub fn render_unified_version_text() -> String {
    format!("grafana-util {}\n", crate::common::TOOL_VERSION)
}

fn render_change_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(format!(
            "{}{}{}",
            CHANGE_SCHEMA_ROOT_HELP_TEXT, ALERT_SYNC_KIND, CHANGE_SCHEMA_ROOT_HELP_TAIL
        )),
        Some("summary") => Some(CHANGE_SUMMARY_SCHEMA_HELP_TEXT.to_string()),
        Some("plan") => Some(CHANGE_PLAN_SCHEMA_HELP_TEXT.to_string()),
        Some("review") => Some(CHANGE_REVIEW_SCHEMA_HELP_TEXT.to_string()),
        Some("apply") => Some(CHANGE_APPLY_SCHEMA_HELP_TEXT.to_string()),
        Some("audit") => Some(CHANGE_AUDIT_SCHEMA_HELP_TEXT.to_string()),
        Some("preflight") => Some(CHANGE_PREFLIGHT_SCHEMA_HELP_TEXT.to_string()),
        Some("assess-alerts") => Some(format!(
            "{}{}{}",
            CHANGE_ASSESS_ALERTS_SCHEMA_HELP_TEXT_HEAD,
            ALERT_SYNC_KIND,
            CHANGE_ASSESS_ALERTS_SCHEMA_HELP_TEXT_TAIL
        )),
        Some("bundle-preflight") => Some(CHANGE_BUNDLE_PREFLIGHT_SCHEMA_HELP_TEXT.to_string()),
        Some("promotion-preflight") => {
            Some(CHANGE_PROMOTION_PREFLIGHT_SCHEMA_HELP_TEXT.to_string())
        }
        Some("bundle") => Some(CHANGE_BUNDLE_SCHEMA_HELP_TEXT.to_string()),
        _ => None,
    }
}

pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if args.len() >= 3
        && args.get(1).map(String::as_str) == Some("change")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(2)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_change_schema_help(target);
    }
    match args.as_slice() {
        [_binary] => Some(render_unified_help_text(colorize)),
        [_binary, flag] if flag == "--help" || flag == "-h" => {
            Some(render_unified_help_text(colorize))
        }
        [_binary, flag] if flag == "--help-full" => Some(render_unified_help_full_text(colorize)),
        [_binary, command, flag] if command == "alert" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(alert_root_command(), colorize))
        }
        [_binary, command, flag]
            if command == "datasource" && (flag == "--help" || flag == "-h") =>
        {
            Some(render_domain_help_text(datasource_root_command(), colorize))
        }
        [_binary, command, flag] if command == "access" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(access_root_command(), colorize))
        }
        [_binary, command, flag] if command == "profile" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(profile_root_command(), colorize))
        }
        [_binary, command, flag] if command == "snapshot" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(snapshot_root_command(), colorize))
        }
        [_binary, command, flag] if command == "overview" && (flag == "--help" || flag == "-h") => {
            Some(render_overview_help_text(colorize))
        }
        [_binary, command, flag] if command == "status" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(
                ProjectStatusCliArgs::command(),
                colorize,
            ))
        }
        [_binary, command, flag] if command == "change" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(SyncCliArgs::command(), colorize))
        }
        [_binary, command, flag] if command == "alert" && flag == "--help-full" => Some(
            render_domain_help_full_text(alert_root_command(), ALERT_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "datasource" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                datasource_root_command(),
                DATASOURCE_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "access" && flag == "--help-full" => Some(
            render_domain_help_full_text(access_root_command(), ACCESS_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "profile" && flag == "--help-full" => {
            Some(render_domain_help_text(profile_root_command(), colorize))
        }
        [_binary, command, flag] if command == "snapshot" && flag == "--help-full" => {
            Some(render_domain_help_text(snapshot_root_command(), colorize))
        }
        [_binary, command, flag] if command == "overview" && flag == "--help-full" => {
            Some(render_overview_help_full_text(colorize))
        }
        [_binary, command, flag] if command == "status" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                ProjectStatusCliArgs::command(),
                PROJECT_STATUS_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "change" && flag == "--help-full" => Some(
            render_domain_help_full_text(SyncCliArgs::command(), SYNC_HELP_FULL_TEXT, colorize),
        ),
        _ => None,
    }
}
