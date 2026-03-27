use reqwest::Method;
use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{message, Result};
use crate::dashboard::{
    build_http_client, build_http_client_for_org, list_datasources, DEFAULT_ORG_ID,
};
use crate::datasource::{render_import_table, resolve_match, DatasourceImportArgs};
use crate::datasource_secret::{
    build_secret_placeholder_plan, describe_secret_placeholder_plan, resolve_secret_placeholders,
    summarize_secret_placeholder_plan,
};
use crate::http::JsonHttpClient;

#[path = "datasource_export_support.rs"]
mod datasource_export_support;
#[path = "datasource_import_export_routed.rs"]
mod datasource_import_export_routed;
#[path = "datasource_import_export_support.rs"]
mod datasource_import_export_support;

pub(crate) use datasource_export_support::{
    build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir,
    build_datasource_export_metadata, build_export_index, build_export_records, build_list_records,
    describe_datasource_import_mode, export_datasource_scope, render_data_source_csv,
    render_data_source_json, render_data_source_table, resolve_target_client,
    validate_import_org_auth,
};
pub(crate) use datasource_import_export_routed::{
    build_routed_datasource_import_dry_run_json, format_routed_datasource_scope_summary_fields,
    format_routed_datasource_target_org_label, render_routed_datasource_import_org_table,
    resolve_export_org_target_plan,
};
pub(crate) use datasource_import_export_support::{
    create_org, discover_export_org_import_scopes, fetch_current_org, list_orgs,
    load_diff_record_values, load_import_records, org_id_string_from_value,
    validate_matching_export_org, DatasourceExportMetadata, DatasourceExportOrgScope,
    DatasourceExportOrgTargetPlan, DatasourceImportDryRunReport, DatasourceImportRecord,
    DATASOURCE_EXPORT_FILENAME, EXPORT_METADATA_FILENAME,
};

fn build_import_secret_visibility_entries(import_dir: &Path) -> Vec<Value> {
    let Ok((_, records)) = load_import_records(import_dir) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for record in records {
        let Some(placeholders) = &record.secure_json_data_placeholders else {
            continue;
        };
        let datasource_spec = Map::from_iter(vec![
            ("uid".to_string(), Value::String(record.uid.clone())),
            ("name".to_string(), Value::String(record.name.clone())),
            (
                "type".to_string(),
                Value::String(record.datasource_type.clone()),
            ),
            (
                "secureJsonDataPlaceholders".to_string(),
                Value::Object(placeholders.clone()),
            ),
        ]);
        match build_secret_placeholder_plan(&datasource_spec) {
            Ok(plan) => entries.push(summarize_secret_placeholder_plan(&plan)),
            Err(error) => entries.push(Value::Object(Map::from_iter(vec![
                (
                    "datasourceUid".to_string(),
                    Value::String(record.uid.clone()),
                ),
                (
                    "datasourceName".to_string(),
                    Value::String(record.name.clone()),
                ),
                (
                    "datasourceType".to_string(),
                    Value::String(record.datasource_type.clone()),
                ),
                (
                    "providerKind".to_string(),
                    Value::String("inline-placeholder-map".to_string()),
                ),
                (
                    "action".to_string(),
                    Value::String("secret-plan-error".to_string()),
                ),
                ("reviewRequired".to_string(), Value::Bool(true)),
                ("error".to_string(), Value::String(error.to_string())),
            ]))),
        }
    }
    entries
}

struct PreparedDatasourceImportRequest {
    method: Method,
    path: String,
    payload: Value,
}

struct PreparedDatasourceImportPlan {
    requests: Vec<PreparedDatasourceImportRequest>,
    would_create: usize,
    would_update: usize,
    would_skip: usize,
}

// Precompute all payloads before any write call so secret resolution fails closed.
fn prepare_datasource_import_plan(
    records: &[DatasourceImportRecord],
    live: &[Map<String, Value>],
    replace_existing: bool,
    update_existing_only: bool,
    secret_values: Option<&Map<String, Value>>,
) -> Result<PreparedDatasourceImportPlan> {
    let mut requests = Vec::new();
    let mut would_create = 0usize;
    let mut would_update = 0usize;
    let mut would_skip = 0usize;

    for record in records {
        let matching = resolve_match(record, live, replace_existing, update_existing_only);
        match matching.action {
            "would-create" => {
                let payload = build_import_payload_with_secret_values_impl(record, secret_values)?;
                requests.push(PreparedDatasourceImportRequest {
                    method: Method::POST,
                    path: "/api/datasources".to_string(),
                    payload,
                });
                would_create += 1;
            }
            "would-update" => {
                let target_id = matching.target_id.ok_or_else(|| {
                    message(format!(
                        "Matched datasource {} does not expose a usable numeric id for update.",
                        matching.target_name
                    ))
                })?;
                let payload = build_import_payload_with_secret_values_impl(record, secret_values)?;
                requests.push(PreparedDatasourceImportRequest {
                    method: Method::PUT,
                    path: format!("/api/datasources/{target_id}"),
                    payload,
                });
                would_update += 1;
            }
            "would-skip-missing" => {
                would_skip += 1;
            }
            _ => {
                return Err(message(format!(
                    "Datasource import blocked for {}: destination={} action={}.",
                    if record.uid.is_empty() {
                        &record.name
                    } else {
                        &record.uid
                    },
                    matching.destination,
                    matching.action
                )));
            }
        }
    }

    Ok(PreparedDatasourceImportPlan {
        requests,
        would_create,
        would_update,
        would_skip,
    })
}

pub(crate) fn collect_datasource_import_dry_run_report(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<DatasourceImportDryRunReport> {
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.import_dir)?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let mode = describe_datasource_import_mode(args.replace_existing, args.update_existing_only);
    let mut rows = Vec::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut blocked = 0usize;
    for (index, record) in records.iter().enumerate() {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        let file_ref = format!("{}#{}", metadata.datasources_file, index);
        rows.push(vec![
            record.uid.clone(),
            record.name.clone(),
            record.datasource_type.clone(),
            matching.destination.to_string(),
            matching.action.to_string(),
            target_org_id.clone(),
            file_ref,
        ]);
        match matching.action {
            "would-create" => created += 1,
            "would-update" => updated += 1,
            "would-skip-missing" => skipped += 1,
            _ => blocked += 1,
        }
    }
    Ok(DatasourceImportDryRunReport {
        mode: mode.to_string(),
        import_dir: args.import_dir.clone(),
        source_org_id: records
            .iter()
            .find(|item| !item.org_id.is_empty())
            .map(|item| item.org_id.clone())
            .unwrap_or_default(),
        target_org_id,
        rows,
        datasource_count: records.len(),
        would_create: created,
        would_update: updated,
        would_skip: skipped,
        would_block: blocked,
    })
}

pub(crate) fn build_datasource_import_dry_run_json_value(
    report: &DatasourceImportDryRunReport,
) -> Value {
    let secret_visibility = build_import_secret_visibility_entries(&report.import_dir);
    Value::Object(Map::from_iter(vec![
        ("mode".to_string(), Value::String(report.mode.clone())),
        (
            "sourceOrgId".to_string(),
            Value::String(report.source_org_id.clone()),
        ),
        (
            "targetOrgId".to_string(),
            Value::String(report.target_org_id.clone()),
        ),
        (
            "datasources".to_string(),
            Value::Array(
                report
                    .rows
                    .iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("uid".to_string(), Value::String(row[0].clone())),
                            ("name".to_string(), Value::String(row[1].clone())),
                            ("type".to_string(), Value::String(row[2].clone())),
                            ("destination".to_string(), Value::String(row[3].clone())),
                            ("action".to_string(), Value::String(row[4].clone())),
                            ("orgId".to_string(), Value::String(row[5].clone())),
                            ("file".to_string(), Value::String(row[6].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((report.datasource_count as i64).into()),
                ),
                (
                    "wouldCreate".to_string(),
                    Value::Number((report.would_create as i64).into()),
                ),
                (
                    "wouldUpdate".to_string(),
                    Value::Number((report.would_update as i64).into()),
                ),
                (
                    "wouldSkip".to_string(),
                    Value::Number((report.would_skip as i64).into()),
                ),
                (
                    "wouldBlock".to_string(),
                    Value::Number((report.would_block as i64).into()),
                ),
                (
                    "secretVisibilityCount".to_string(),
                    Value::Number((secret_visibility.len() as i64).into()),
                ),
            ])),
        ),
        (
            "secretVisibility".to_string(),
            Value::Array(secret_visibility),
        ),
    ]))
}

pub(crate) fn print_datasource_import_dry_run_report(
    report: &DatasourceImportDryRunReport,
    args: &DatasourceImportArgs,
) -> Result<()> {
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_datasource_import_dry_run_json_value(report))?
        );
    } else if args.table {
        for line in render_import_table(
            &report.rows,
            !args.no_header,
            if args.output_columns.is_empty() {
                None
            } else {
                Some(args.output_columns.as_slice())
            },
        ) {
            println!("{line}");
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
        let secret_visibility = build_import_secret_visibility_entries(&report.import_dir);
        if !secret_visibility.is_empty() {
            println!(
                "Secret placeholder visibility: {}",
                Value::Array(secret_visibility)
            );
        }
    } else {
        println!("Import mode: {}", report.mode);
        for row in &report.rows {
            println!(
                "Dry-run datasource uid={} name={} dest={} action={} file={}",
                row[0], row[1], row[3], row[4], row[6]
            );
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
        let secret_visibility = build_import_secret_visibility_entries(&report.import_dir);
        if !secret_visibility.is_empty() {
            println!(
                "Secret placeholder visibility: {}",
                Value::Array(secret_visibility)
            );
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn build_import_payload(record: &DatasourceImportRecord) -> Value {
    build_import_payload_with_secret_values(record, None)
        .expect("import payload without secret values should remain valid")
}

fn parse_secret_values_argument(value: Option<&str>) -> Result<Option<Map<String, Value>>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| message(format!("Invalid JSON for --secret-values: {error}")))?;
    let object = value
        .as_object()
        .cloned()
        .ok_or_else(|| message("--secret-values must decode to a JSON object."))?;
    Ok(Some(object))
}

#[cfg(test)]
pub(crate) fn build_import_payload_with_secret_values(
    record: &DatasourceImportRecord,
    secret_values: Option<&Map<String, Value>>,
) -> Result<Value> {
    build_import_payload_with_secret_values_impl(record, secret_values)
}

fn build_import_payload_with_secret_values_impl(
    record: &DatasourceImportRecord,
    secret_values: Option<&Map<String, Value>>,
) -> Result<Value> {
    let mut payload = Map::from_iter(vec![
        ("name".to_string(), Value::String(record.name.clone())),
        (
            "type".to_string(),
            Value::String(record.datasource_type.clone()),
        ),
        ("url".to_string(), Value::String(record.url.clone())),
        ("access".to_string(), Value::String(record.access.clone())),
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
    ]);
    if let Some(placeholders) = &record.secure_json_data_placeholders {
        let datasource_spec = Map::from_iter(vec![
            ("uid".to_string(), Value::String(record.uid.clone())),
            ("name".to_string(), Value::String(record.name.clone())),
            (
                "type".to_string(),
                Value::String(record.datasource_type.clone()),
            ),
            (
                "secureJsonDataPlaceholders".to_string(),
                Value::Object(placeholders.clone()),
            ),
        ]);
        let plan = build_secret_placeholder_plan(&datasource_spec)?;
        let secret_values = secret_values.ok_or_else(|| {
            message(format!(
                "Datasource import for '{}' requires --secret-values because secureJsonDataPlaceholders are present. {}",
                if record.uid.is_empty() { &record.name } else { &record.uid },
                describe_secret_placeholder_plan(&plan)
            ))
        })?;
        let resolved = resolve_secret_placeholders(&plan.placeholders, secret_values)?;
        if !resolved.is_empty() {
            payload.insert("secureJsonData".to_string(), Value::Object(resolved));
        }
    }
    Ok(Value::Object(payload))
}

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
    let (metadata, records) = load_import_records(&args.import_dir)?;
    let secret_values = parse_secret_values_argument(args.secret_values.as_deref())?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
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
        args.import_dir.display(),
        plan.would_update,
        plan.would_skip,
        0usize
    );
    Ok(plan.would_create + plan.would_update)
}

pub(crate) fn import_datasources_by_export_org(args: &DatasourceImportArgs) -> Result<usize> {
    let admin_client = build_http_client(&args.common)?;
    let scopes = discover_export_org_import_scopes(args)?;
    if args.dry_run && args.json {
        println!("{}", build_routed_datasource_import_dry_run_json(args)?);
        return Ok(0);
    }
    let mut org_rows = Vec::new();
    let mut plans = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(&admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.import_dir)?.1.len();
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
            plan.import_dir.display().to_string(),
        ]);
        plans.push(plan);
    }
    if args.dry_run && args.table {
        for line in render_routed_datasource_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(0);
    }
    let mut imported_count = 0usize;
    for plan in plans {
        println!(
            "Importing {}",
            format_routed_datasource_scope_summary_fields(
                plan.source_org_id,
                &plan.source_org_name,
                plan.org_action,
                plan.target_org_id,
                &plan.import_dir,
            )
        );
        let Some(target_org_id) = plan.target_org_id else {
            continue;
        };
        let mut scoped_args = args.clone();
        scoped_args.org_id = Some(target_org_id);
        scoped_args.use_export_org = false;
        scoped_args.only_org_id = Vec::new();
        scoped_args.create_missing_orgs = false;
        scoped_args.import_dir = plan.import_dir.clone();
        let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
        imported_count +=
            import_datasources_with_client(&scoped_client, &scoped_args).map_err(|error| {
                message(format!(
                    "Datasource routed import failed for {}: {}",
                    format_routed_datasource_scope_summary_fields(
                        plan.source_org_id,
                        &plan.source_org_name,
                        plan.org_action,
                        plan.target_org_id,
                        &plan.import_dir,
                    ),
                    error
                ))
            })?;
    }
    Ok(imported_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::CommonCliArgs;
    use crate::http::{JsonHttpClient, JsonHttpClientConfig};
    use serde_json::json;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    fn build_test_common_args(base_url: String) -> CommonCliArgs {
        CommonCliArgs {
            url: base_url,
            api_token: None,
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 5,
            verify_ssl: false,
        }
    }

    fn write_import_fixture(import_dir: &Path) {
        fs::write(
            import_dir.join(EXPORT_METADATA_FILENAME),
            format!(
                "{{\n  \"schemaVersion\": {},\n  \"kind\": \"{}\",\n  \"variant\": \"root\",\n  \"resource\": \"datasource\",\n  \"datasourceCount\": 2,\n  \"datasourcesFile\": \"{}\",\n  \"indexFile\": \"index.json\",\n  \"format\": \"grafana-datasource-inventory-v1\"\n}}\n",
                1,
                "grafana-utils-datasource-export-index",
                DATASOURCE_EXPORT_FILENAME
            ),
        )
        .unwrap();
        fs::write(
            import_dir.join(DATASOURCE_EXPORT_FILENAME),
            r#"[
  {
    "uid": "prom-main",
    "name": "Prometheus Main",
    "type": "prometheus",
    "access": "proxy",
    "url": "http://prometheus:9090",
    "isDefault": false,
    "orgId": "1"
  },
  {
    "uid": "loki-main",
    "name": "Loki Main",
    "type": "loki",
    "access": "proxy",
    "url": "http://loki:3100",
    "isDefault": false,
    "orgId": "1",
    "secureJsonDataPlaceholders": {
      "basicAuthPassword": "${secret:loki-basic-auth}",
      "httpHeaderValue1": "${secret:loki-tenant-token}"
    }
  }
]
"#,
        )
        .unwrap();
    }

    fn spawn_datasource_import_server() -> (
        String,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
        thread::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let saw_write = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let saw_write_thread = Arc::clone(&saw_write);
        let stop_thread = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            while !stop_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut buffer = [0_u8; 1024];
                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(read) => {
                                    request.extend_from_slice(&buffer[..read]);
                                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                        break;
                                    }
                                }
                                Err(error)
                                    if error.kind() == std::io::ErrorKind::WouldBlock
                                        || error.kind() == std::io::ErrorKind::TimedOut =>
                                {
                                    break;
                                }
                                Err(error) => panic!("failed to read test request: {error}"),
                            }
                        }
                        let request_text = String::from_utf8_lossy(&request);
                        let request_line = request_text.lines().next().unwrap_or_default();
                        let response = if request_line.starts_with("GET /api/datasources ") {
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]".to_vec()
                        } else {
                            saw_write_thread.store(true, Ordering::SeqCst);
                            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}".to_vec()
                        };
                        stream.write_all(&response).unwrap();
                        let _ = stream.flush();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("failed to accept test connection: {error}"),
                }
            }
        });
        (format!("http://{address}"), saw_write, stop, handle)
    }

    #[test]
    fn import_datasources_preflights_secret_resolution_before_any_write() {
        let temp = tempdir().unwrap();
        write_import_fixture(temp.path());
        let (base_url, saw_write, stop, handle) = spawn_datasource_import_server();
        let client = JsonHttpClient::new(JsonHttpClientConfig {
            base_url,
            headers: Vec::new(),
            timeout_secs: 5,
            verify_ssl: false,
        })
        .unwrap();
        let args = DatasourceImportArgs {
            common: build_test_common_args("http://unused".to_string()),
            import_dir: temp.path().to_path_buf(),
            org_id: None,
            use_export_org: false,
            only_org_id: Vec::new(),
            create_missing_orgs: false,
            require_matching_export_org: false,
            replace_existing: false,
            update_existing_only: false,
            secret_values: Some(r#"{"loki-basic-auth":"secret-value"}"#.to_string()),
            dry_run: false,
            table: false,
            json: false,
            output_format: None,
            no_header: false,
            output_columns: Vec::new(),
            progress: false,
            verbose: false,
        };

        let error = import_datasources_with_client(&client, &args)
            .unwrap_err()
            .to_string();
        stop.store(true, Ordering::SeqCst);
        handle.join().unwrap();

        assert!(error.contains("must resolve to non-empty strings before import"));
        assert!(error.contains("loki-tenant-token"));
        assert!(!saw_write.load(Ordering::SeqCst));
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
                org_id: "1".to_string(),
                secure_json_data_placeholders: None,
            },
            DatasourceImportRecord {
                uid: "loki-main".to_string(),
                name: "Loki Main".to_string(),
                datasource_type: "loki".to_string(),
                access: "proxy".to_string(),
                url: "http://loki:3100".to_string(),
                is_default: false,
                org_id: "1".to_string(),
                secure_json_data_placeholders: json!({
                    "basicAuthPassword": "${secret:loki-basic-auth}",
                })
                .as_object()
                .cloned(),
            },
        ];
        let live = Vec::<Map<String, Value>>::new();
        let secret_values = json!({
            "loki-basic-auth": "secret-value"
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
        assert_eq!(plan.requests[0].method, Method::POST);
        assert_eq!(plan.requests[1].method, Method::POST);
        assert_eq!(plan.requests[1].path, "/api/datasources");
        assert_eq!(
            plan.requests[1].payload["secureJsonData"]["basicAuthPassword"],
            json!("secret-value")
        );
    }
}
