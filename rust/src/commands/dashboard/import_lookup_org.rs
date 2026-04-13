use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::Result;
use crate::grafana_api::DashboardResourceClient;

use super::import_lookup_cache::ImportLookupCache;

pub(crate) fn resolve_import_target_org_id_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    args: &super::super::ImportArgs,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(org_id) = args.org_id {
        return Ok(org_id.to_string());
    }
    if let Some(org_id) = cache.current_org_id.as_ref() {
        return Ok(org_id.clone());
    }
    let org = super::super::list::fetch_current_org_with_request(request_json)?;
    let org_id: String = super::super::list::org_id_value(&org)?.to_string();
    cache.current_org_id = Some(org_id.clone());
    Ok(org_id)
}

#[allow(dead_code)]
pub(crate) fn resolve_import_target_org_id_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
    args: &super::super::ImportArgs,
) -> Result<String> {
    if let Some(org_id) = args.org_id {
        return Ok(org_id.to_string());
    }
    if let Some(org_id) = cache.current_org_id.as_ref() {
        return Ok(org_id.clone());
    }
    let org = client.fetch_current_org()?;
    let org_id: String = super::super::list::org_id_value(&org)?.to_string();
    cache.current_org_id = Some(org_id.clone());
    Ok(org_id)
}

pub(crate) fn list_orgs_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(orgs) = cache.orgs.as_ref() {
        return Ok(orgs.clone());
    }
    let orgs = super::super::list::list_orgs_with_request(request_json)?;
    cache.orgs = Some(orgs.clone());
    Ok(orgs)
}

#[allow(dead_code)]
pub(crate) fn list_orgs_cached_with_client(
    client: &DashboardResourceClient<'_>,
    cache: &mut ImportLookupCache,
) -> Result<Vec<Map<String, Value>>> {
    if let Some(orgs) = cache.orgs.as_ref() {
        return Ok(orgs.clone());
    }
    let orgs = client.list_orgs()?;
    cache.orgs = Some(orgs.clone());
    Ok(orgs)
}
