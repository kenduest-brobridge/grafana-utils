//! Import orchestration for Core resources, including input normalization and apply contract handling.

use serde_json::{json, Map, Value};

use crate::common::{message, string_field, Result};

use super::alert_client::GrafanaAlertClient;
use super::alert_compare_support::{
    build_compare_diff_text, build_compare_document, build_resource_identity,
    serialize_compare_document,
};
use super::alert_linkage_support::rewrite_rule_dashboard_linkage;
use super::alert_support::{
    build_import_operation, discover_alert_resource_files, load_alert_resource_file,
    normalize_compare_payload, AlertLinkageMappings,
};
use super::{
    build_alert_diff_document, build_alert_import_dry_run_document, build_auth_context,
    AlertCliArgs, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND, TEMPLATE_KIND,
};

fn count_policy_documents(kind: &str, policies_seen: usize) -> Result<usize> {
    if kind != POLICIES_KIND {
        return Ok(policies_seen);
    }
    let next = policies_seen + 1;
    if next > 1 {
        return Err(message(
            "Multiple notification policy documents found in import set. Import only one policy tree at a time.",
        ));
    }
    Ok(next)
}

fn prepare_import_payload_for_target(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    document: &Value,
    linkage_mappings: &AlertLinkageMappings,
) -> Result<Map<String, Value>> {
    if kind == RULE_KIND {
        return rewrite_rule_dashboard_linkage(client, payload, document, linkage_mappings);
    }
    Ok(payload.clone())
}

fn determine_rule_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let uid = string_field(payload, "uid", "");
    if uid.is_empty() {
        return Ok("would-create");
    }
    match client.get_alert_rule(&uid) {
        Ok(_) if replace_existing => Ok("would-update"),
        Ok(_) => Ok("would-fail-existing"),
        Err(error) if error.status_code() == Some(404) => Ok("would-create"),
        Err(error) => Err(error),
    }
}

fn determine_contact_point_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let uid = string_field(payload, "uid", "");
    let exists = client
        .list_contact_points()?
        .into_iter()
        .any(|item| string_field(&item, "uid", "") == uid);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_mute_timing_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let name = string_field(payload, "name", "");
    let exists = client
        .list_mute_timings()?
        .into_iter()
        .any(|item| string_field(&item, "name", "") == name);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_template_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let name = string_field(payload, "name", "");
    let exists = client
        .list_templates()?
        .into_iter()
        .any(|item| string_field(&item, "name", "") == name);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_import_action(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    match kind {
        RULE_KIND => determine_rule_import_action(client, payload, replace_existing),
        CONTACT_POINT_KIND => {
            determine_contact_point_import_action(client, payload, replace_existing)
        }
        MUTE_TIMING_KIND => determine_mute_timing_import_action(client, payload, replace_existing),
        TEMPLATE_KIND => determine_template_import_action(client, payload, replace_existing),
        POLICIES_KIND => Ok("would-update"),
        _ => unreachable!(),
    }
}

fn fetch_live_compare_document(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Option<Value>> {
    match kind {
        RULE_KIND => {
            let uid = string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok(None);
            }
            match client.get_alert_rule(&uid) {
                Ok(remote) => Ok(Some(build_compare_document(
                    kind,
                    &normalize_compare_payload(kind, &remote),
                ))),
                Err(error) if error.status_code() == Some(404) => Ok(None),
                Err(error) => Err(error),
            }
        }
        CONTACT_POINT_KIND => {
            let uid = string_field(payload, "uid", "");
            let remote = client
                .list_contact_points()?
                .into_iter()
                .find(|item| string_field(item, "uid", "") == uid);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        MUTE_TIMING_KIND => {
            let name = string_field(payload, "name", "");
            let remote = client
                .list_mute_timings()?
                .into_iter()
                .find(|item| string_field(item, "name", "") == name);
            Ok(remote
                .map(|item| build_compare_document(kind, &normalize_compare_payload(kind, &item))))
        }
        TEMPLATE_KIND => {
            let name = string_field(payload, "name", "");
            match client.get_template(&name) {
                Ok(remote) => Ok(Some(build_compare_document(
                    kind,
                    &normalize_compare_payload(kind, &remote),
                ))),
                Err(error) if error.status_code() == Some(404) => Ok(None),
                Err(error) => Err(error),
            }
        }
        POLICIES_KIND => {
            let remote = client.get_notification_policies()?;
            Ok(Some(build_compare_document(
                kind,
                &normalize_compare_payload(kind, &remote),
            )))
        }
        _ => unreachable!(),
    }
}

fn import_rule_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let uid = string_field(payload, "uid", "");
    if replace_existing && !uid.is_empty() {
        match client.get_alert_rule(&uid) {
            Ok(_) => {
                let result = client.update_alert_rule(&uid, payload)?;
                return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
            }
            Err(error) if error.status_code() == Some(404) => {}
            Err(error) => return Err(error),
        }
    }
    let result = client.create_alert_rule(payload)?;
    Ok(("created".to_string(), string_field(&result, "uid", &uid)))
}

fn import_contact_point_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let uid = string_field(payload, "uid", "");
    if replace_existing && !uid.is_empty() {
        let existing: Vec<String> = client
            .list_contact_points()?
            .into_iter()
            .map(|item| string_field(&item, "uid", ""))
            .collect();
        if existing.iter().any(|item| item == &uid) {
            let result = client.update_contact_point(&uid, payload)?;
            return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
        }
    }
    let result = client.create_contact_point(payload)?;
    Ok(("created".to_string(), string_field(&result, "uid", &uid)))
}

fn import_mute_timing_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let name = string_field(payload, "name", "");
    if replace_existing && !name.is_empty() {
        let existing: Vec<String> = client
            .list_mute_timings()?
            .into_iter()
            .map(|item| string_field(&item, "name", ""))
            .collect();
        if existing.iter().any(|item| item == &name) {
            let result = client.update_mute_timing(&name, payload)?;
            return Ok(("updated".to_string(), string_field(&result, "name", &name)));
        }
    }
    let result = client.create_mute_timing(payload)?;
    Ok(("created".to_string(), string_field(&result, "name", &name)))
}

fn import_template_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let name = string_field(payload, "name", "");
    let existing_templates = client.list_templates()?;
    let exists = existing_templates
        .iter()
        .any(|item| string_field(item, "name", "") == name);
    if exists && !replace_existing {
        return Err(message(format!(
            "Template {name:?} already exists. Use --replace-existing."
        )));
    }

    let mut template_payload = payload.clone();
    if exists {
        let current = client.get_template(&name)?;
        template_payload.insert(
            "version".to_string(),
            Value::String(string_field(&current, "version", "")),
        );
    } else {
        template_payload.insert("version".to_string(), Value::String(String::new()));
    }

    let result = client.update_template(&name, &template_payload)?;
    Ok((
        (if exists { "updated" } else { "created" }).to_string(),
        string_field(&result, "name", &name),
    ))
}

fn import_policies_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
) -> Result<(String, String)> {
    client.update_notification_policies(payload)?;
    Ok((
        "updated".to_string(),
        string_field(payload, "receiver", "root"),
    ))
}

fn import_resource_document(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    args: &AlertCliArgs,
) -> Result<(String, String)> {
    match kind {
        RULE_KIND => import_rule_document(client, payload, args.replace_existing),
        CONTACT_POINT_KIND => import_contact_point_document(client, payload, args.replace_existing),
        MUTE_TIMING_KIND => import_mute_timing_document(client, payload, args.replace_existing),
        TEMPLATE_KIND => import_template_document(client, payload, args.replace_existing),
        POLICIES_KIND => import_policies_document(client, payload),
        _ => unreachable!(),
    }
}

pub(crate) fn import_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let import_dir = args
        .import_dir
        .as_ref()
        .ok_or_else(|| message("Import directory is required for alerting import."))?;
    let resource_files = discover_alert_resource_files(import_dir)?;
    let linkage_mappings = AlertLinkageMappings::load(
        args.dashboard_uid_map.as_deref(),
        args.panel_id_map.as_deref(),
    )?;
    let mut policies_seen = 0usize;
    let mut dry_run_rows: Vec<Value> = Vec::new();

    if args.json && !args.dry_run {
        return Err(message(
            "--json for alert import is only supported with --dry-run.",
        ));
    }

    for resource_file in &resource_files {
        let document = load_alert_resource_file(resource_file, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        let payload = prepare_import_payload_for_target(
            &client,
            &kind,
            &payload,
            &document,
            &linkage_mappings,
        )?;
        policies_seen = count_policy_documents(&kind, policies_seen)?;
        let identity = build_resource_identity(&kind, &payload);
        if args.dry_run {
            let action = determine_import_action(&client, &kind, &payload, args.replace_existing)?;
            if args.json {
                dry_run_rows.push(json!({
                    "path": resource_file.to_string_lossy().to_string(),
                    "kind": kind,
                    "identity": identity,
                    "action": action,
                }));
                continue;
            }
            println!(
                "Dry-run {} -> kind={} id={} action={}",
                resource_file.display(),
                kind,
                identity,
                action
            );
            continue;
        }

        let (action, identity) = import_resource_document(&client, &kind, &payload, args)?;
        println!(
            "Imported {} -> kind={} id={} action={}",
            resource_file.display(),
            kind,
            identity,
            action
        );
    }

    if args.dry_run {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_alert_import_dry_run_document(&dry_run_rows))?
            );
            return Ok(());
        }
        println!(
            "Dry-run checked {} alerting resource files from {}",
            resource_files.len(),
            import_dir.display()
        );
    } else {
        println!(
            "Imported {} alerting resource files from {}",
            resource_files.len(),
            import_dir.display()
        );
    }
    Ok(())
}

pub(crate) fn diff_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let diff_dir = args
        .diff_dir
        .as_ref()
        .ok_or_else(|| message("Diff directory is required for alerting diff."))?;
    let resource_files = discover_alert_resource_files(diff_dir)?;
    let linkage_mappings = AlertLinkageMappings::load(
        args.dashboard_uid_map.as_deref(),
        args.panel_id_map.as_deref(),
    )?;
    let mut policies_seen = 0usize;
    let mut differences = 0usize;
    let mut diff_rows: Vec<Value> = Vec::new();

    for resource_file in &resource_files {
        let document = load_alert_resource_file(resource_file, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        let payload = prepare_import_payload_for_target(
            &client,
            &kind,
            &payload,
            &document,
            &linkage_mappings,
        )?;
        policies_seen = count_policy_documents(&kind, policies_seen)?;
        let identity = build_resource_identity(&kind, &payload);
        let local_compare =
            build_compare_document(&kind, &normalize_compare_payload(&kind, &payload));
        let remote_compare = fetch_live_compare_document(&client, &kind, &payload)?;

        if let Some(remote_compare) = remote_compare {
            if serialize_compare_document(&local_compare)?
                == serialize_compare_document(&remote_compare)?
            {
                if args.json {
                    diff_rows.push(json!({
                        "path": resource_file.to_string_lossy().to_string(),
                        "kind": kind,
                        "identity": identity,
                        "action": "same",
                    }));
                    continue;
                }
                println!(
                    "Diff same {} -> kind={} id={}",
                    resource_file.display(),
                    kind,
                    identity
                );
                continue;
            }

            if args.json {
                diff_rows.push(json!({
                    "path": resource_file.to_string_lossy().to_string(),
                    "kind": kind,
                    "identity": identity,
                    "action": "different",
                }));
                differences += 1;
                continue;
            }
            println!(
                "Diff different {} -> kind={} id={}",
                resource_file.display(),
                kind,
                identity
            );
            print!(
                "{}",
                build_compare_diff_text(&remote_compare, &local_compare, &identity, resource_file)?
            );
            differences += 1;
            continue;
        }

        if args.json {
            diff_rows.push(json!({
                "path": resource_file.to_string_lossy().to_string(),
                "kind": kind,
                "identity": identity,
                "action": "missing-remote",
            }));
            differences += 1;
            continue;
        }
        println!(
            "Diff missing-remote {} -> kind={} id={}",
            resource_file.display(),
            kind,
            identity
        );
        print!(
            "{}",
            build_compare_diff_text(&json!({}), &local_compare, &identity, resource_file)?
        );
        differences += 1;
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_alert_diff_document(&diff_rows))?
        );
    }

    if differences > 0 {
        return Err(message(format!(
            "Found {differences} alerting differences across {} files.",
            resource_files.len()
        )));
    }

    println!(
        "No alerting differences across {} files.",
        resource_files.len()
    );
    Ok(())
}
