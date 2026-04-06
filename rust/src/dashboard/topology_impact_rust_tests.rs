//! Rust regression coverage for Dashboard behavior at this module boundary.

use super::test_support::{
    build_governance_gate_tui_groups, build_governance_gate_tui_items, build_impact_tui_groups,
    filter_impact_tui_items, parse_cli_from, DashboardCliArgs, DashboardCommand,
    DashboardGovernanceGateFinding, DashboardGovernanceGateResult, DashboardGovernanceGateSummary,
    GovernanceGateOutputFormat, ImpactAlertResource, ImpactDashboard, ImpactDocument,
    ImpactOutputFormat, ImpactSummary, TopologyOutputFormat,
};
use crate::dashboard::GovernancePolicySource;
use clap::CommandFactory;
use serde_json::json;
use std::path::PathBuf;

fn render_dashboard_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing subcommand {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn parse_cli_supports_dashboard_governance_gate_command() {
    let args = parse_cli_from([
        "grafana-util",
        "governance-gate",
        "--policy-source",
        "file",
        "--policy",
        "./policy.json",
        "--governance",
        "./governance.json",
        "--queries",
        "./queries.json",
        "--output-format",
        "json",
        "--json-output",
        "./governance-check.json",
    ]);

    match args.command {
        DashboardCommand::GovernanceGate(gate_args) => {
            assert_eq!(gate_args.policy_source, GovernancePolicySource::File);
            assert_eq!(gate_args.policy, Some(PathBuf::from("./policy.json")));
            assert!(gate_args.builtin_policy.is_none());
            assert_eq!(
                gate_args.governance,
                Some(PathBuf::from("./governance.json"))
            );
            assert_eq!(gate_args.queries, Some(PathBuf::from("./queries.json")));
            assert_eq!(gate_args.output_format, GovernanceGateOutputFormat::Json);
            assert_eq!(
                gate_args.json_output,
                Some(PathBuf::from("./governance-check.json"))
            );
        }
        _ => panic!("expected governance-gate command"),
    }
}

#[test]
fn governance_gate_help_mentions_policy_and_queries_inputs() {
    let help = render_dashboard_subcommand_help("governance-gate");

    assert!(help.contains(
        "Check dashboard findings against a policy from live Grafana or a local export tree."
    ));
    assert!(help.contains("--policy-source"));
    assert!(help.contains("--policy"));
    assert!(help.contains("--builtin-policy"));
    assert!(help.contains("JSON or YAML"));
    assert!(help.contains("--import-dir"));
    assert!(help.contains("--governance"));
    assert!(help.contains("--queries"));
    assert!(help.contains("--json-output"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("governance-gate"));
    assert!(help.contains("Check live Grafana directly"));
    assert!(help.contains("Advanced reuse: recheck saved analysis artifacts"));
}

#[test]
fn parse_cli_supports_dashboard_governance_gate_builtin_policy_command() {
    let args = parse_cli_from([
        "grafana-util",
        "governance-gate",
        "--policy-source",
        "builtin",
        "--builtin-policy",
        "default",
        "--governance",
        "./governance.json",
        "--queries",
        "./queries.json",
    ]);

    match args.command {
        DashboardCommand::GovernanceGate(gate_args) => {
            assert_eq!(gate_args.policy_source, GovernancePolicySource::Builtin);
            assert!(gate_args.policy.is_none());
            assert_eq!(gate_args.builtin_policy.as_deref(), Some("default"));
            assert_eq!(
                gate_args.governance,
                Some(PathBuf::from("./governance.json"))
            );
            assert_eq!(gate_args.queries, Some(PathBuf::from("./queries.json")));
            assert_eq!(gate_args.output_format, GovernanceGateOutputFormat::Text);
        }
        _ => panic!("expected governance-gate command"),
    }
}

#[test]
fn governance_gate_help_mentions_interactive_browser() {
    let help = render_dashboard_subcommand_help("governance-gate");
    assert!(help.contains("--interactive"));
}

#[test]
fn parse_cli_supports_dashboard_topology_command() {
    let args = parse_cli_from([
        "grafana-util",
        "topology",
        "--governance",
        "./governance.json",
        "--alert-contract",
        "./alert-contract.json",
        "--output-format",
        "mermaid",
        "--output-file",
        "./dashboard-topology.mmd",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::Topology(topology_args) => {
            assert_eq!(
                topology_args.governance,
                Some(PathBuf::from("./governance.json"))
            );
            assert_eq!(
                topology_args.alert_contract,
                Some(PathBuf::from("./alert-contract.json"))
            );
            assert_eq!(topology_args.output_format, TopologyOutputFormat::Mermaid);
            assert_eq!(
                topology_args.output_file,
                Some(PathBuf::from("./dashboard-topology.mmd"))
            );
            assert!(!topology_args.also_stdout);
            assert!(topology_args.interactive);
        }
        _ => panic!("expected topology command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_topology_also_stdout() {
    let args = parse_cli_from([
        "grafana-util",
        "topology",
        "--governance",
        "./governance.json",
        "--output-file",
        "./dashboard-topology.mmd",
        "--also-stdout",
    ]);

    match args.command {
        DashboardCommand::Topology(topology_args) => {
            assert_eq!(
                topology_args.output_file,
                Some(PathBuf::from("./dashboard-topology.mmd"))
            );
            assert!(topology_args.also_stdout);
        }
        _ => panic!("expected topology command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_topology_live_source_flags() {
    let args = parse_cli_from([
        "grafana-util",
        "topology",
        "--url",
        "http://localhost:3000",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--org-id",
        "7",
        "--page-size",
        "250",
        "--output-format",
        "mermaid",
    ]);

    match args.command {
        DashboardCommand::Topology(topology_args) => {
            assert_eq!(topology_args.common.url, "http://localhost:3000");
            assert_eq!(topology_args.org_id, Some(7));
            assert_eq!(topology_args.page_size, 250);
            assert!(topology_args.import_dir.is_none());
            assert!(topology_args.governance.is_none());
        }
        _ => panic!("expected topology command"),
    }
}

#[test]
fn topology_help_mentions_alert_contract_and_visual_formats() {
    let help = render_dashboard_subcommand_help("topology");

    assert!(help.contains(
        "Show dashboard dependencies directly from live Grafana or a local export tree."
    ));
    assert!(help.contains("--import-dir"));
    assert!(help.contains("--governance"));
    assert!(help.contains("--alert-contract"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--output-file"));
    assert!(help.contains("--also-stdout"));
    assert!(help.contains("--interactive"));
    assert!(help.contains("Analyze live Grafana directly"));
    assert!(help.contains("governance.json"));
    assert!(help.contains("mermaid"));
    assert!(help.contains("dot"));
    assert!(help.contains("graph"));
    assert!(help.contains("dashboard dependencies directly"));
    assert!(help.contains("topology"));
    assert!(help.contains("Advanced reuse: render Graphviz DOT"));
}

#[test]
fn parse_cli_supports_dashboard_governance_gate_import_dir() {
    let args = parse_cli_from([
        "grafana-util",
        "governance-gate",
        "--policy-source",
        "builtin",
        "--builtin-policy",
        "default",
        "--import-dir",
        "./dashboards",
        "--input-format",
        "raw",
    ]);

    match args.command {
        DashboardCommand::GovernanceGate(gate_args) => {
            assert_eq!(gate_args.import_dir, Some(PathBuf::from("./dashboards")));
            assert_eq!(
                gate_args.input_format,
                crate::dashboard::DashboardImportInputFormat::Raw
            );
            assert!(gate_args.governance.is_none());
            assert!(gate_args.queries.is_none());
        }
        _ => panic!("expected governance-gate command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_impact_command() {
    let args = parse_cli_from([
        "grafana-util",
        "impact",
        "--governance",
        "./governance.json",
        "--datasource-uid",
        "prom-main",
        "--alert-contract",
        "./alert-contract.json",
        "--output-format",
        "json",
        "--interactive",
    ]);

    match args.command {
        DashboardCommand::Impact(impact_args) => {
            assert_eq!(
                impact_args.governance,
                Some(PathBuf::from("./governance.json"))
            );
            assert_eq!(impact_args.datasource_uid, "prom-main");
            assert_eq!(
                impact_args.alert_contract,
                Some(PathBuf::from("./alert-contract.json"))
            );
            assert_eq!(impact_args.output_format, ImpactOutputFormat::Json);
            assert!(impact_args.interactive);
        }
        _ => panic!("expected impact command"),
    }
}

#[test]
fn parse_cli_supports_dashboard_impact_import_dir_source() {
    let args = parse_cli_from([
        "grafana-util",
        "impact",
        "--import-dir",
        "./dashboards",
        "--input-format",
        "raw",
        "--datasource-uid",
        "prom-main",
    ]);

    match args.command {
        DashboardCommand::Impact(impact_args) => {
            assert_eq!(impact_args.import_dir, Some(PathBuf::from("./dashboards")));
            assert_eq!(
                impact_args.input_format,
                crate::dashboard::DashboardImportInputFormat::Raw
            );
            assert!(impact_args.governance.is_none());
            assert_eq!(impact_args.datasource_uid, "prom-main");
        }
        _ => panic!("expected impact command"),
    }
}

#[test]
fn impact_help_mentions_datasource_uid_and_output_format() {
    let help = render_dashboard_subcommand_help("impact");

    assert!(help.contains("--import-dir"));
    assert!(help.contains("--url"));
    assert!(help.contains("--governance"));
    assert!(help.contains("--datasource-uid"));
    assert!(help.contains("--alert-contract"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--interactive"));
    assert!(help.contains("blast radius"));
    assert!(help.contains("Check blast radius directly from live Grafana"));
}

#[test]
fn build_impact_tui_groups_summarizes_operator_sections() {
    let document = ImpactDocument {
        summary: ImpactSummary {
            datasource_uid: "prom-main".to_string(),
            dashboard_count: 1,
            alert_resource_count: 4,
            alert_rule_count: 1,
            contact_point_count: 1,
            mute_timing_count: 1,
            notification_policy_count: 1,
            template_count: 0,
        },
        dashboards: vec![ImpactDashboard {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "Platform".to_string(),
            panel_count: 2,
            query_count: 3,
        }],
        alert_resources: vec![
            ImpactAlertResource {
                kind: "alert-rule".to_string(),
                identity: "cpu-high".to_string(),
                title: "CPU High".to_string(),
                source_path: "rules/cpu-high.json".to_string(),
            },
            ImpactAlertResource {
                kind: "mute-timing".to_string(),
                identity: "weekday".to_string(),
                title: "Weekday".to_string(),
                source_path: "mute/weekday.yaml".to_string(),
            },
            ImpactAlertResource {
                kind: "contact-point".to_string(),
                identity: "ops-email".to_string(),
                title: "Ops Email".to_string(),
                source_path: "contact/ops-email.yaml".to_string(),
            },
            ImpactAlertResource {
                kind: "notification-policy".to_string(),
                identity: "default".to_string(),
                title: "Default Policy".to_string(),
                source_path: "policies/default.yaml".to_string(),
            },
        ],
        affected_contact_points: vec![ImpactAlertResource {
            kind: "contact-point".to_string(),
            identity: "ops-email".to_string(),
            title: "Ops Email".to_string(),
            source_path: "contact/ops-email.yaml".to_string(),
        }],
        affected_policies: vec![ImpactAlertResource {
            kind: "notification-policy".to_string(),
            identity: "default".to_string(),
            title: "Default Policy".to_string(),
            source_path: "policies/default.yaml".to_string(),
        }],
        affected_templates: Vec::new(),
    };
    let groups = build_impact_tui_groups(&document);

    assert_eq!(groups[0].label, "All");
    assert_eq!(groups[0].count, 5);
    assert_eq!(groups[1].label, "Dashboards");
    assert_eq!(groups[1].count, 1);
    assert_eq!(groups[2].label, "Alert Rules");
    assert_eq!(groups[2].count, 1);
    assert_eq!(groups[4].label, "Contact Points");
    assert_eq!(groups[4].count, 1);
}

#[test]
fn filter_impact_tui_items_limits_items_to_selected_group() {
    let document = ImpactDocument {
        summary: ImpactSummary {
            datasource_uid: "prom-main".to_string(),
            dashboard_count: 1,
            alert_resource_count: 3,
            alert_rule_count: 1,
            contact_point_count: 1,
            mute_timing_count: 0,
            notification_policy_count: 1,
            template_count: 0,
        },
        dashboards: vec![ImpactDashboard {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "Platform".to_string(),
            panel_count: 2,
            query_count: 3,
        }],
        alert_resources: vec![
            ImpactAlertResource {
                kind: "alert-rule".to_string(),
                identity: "cpu-high".to_string(),
                title: "CPU High".to_string(),
                source_path: "rules/cpu-high.json".to_string(),
            },
            ImpactAlertResource {
                kind: "contact-point".to_string(),
                identity: "ops-email".to_string(),
                title: "Ops Email".to_string(),
                source_path: "contact/ops-email.yaml".to_string(),
            },
            ImpactAlertResource {
                kind: "notification-policy".to_string(),
                identity: "default".to_string(),
                title: "Default Policy".to_string(),
                source_path: "policies/default.yaml".to_string(),
            },
        ],
        affected_contact_points: vec![ImpactAlertResource {
            kind: "contact-point".to_string(),
            identity: "ops-email".to_string(),
            title: "Ops Email".to_string(),
            source_path: "contact/ops-email.yaml".to_string(),
        }],
        affected_policies: vec![ImpactAlertResource {
            kind: "notification-policy".to_string(),
            identity: "default".to_string(),
            title: "Default Policy".to_string(),
            source_path: "policies/default.yaml".to_string(),
        }],
        affected_templates: Vec::new(),
    };
    let dashboard_items = filter_impact_tui_items(&document, "dashboard");
    let alert_rule_items = filter_impact_tui_items(&document, "alert-rule");
    let all_items = filter_impact_tui_items(&document, "all");

    assert_eq!(dashboard_items.len(), 1);
    assert!(dashboard_items.iter().all(|item| item.kind == "dashboard"));
    assert_eq!(alert_rule_items.len(), 1);
    assert!(alert_rule_items
        .iter()
        .all(|item| item.kind == "alert-rule"));
    assert_eq!(all_items.len(), 4);
}

#[test]
fn build_governance_gate_tui_groups_summarizes_findings() {
    let result = DashboardGovernanceGateResult {
        ok: false,
        summary: DashboardGovernanceGateSummary {
            dashboard_count: 2,
            query_record_count: 5,
            violation_count: 2,
            warning_count: 1,
            checked_rules: json!([]),
        },
        violations: vec![
            DashboardGovernanceGateFinding {
                severity: "error".to_string(),
                code: "max-queries-per-dashboard".to_string(),
                risk_kind: "".to_string(),
                dashboard_uid: "cpu-main".to_string(),
                dashboard_title: "CPU Main".to_string(),
                panel_id: "".to_string(),
                panel_title: "".to_string(),
                ref_id: "".to_string(),
                datasource: "".to_string(),
                datasource_uid: "".to_string(),
                datasource_family: "".to_string(),
                message: "too many queries".to_string(),
            },
            DashboardGovernanceGateFinding {
                severity: "error".to_string(),
                code: "forbid-mixed-families".to_string(),
                risk_kind: "".to_string(),
                dashboard_uid: "logs-main".to_string(),
                dashboard_title: "Logs Main".to_string(),
                panel_id: "".to_string(),
                panel_title: "".to_string(),
                ref_id: "".to_string(),
                datasource: "".to_string(),
                datasource_uid: "".to_string(),
                datasource_family: "".to_string(),
                message: "mixed families".to_string(),
            },
        ],
        warnings: vec![DashboardGovernanceGateFinding {
            severity: "warning".to_string(),
            code: "warning-risk".to_string(),
            risk_kind: "broad-loki-selector".to_string(),
            dashboard_uid: "logs-main".to_string(),
            dashboard_title: "Logs Main".to_string(),
            panel_id: "".to_string(),
            panel_title: "".to_string(),
            ref_id: "".to_string(),
            datasource: "".to_string(),
            datasource_uid: "".to_string(),
            datasource_family: "loki".to_string(),
            message: "wide query".to_string(),
        }],
    };

    let groups = build_governance_gate_tui_groups(&result);
    assert_eq!(groups[0].label, "All");
    assert_eq!(groups[0].count, 3);
    assert_eq!(groups[1].label, "Violations");
    assert_eq!(groups[1].count, 2);
    assert_eq!(groups[2].label, "Warnings");
    assert_eq!(groups[2].count, 1);
}

#[test]
fn build_governance_gate_tui_items_filters_by_kind() {
    let result = DashboardGovernanceGateResult {
        ok: false,
        summary: DashboardGovernanceGateSummary {
            dashboard_count: 2,
            query_record_count: 5,
            violation_count: 1,
            warning_count: 1,
            checked_rules: json!([]),
        },
        violations: vec![DashboardGovernanceGateFinding {
            severity: "error".to_string(),
            code: "max-queries-per-dashboard".to_string(),
            risk_kind: "".to_string(),
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            panel_id: "".to_string(),
            panel_title: "".to_string(),
            ref_id: "".to_string(),
            datasource: "".to_string(),
            datasource_uid: "".to_string(),
            datasource_family: "".to_string(),
            message: "too many queries".to_string(),
        }],
        warnings: vec![DashboardGovernanceGateFinding {
            severity: "warning".to_string(),
            code: "warning-risk".to_string(),
            risk_kind: "broad-loki-selector".to_string(),
            dashboard_uid: "logs-main".to_string(),
            dashboard_title: "Logs Main".to_string(),
            panel_id: "".to_string(),
            panel_title: "".to_string(),
            ref_id: "".to_string(),
            datasource: "".to_string(),
            datasource_uid: "".to_string(),
            datasource_family: "loki".to_string(),
            message: "wide query".to_string(),
        }],
    };

    let violation_items = build_governance_gate_tui_items(&result, "violation");
    let warning_items = build_governance_gate_tui_items(&result, "warning");
    let all_items = build_governance_gate_tui_items(&result, "all");

    assert_eq!(violation_items.len(), 1);
    assert_eq!(violation_items[0].kind, "violation");
    assert_eq!(warning_items.len(), 1);
    assert_eq!(warning_items[0].kind, "warning");
    assert_eq!(all_items.len(), 2);
}
