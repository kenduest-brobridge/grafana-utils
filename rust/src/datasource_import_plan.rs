//! Datasource import request planning helpers.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::datasource::resolve_match;

use super::datasource_import_export_support::DatasourceImportRecord;
use super::datasource_import_payload::build_import_payload_with_secret_values;

pub(crate) struct PreparedDatasourceImportRequest {
    pub(crate) method: Method,
    pub(crate) path: String,
    pub(crate) payload: Value,
}

pub(crate) struct PreparedDatasourceImportPlan {
    pub(crate) requests: Vec<PreparedDatasourceImportRequest>,
    pub(crate) would_create: usize,
    pub(crate) would_update: usize,
    pub(crate) would_skip: usize,
}

pub(crate) fn prepare_datasource_import_plan(
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
                let payload = build_import_payload_with_secret_values(record, secret_values)?;
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
                let payload = build_import_payload_with_secret_values(record, secret_values)?;
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
