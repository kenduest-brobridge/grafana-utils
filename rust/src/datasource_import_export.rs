//! Datasource import/export orchestration.
//!
//! Maintainer notes:
//! - Keep secret placeholder handling fail-closed: dry-run may describe required
//!   placeholders, but live import must resolve every placeholder before issuing
//!   any write request.
//! - Keep routed `--use-export-org` imports explicit: plan org routing first,
//!   then execute one scoped import per destination org.

use crate::common::{message, Result};
use crate::dashboard::{build_api_client, build_http_client_for_org_from_api};
use crate::grafana_api::DatasourceResourceClient;
use crate::http::JsonHttpClient;

use super::render_import_table;
use super::{DatasourceImportArgs, DatasourceImportInputFormat};

#[path = "datasource_export_support.rs"]
mod datasource_export_support;
#[path = "datasource_import_dry_run.rs"]
mod datasource_import_dry_run;
#[path = "datasource_import_export_routed.rs"]
mod datasource_import_export_routed;
#[path = "datasource_import_export_support.rs"]
mod datasource_import_export_support;
#[path = "datasource_import_payload.rs"]
mod datasource_import_payload;
#[path = "datasource_import_plan.rs"]
mod datasource_import_plan;

pub(crate) use datasource_export_support::{
    build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir,
    build_datasource_export_metadata, build_datasource_provisioning_document, build_export_index,
    build_export_records, build_list_records, datasource_list_column_ids,
    describe_datasource_import_mode, render_data_source_csv, render_data_source_json,
    render_data_source_summary_line, render_data_source_table, resolve_target_client,
    validate_import_org_auth, write_yaml_file, DATASOURCE_PROVISIONING_FILENAME,
    DATASOURCE_PROVISIONING_SUBDIR,
};
pub(crate) use datasource_import_dry_run::{
    build_datasource_import_dry_run_json_value, collect_datasource_import_dry_run_report,
    print_datasource_import_dry_run_report,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use datasource_import_export_routed::format_routed_datasource_scope_summary_fields;
#[allow(unused_imports)]
pub(crate) use datasource_import_export_routed::{
    build_routed_datasource_import_dry_run_json, format_routed_datasource_import_summary_line,
    format_routed_datasource_source_org_label, format_routed_datasource_target_org_label,
    render_routed_datasource_import_org_table, resolve_export_org_target_plan,
};
#[allow(unused_imports)]
pub(crate) use datasource_import_export_support::{
    classify_datasource_export_root_scope_kind, create_org,
    discover_datasource_inventory_scope_dirs, discover_export_org_import_scopes, fetch_current_org,
    list_orgs, load_datasource_export_root_manifest,
    load_datasource_inventory_records_from_export_root, load_diff_record_values,
    load_import_records, org_id_string_from_value, resolve_datasource_export_root_dir,
    validate_matching_export_org, DatasourceExportOrgScope, DatasourceExportOrgTargetPlan,
    DatasourceExportRootManifest, DatasourceExportRootScopeKind, DatasourceImportDryRunReport,
    DatasourceImportRecord, DATASOURCE_EXPORT_FILENAME, EXPORT_METADATA_FILENAME, ROOT_INDEX_KIND,
};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use datasource_import_payload::build_import_payload;
#[allow(unused_imports)]
pub(crate) use datasource_import_payload::{
    build_import_payload_with_secret_values, parse_secret_values_inputs,
};
#[allow(unused_imports)]
pub(crate) use datasource_import_plan::{
    prepare_datasource_import_plan, PreparedDatasourceImportPlan, PreparedDatasourceImportRequest,
};

pub(crate) fn import_datasources_with_client(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<usize> {
    if args.dry_run {
        let report = collect_datasource_import_dry_run_report(client, args)?;
        print_datasource_import_dry_run_report(&report, args)?;
        return Ok(0);
    }

    let replace_existing = args.replace_existing || args.update_existing_only;
    let (_metadata, records) = load_import_records(&args.input_dir, args.input_format)?;
    let secret_values = parse_secret_values_inputs(
        args.secret_values.as_deref(),
        args.secret_values_file.as_deref(),
    )?;
    validate_matching_export_org(client, args, &records)?;

    let live = DatasourceResourceClient::new(client).list_datasources()?;
    let plan = prepare_datasource_import_plan(
        &records,
        &live,
        replace_existing,
        args.update_existing_only,
        secret_values.as_ref(),
    )?;
    for request in &plan.requests {
        client.request_json(
            request.method.clone(),
            &request.path,
            &[],
            Some(&request.payload),
        )?;
    }
    println!(
        "Imported {} datasource(s) from {}; updated {}, skipped {}, blocked {}",
        plan.would_create + plan.would_update,
        args.input_dir.display(),
        plan.would_update,
        plan.would_skip,
        0usize
    );
    Ok(plan.would_create + plan.would_update)
}

pub(crate) fn import_datasources_by_export_org(args: &DatasourceImportArgs) -> Result<usize> {
    let admin_api = build_api_client(&args.common)?;
    let admin_client = admin_api.http_client();
    let scopes = discover_export_org_import_scopes(args)?;

    if args.dry_run && args.json {
        println!("{}", build_routed_datasource_import_dry_run_json(args)?);
        return Ok(0);
    }

    let mut org_rows = Vec::new();
    let mut plans = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.input_dir, args.input_format)?
            .1
            .len();
        org_rows.push(vec![
            plan.source_org_id.to_string(),
            if plan.source_org_name.is_empty() {
                "-".to_string()
            } else {
                plan.source_org_name.clone()
            },
            plan.org_action.to_string(),
            format_routed_datasource_target_org_label(plan.target_org_id),
            datasource_count.to_string(),
            plan.input_dir.display().to_string(),
        ]);
        plans.push(plan);
    }
    let source_org_labels = plans
        .iter()
        .map(|plan| {
            format_routed_datasource_source_org_label(plan.source_org_id, &plan.source_org_name)
        })
        .collect::<Vec<String>>();
    if args.dry_run && args.table {
        for line in render_routed_datasource_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        let existing_org_count = plans
            .iter()
            .filter(|plan| plan.org_action == "exists")
            .count();
        let missing_org_count = plans
            .iter()
            .filter(|plan| plan.org_action == "missing")
            .count();
        let would_create_org_count = plans
            .iter()
            .filter(|plan| plan.org_action == "would-create")
            .count();
        let datasource_count = org_rows
            .iter()
            .filter_map(|row| row.get(4))
            .filter_map(|value| value.parse::<usize>().ok())
            .sum();
        println!(
            "{}",
            format_routed_datasource_import_summary_line(
                org_rows.len(),
                &source_org_labels,
                existing_org_count,
                missing_org_count,
                would_create_org_count,
                datasource_count,
                &args.input_dir,
            )
        );
        return Ok(0);
    }
    if args.dry_run && args.json {
        println!("{}", build_routed_datasource_import_dry_run_json(args)?);
        return Ok(0);
    }

    let mut imported = 0usize;
    for plan in plans {
        let scoped_args = DatasourceImportArgs {
            common: args.common.clone(),
            input_dir: plan.input_dir.clone(),
            input_format: args.input_format,
            org_id: None,
            use_export_org: false,
            only_org_id: Vec::new(),
            create_missing_orgs: args.create_missing_orgs,
            require_matching_export_org: args.require_matching_export_org,
            replace_existing: args.replace_existing,
            update_existing_only: args.update_existing_only,
            secret_values: args.secret_values.clone(),
            secret_values_file: args.secret_values_file.clone(),
            dry_run: false,
            table: args.table,
            json: args.json,
            output_format: args.output_format,
            no_header: args.no_header,
            output_columns: args.output_columns.clone(),
            list_columns: args.list_columns,
            progress: args.progress,
            verbose: args.verbose,
        };
        let target_org_id = plan.target_org_id.ok_or_else(|| {
            message(format!(
                "Datasource import for export org {} did not resolve a destination org id.",
                plan.source_org_id
            ))
        })?;
        let scoped_client = build_http_client_for_org_from_api(&admin_api, target_org_id)?;
        imported +=
            import_datasources_with_client(&scoped_client, &scoped_args).map_err(|error| {
                message(format!(
                    "Datasource import for export org {} failed: {error}",
                    plan.source_org_id
                ))
            })?;
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn build_import_payload_resolves_secret_placeholders_into_secure_json_data() {
        let record = DatasourceImportRecord {
            uid: "loki-main".to_string(),
            name: "Loki Main".to_string(),
            datasource_type: "loki".to_string(),
            access: "proxy".to_string(),
            url: "http://loki:3100".to_string(),
            is_default: false,
            org_name: String::new(),
            org_id: "1".to_string(),
            basic_auth: Some(true),
            basic_auth_user: "loki-user".to_string(),
            database: "logs-main".to_string(),
            json_data: json!({
                "httpMethod": "POST",
                "httpHeaderName1": "X-Scope-OrgID",
            })
            .as_object()
            .cloned(),
            secure_json_data_placeholders: json!({
                "basicAuthPassword": "${secret:loki-basic-auth}",
                "httpHeaderValue1": "${secret:loki-tenant-token}",
            })
            .as_object()
            .cloned(),
            user: "query-user".to_string(),
            with_credentials: Some(true),
        };
        let secret_values = json!({
            "loki-basic-auth": "secret-value",
            "loki-tenant-token": "tenant-token",
        });

        let payload =
            build_import_payload_with_secret_values(&record, secret_values.as_object()).unwrap();

        assert_eq!(
            payload["secureJsonData"]["basicAuthPassword"],
            json!("secret-value")
        );
        assert_eq!(
            payload["secureJsonData"]["httpHeaderValue1"],
            json!("tenant-token")
        );
    }

    #[test]
    fn prepare_datasource_import_plan_resolves_all_payloads_before_writes() {
        let records = vec![
            DatasourceImportRecord {
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: "proxy".to_string(),
                url: "http://prometheus:9090".to_string(),
                is_default: false,
                org_name: String::new(),
                org_id: "1".to_string(),
                basic_auth: None,
                basic_auth_user: String::new(),
                database: String::new(),
                json_data: None,
                secure_json_data_placeholders: None,
                user: String::new(),
                with_credentials: None,
            },
            DatasourceImportRecord {
                uid: "loki-main".to_string(),
                name: "Loki Main".to_string(),
                datasource_type: "loki".to_string(),
                access: "proxy".to_string(),
                url: "http://loki:3100".to_string(),
                is_default: false,
                org_name: String::new(),
                org_id: "1".to_string(),
                basic_auth: Some(true),
                basic_auth_user: "loki-user".to_string(),
                database: "logs-main".to_string(),
                json_data: json!({
                    "httpMethod": "POST",
                    "httpHeaderName1": "X-Scope-OrgID",
                })
                .as_object()
                .cloned(),
                secure_json_data_placeholders: json!({
                    "basicAuthPassword": "${secret:loki-basic-auth}",
                    "httpHeaderValue1": "${secret:loki-tenant-token}",
                })
                .as_object()
                .cloned(),
                user: "query-user".to_string(),
                with_credentials: Some(true),
            },
        ];
        let live = Vec::<Map<String, Value>>::new();
        let secret_values = json!({
            "loki-basic-auth": "secret-value",
            "loki-tenant-token": "tenant-token",
        });

        let plan = prepare_datasource_import_plan(
            &records,
            &live,
            false,
            false,
            secret_values.as_object(),
        )
        .unwrap();

        assert_eq!(plan.would_create, 2);
        assert_eq!(plan.would_update, 0);
        assert_eq!(plan.would_skip, 0);
        assert_eq!(plan.requests.len(), 2);
        assert_eq!(plan.requests[1].method, reqwest::Method::POST);
        assert_eq!(plan.requests[1].path, "/api/datasources");
        assert_eq!(
            plan.requests[1].payload["secureJsonData"]["basicAuthPassword"],
            json!("secret-value")
        );
        assert_eq!(
            plan.requests[1].payload["secureJsonData"]["httpHeaderValue1"],
            json!("tenant-token")
        );
    }

    #[test]
    fn parse_secret_values_inputs_reads_json_file() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("secret-values.json");
        fs::write(&path, "{\n  \"loki-basic-auth\": \"secret-value\"\n}\n").unwrap();

        let values = parse_secret_values_inputs(None, Some(&path))
            .unwrap()
            .expect("values");

        assert_eq!(values["loki-basic-auth"], json!("secret-value"));
    }
}
