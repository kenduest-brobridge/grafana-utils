//! Build staged alert export artifacts from live Grafana data.
//!
//! Responsibilities:
//! - Resolve alert resource trees from Grafana APIs and serialize each resource kind
//!   into a stable on-disk layout.
//! - Build and write root and per-kind manifest indexes used by import/diff/review.
//! - Keep alert export output deterministic for downstream snapshot and sync consumers.

use serde_json::{json, Value};
use std::fs;

use crate::common::{message, string_field, write_json_file, Result};

use super::alert_client::GrafanaAlertClient;
use super::alert_compare_support::{
    append_root_index_item, format_export_summary, write_resource_indexes,
};
use super::{
    build_auth_context, build_contact_point_export_document, build_contact_point_output_path,
    build_empty_root_index, build_mute_timing_export_document, build_mute_timing_output_path,
    build_policies_export_document, build_policies_output_path, build_resource_dirs,
    build_rule_export_document, build_rule_output_path, build_template_export_document,
    build_template_output_path, AlertCliArgs, CONTACT_POINTS_SUBDIR, CONTACT_POINT_KIND,
    MUTE_TIMINGS_SUBDIR, MUTE_TIMING_KIND, POLICIES_KIND, POLICIES_SUBDIR, RAW_EXPORT_SUBDIR,
    RULES_SUBDIR, RULE_KIND, TEMPLATES_SUBDIR, TEMPLATE_KIND,
};

pub(crate) fn export_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let output_dir = args.output_dir.clone();
    let raw_dir = output_dir.join(RAW_EXPORT_SUBDIR);
    fs::create_dir_all(&raw_dir)?;

    let resource_dirs = build_resource_dirs(&raw_dir);
    for path in resource_dirs.values() {
        fs::create_dir_all(path)?;
    }

    let rules = client.list_alert_rules()?;
    let contact_points = client.list_contact_points()?;
    let mute_timings = client.list_mute_timings()?;
    let policies = client.get_notification_policies()?;
    let templates = client.list_templates()?;

    let mut root_index = build_empty_root_index();

    for rule in rules {
        let mut normalized_rule = rule.clone();
        if let Some(linked_dashboard) =
            super::alert_linkage_support::build_linked_dashboard_metadata(&client, &rule)?
        {
            normalized_rule.insert(
                "__linkedDashboardMetadata__".to_string(),
                Value::Object(linked_dashboard),
            );
        }
        let document = build_rule_export_document(&normalized_rule);
        let spec = document["spec"]
            .as_object()
            .ok_or_else(|| message("Rule export spec must be an object."))?;
        let output_path = build_rule_output_path(&resource_dirs[RULE_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            RULES_SUBDIR,
            json!({
                "kind": RULE_KIND,
                "uid": string_field(spec, "uid", ""),
                "title": string_field(spec, "title", ""),
                "folderUID": string_field(spec, "folderUID", ""),
                "ruleGroup": string_field(spec, "ruleGroup", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!(
            "Exported alert rule {} -> {}",
            string_field(spec, "uid", "unknown"),
            output_path.display()
        );
    }

    for contact_point in contact_points {
        let document = build_contact_point_export_document(&contact_point);
        let spec = document["spec"]
            .as_object()
            .ok_or_else(|| message("Contact-point export spec must be an object."))?;
        let output_path =
            build_contact_point_output_path(&resource_dirs[CONTACT_POINT_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            CONTACT_POINTS_SUBDIR,
            json!({
                "kind": CONTACT_POINT_KIND,
                "uid": string_field(spec, "uid", ""),
                "name": string_field(spec, "name", ""),
                "type": string_field(spec, "type", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!(
            "Exported contact point {} -> {}",
            string_field(spec, "uid", "unknown"),
            output_path.display()
        );
    }

    for mute_timing in mute_timings {
        let document = build_mute_timing_export_document(&mute_timing);
        let spec = document["spec"]
            .as_object()
            .ok_or_else(|| message("Mute-timing export spec must be an object."))?;
        let output_path =
            build_mute_timing_output_path(&resource_dirs[MUTE_TIMING_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            MUTE_TIMINGS_SUBDIR,
            json!({
                "kind": MUTE_TIMING_KIND,
                "name": string_field(spec, "name", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!(
            "Exported mute timing {} -> {}",
            string_field(spec, "name", "unknown"),
            output_path.display()
        );
    }

    let policies_document = build_policies_export_document(&policies);
    let policies_path = build_policies_output_path(&resource_dirs[POLICIES_KIND]);
    write_json_file(&policies_path, &policies_document, args.overwrite)?;
    append_root_index_item(
        &mut root_index,
        POLICIES_SUBDIR,
        json!({
            "kind": POLICIES_KIND,
            "receiver": policies_document["spec"]["receiver"],
            "path": policies_path.to_string_lossy(),
        }),
    );
    println!(
        "Exported notification policies {} -> {}",
        policies_document["spec"]["receiver"]
            .as_str()
            .unwrap_or("unknown"),
        policies_path.display()
    );

    for template in templates {
        let document = build_template_export_document(&template);
        let spec = document["spec"]
            .as_object()
            .ok_or_else(|| message("Template export spec must be an object."))?;
        let output_path =
            build_template_output_path(&resource_dirs[TEMPLATE_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            TEMPLATES_SUBDIR,
            json!({
                "kind": TEMPLATE_KIND,
                "name": string_field(spec, "name", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!(
            "Exported template {} -> {}",
            string_field(spec, "name", "unknown"),
            output_path.display()
        );
    }

    write_resource_indexes(&resource_dirs, &root_index)?;
    let index_path = output_dir.join("index.json");
    write_json_file(&index_path, &Value::Object(root_index.clone()), true)?;
    println!("{}", format_export_summary(&root_index, &index_path));
    Ok(())
}
