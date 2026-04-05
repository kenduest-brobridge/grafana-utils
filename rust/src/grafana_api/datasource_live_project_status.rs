use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};
use crate::grafana_api::project_status_live as project_status_live_support;
use crate::datasource_live_project_status::LiveDatasourceProjectStatusInputs;

pub(crate) fn collect_live_datasource_project_status_inputs_with_request<F>(
    request_json: &mut F,
) -> Result<LiveDatasourceProjectStatusInputs>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let datasource_list = match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_object()
                    .cloned()
                    .ok_or_else(|| message("Unexpected datasource list response from Grafana."))
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => Vec::new(),
    };
    let org_list =
        project_status_live_support::list_visible_orgs_with_request(request_json).unwrap_or_default();
    let current_org = project_status_live_support::fetch_current_org_with_request(request_json).ok();
    Ok(LiveDatasourceProjectStatusInputs {
        datasource_list,
        org_list,
        current_org,
    })
}
