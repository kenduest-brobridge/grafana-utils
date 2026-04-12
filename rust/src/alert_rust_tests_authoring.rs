use super::{
    build_import_operation, build_managed_policy_route_preview,
    build_new_rule_scaffold_document_with_route, build_stable_route_label_value,
    init_alert_runtime_layout, load_alert_resource_file, run_alert_cli,
    write_contact_point_scaffold, write_new_contact_point_scaffold, write_new_rule_scaffold,
    write_new_template_scaffold, AlertCliArgs, CONTACT_POINT_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND,
};
use serde_json::{json, Value};
use tempfile::tempdir;

#[test]
fn alert_runtime_init_and_scaffolds_write_valid_desired_files() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("alerts-managed");

    let init = init_alert_runtime_layout(&root).unwrap();
    assert_eq!(init["root"], json!(root.to_string_lossy().to_string()));
    assert!(root.join("rules").is_dir());
    assert!(root.join("contact-points").is_dir());
    assert!(root.join("templates").is_dir());

    let rule_path = root.join("rules").join("rule.yaml");
    let contact_point_path = root.join("contact-points").join("contact-point.json");
    let template_path = root.join("templates").join("template.yaml");

    write_new_rule_scaffold(&rule_path, "cpu-main", true).unwrap();
    write_new_contact_point_scaffold(&contact_point_path, "pagerduty-primary", true).unwrap();
    write_new_template_scaffold(&template_path, "sev1-notification", true).unwrap();

    let (rule_kind, _) =
        build_import_operation(&load_alert_resource_file(&rule_path, "rule scaffold").unwrap())
            .unwrap();
    let (contact_point_kind, _) = build_import_operation(
        &load_alert_resource_file(&contact_point_path, "contact point scaffold").unwrap(),
    )
    .unwrap();
    let (template_kind, _) = build_import_operation(
        &load_alert_resource_file(&template_path, "template scaffold").unwrap(),
    )
    .unwrap();

    assert_eq!(rule_kind, RULE_KIND);
    assert_eq!(contact_point_kind, CONTACT_POINT_KIND);
    assert_eq!(template_kind, TEMPLATE_KIND);
    assert_eq!(
        load_alert_resource_file(&rule_path, "rule scaffold").unwrap()["spec"]["uid"],
        json!("cpu-main")
    );
    assert_eq!(
        load_alert_resource_file(&contact_point_path, "contact point scaffold").unwrap()["spec"]
            ["name"],
        json!("pagerduty-primary")
    );
    assert_eq!(
        load_alert_resource_file(&template_path, "template scaffold").unwrap()["spec"]["name"],
        json!("sev1-notification")
    );
}

#[test]
fn managed_route_helpers_build_stable_rule_authoring_contracts() {
    assert_eq!(
        super::alert_support::stable_route_label_key(),
        "grafana_utils_route"
    );
    assert_eq!(
        build_stable_route_label_value("Team Alerts / Primary"),
        "Team_Alerts_Primary"
    );
    assert_eq!(
        super::alert_support::build_stable_route_matcher("Team Alerts / Primary"),
        json!(["grafana_utils_route", "=", "Team_Alerts_Primary"])
    );

    let folder_contract =
        super::alert_support::build_folder_resolution_contract("infra", Some("Infrastructure"));
    assert_eq!(folder_contract["folderUid"], json!("infra"));
    assert_eq!(folder_contract["folderTitle"], json!("Infrastructure"));
    assert_eq!(folder_contract["resolution"], json!("uid-or-title"));

    let rule_body =
        super::alert_support::build_simple_rule_body("CPU High", "infra", "cpu", "Team Alerts");
    assert_eq!(rule_body["folderUID"], json!("infra"));
    assert_eq!(rule_body["ruleGroup"], json!("cpu"));
    assert_eq!(
        rule_body["labels"]["grafana_utils_route"],
        json!("Team_Alerts")
    );

    let scaffold =
        build_new_rule_scaffold_document_with_route("CPU High", "infra", "cpu", "Team Alerts");
    assert_eq!(scaffold["kind"], json!(RULE_KIND));
    assert_eq!(
        scaffold["spec"]["labels"]["grafana_utils_route"],
        json!("Team_Alerts")
    );
    assert_eq!(scaffold["metadata"]["folder"]["folderUid"], json!("infra"));
    assert_eq!(
        scaffold["metadata"]["route"]["labelKey"],
        json!("grafana_utils_route")
    );
    assert_eq!(
        scaffold["metadata"]["route"]["labelValue"],
        json!("Team_Alerts")
    );
}

#[test]
fn managed_policy_subtree_upsert_is_idempotent_and_leaves_unmanaged_routes_untouched() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]],
                "routes": [{"receiver": "legacy-nested"}]
            }
        ]
    });
    let desired_route = json!({
        "receiver": "team-webhook",
        "group_by": ["alertname", "grafana_folder"],
        "routes": [{"receiver": "team-slack"}]
    });

    let (first_policy, first_action) = super::alert_support::upsert_managed_policy_subtree(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        desired_route.as_object().unwrap(),
    )
    .unwrap();
    assert_eq!(first_action, "created");
    assert_eq!(first_policy["routes"].as_array().unwrap().len(), 2);
    assert_eq!(first_policy["routes"][0], current_policy["routes"][0]);
    let managed_route = first_policy["routes"][1].as_object().unwrap();
    assert!(super::alert_support::route_matches_stable_label(
        managed_route,
        "Team Alerts"
    ));
    assert_eq!(managed_route["receiver"], json!("team-webhook"));
    assert_eq!(
        managed_route["object_matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );

    let preview = build_managed_policy_route_preview(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        Some(desired_route.as_object().unwrap()),
    )
    .unwrap();
    assert_eq!(preview["action"], json!("created"));
    assert_eq!(preview["managedRouteValue"], json!("Team_Alerts"));
    assert_eq!(
        preview["nextRoute"]["groupBy"],
        json!(["alertname", "grafana_folder"])
    );
    assert_eq!(
        preview["nextRoute"]["matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );

    let (second_policy, second_action) = super::alert_support::upsert_managed_policy_subtree(
        &first_policy,
        "Team Alerts",
        desired_route.as_object().unwrap(),
    )
    .unwrap();
    assert_eq!(second_action, "noop");
    assert_eq!(second_policy, first_policy);
}

#[test]
fn managed_policy_subtree_remove_only_touches_tool_owned_route() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]]
            },
            {
                "receiver": "team-webhook",
                "object_matchers": [["grafana_utils_route", "=", "Team_Alerts"]]
            }
        ]
    });

    let (next_policy, action) = super::alert_support::remove_managed_policy_subtree(
        current_policy.as_object().unwrap(),
        "Team Alerts",
    )
    .unwrap();
    assert_eq!(action, "deleted");
    assert_eq!(next_policy["routes"].as_array().unwrap().len(), 1);
    assert_eq!(next_policy["routes"][0], current_policy["routes"][0]);

    let preview = build_managed_policy_route_preview(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        None,
    )
    .unwrap();
    assert_eq!(preview["action"], json!("deleted"));
    assert_eq!(preview["nextRoute"], Value::Null);

    let (noop_policy, noop_action) =
        super::alert_support::remove_managed_policy_subtree(&next_policy, "Team Alerts").unwrap();
    assert_eq!(noop_action, "noop");
    assert_eq!(noop_policy, next_policy);
}

#[test]
fn runtime_managed_policy_helpers_produce_idempotent_documents() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]]
            }
        ]
    });
    let desired_route = json!({
        "receiver": "team-webhook",
        "group_by": ["alertname", "grafana_folder"],
        "object_matchers": [["grafana_utils_route", "=", "old-value"]],
        "routes": [{"receiver": "team-slack"}]
    });

    let preview = super::alert_runtime_support::build_managed_policy_edit_preview_document(
        &current_policy,
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(
        preview["kind"],
        json!("grafana-util-alert-managed-policy-preview")
    );
    assert_eq!(preview["schemaVersion"], json!(1));
    assert_eq!(preview["toolVersion"], json!(super::TOOL_VERSION));
    assert_eq!(preview["reviewRequired"], json!(true));
    assert_eq!(preview["reviewed"], json!(false));
    assert_eq!(preview["preview"]["action"], json!("created"));
    assert_eq!(
        preview["preview"]["nextRoute"]["groupBy"],
        json!(["alertname", "grafana_folder"])
    );
    assert_eq!(
        preview["preview"]["nextRoute"]["matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );

    let first_apply = super::alert_runtime_support::apply_managed_policy_subtree_edit_document(
        &current_policy,
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(first_apply["kind"], json!(POLICIES_KIND));
    assert_eq!(first_apply["action"], json!("created"));
    assert_eq!(
        first_apply["spec"]["routes"][1]["object_matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );
    assert_eq!(
        first_apply["spec"]["routes"][0],
        current_policy["routes"][0]
    );

    let second_apply = super::alert_runtime_support::apply_managed_policy_subtree_edit_document(
        &first_apply["spec"],
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(second_apply["action"], json!("noop"));
    assert_eq!(second_apply["spec"], first_apply["spec"]);
}

#[test]
fn contact_point_scaffolds_cover_webhook_email_and_slack_authoring_shapes() {
    let webhook =
        super::alert_support::build_contact_point_scaffold_document("team-webhook", "webhook");
    let email = super::alert_support::build_contact_point_scaffold_document("team-email", "email");
    let slack = super::alert_support::build_contact_point_scaffold_document("team-slack", "slack");

    assert_eq!(webhook["spec"]["type"], json!("webhook"));
    assert_eq!(
        webhook["spec"]["settings"]["url"],
        json!("http://127.0.0.1:9000/notify")
    );
    assert_eq!(email["spec"]["type"], json!("email"));
    assert_eq!(
        email["spec"]["settings"]["addresses"],
        json!(["alerts@example.com"])
    );
    assert_eq!(slack["spec"]["type"], json!("slack"));
    assert_eq!(slack["spec"]["settings"]["recipient"], json!("#alerts"));
    assert_eq!(
        slack["metadata"]["authoring"]["settingsKeys"],
        json!(["recipient", "text", "token"])
    );

    let temp = tempdir().unwrap();
    let slack_path = temp.path().join("team-slack.yaml");
    write_contact_point_scaffold(&slack_path, "team-slack", "slack", true).unwrap();
    let written = load_alert_resource_file(&slack_path, "typed contact point scaffold").unwrap();
    let (kind, payload) = build_import_operation(&written).unwrap();
    assert_eq!(kind, CONTACT_POINT_KIND);
    assert_eq!(payload["type"], json!("slack"));
    assert_eq!(payload["settings"]["recipient"], json!("#alerts"));
}

#[test]
fn run_alert_cli_add_rule_writes_desired_rule_and_managed_policy_files() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let args: AlertCliArgs = super::parse_cli_from([
        "grafana-util alert",
        "add-rule",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--name",
        "cpu-high",
        "--folder",
        "platform-alerts",
        "--rule-group",
        "cpu",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--severity",
        "critical",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
    ]);
    run_alert_cli(args).unwrap();

    let rule_path = desired_dir.join("rules").join("cpu-high.yaml");
    let policy_path = desired_dir
        .join("policies")
        .join("notification-policies.yaml");
    let rule = load_alert_resource_file(&rule_path, "authored rule").unwrap();
    let policy = load_alert_resource_file(&policy_path, "managed policy").unwrap();

    assert_eq!(rule["kind"], json!(RULE_KIND));
    assert_eq!(rule["spec"]["folderUID"], json!("platform-alerts"));
    assert_eq!(rule["spec"]["ruleGroup"], json!("cpu"));
    assert_eq!(rule["spec"]["labels"]["team"], json!("platform"));
    assert_eq!(rule["spec"]["labels"]["severity"], json!("critical"));
    assert_eq!(
        rule["spec"]["labels"]["grafana_utils_route"],
        json!("pagerduty-primary")
    );
    assert_eq!(policy["kind"], json!(POLICIES_KIND));
    assert_eq!(
        policy["spec"]["routes"][0]["receiver"],
        json!("pagerduty-primary")
    );
    assert_eq!(
        policy["spec"]["routes"][0]["object_matchers"][0],
        json!(["grafana_utils_route", "=", "pagerduty-primary"])
    );
}

#[test]
fn run_alert_cli_clone_rule_dry_run_leaves_target_files_absent() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let source_path = desired_dir.join("rules").join("cpu-high.yaml");
    write_new_rule_scaffold(&source_path, "cpu-high", true).unwrap();

    let args: AlertCliArgs = super::parse_cli_from([
        "grafana-util alert",
        "clone-rule",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--no-route",
        "--dry-run",
    ]);
    run_alert_cli(args).unwrap();

    assert!(!desired_dir
        .join("rules")
        .join("cpu-high-staging.yaml")
        .exists());
    assert!(!desired_dir
        .join("policies")
        .join("notification-policies.yaml")
        .exists());
}

#[test]
fn run_alert_cli_set_route_overwrites_managed_route_in_place() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let first = super::parse_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
    ]);
    run_alert_cli(first).unwrap();

    let second = super::parse_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=infra",
    ]);
    run_alert_cli(second).unwrap();

    let policy = load_alert_resource_file(
        &desired_dir
            .join("policies")
            .join("notification-policies.yaml"),
        "managed policy",
    )
    .unwrap();
    let routes = policy["spec"]["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["receiver"], json!("pagerduty-primary"));
    assert_eq!(
        routes[0]["object_matchers"],
        json!([
            ["grafana_utils_route", "=", "pagerduty-primary"],
            ["team", "=", "infra"]
        ])
    );
}
