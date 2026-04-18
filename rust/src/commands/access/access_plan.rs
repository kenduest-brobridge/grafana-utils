//! Access plan review helpers.
//!
//! Access plan orchestrator:
//! - parser already lives in `cli_defs`
//! - pure model and renderers live here so later TUI code can reuse the same document
//! - resource-specific planners stay in sibling modules
//! - aggregate `--resource all` stays in `access_plan_all`

use reqwest::Method;
use serde_json::Value;

use super::access_plan_org::build_org_access_plan_actions;
use crate::access::cli_defs::{AccessPlanArgs, AccessPlanResource, PlanOutputFormat};
use crate::common::{tool_version, Result};

#[path = "access_plan_all.rs"]
mod access_plan_all;
#[path = "access_plan_render.rs"]
mod access_plan_render;
#[path = "access_plan_service_account.rs"]
mod access_plan_service_account;
#[path = "access_plan_types.rs"]
mod access_plan_types;
#[path = "access_plan_user.rs"]
mod access_plan_user;

pub(crate) use access_plan_render::print_access_plan_columns;
use access_plan_render::{
    render_plan_json, render_plan_table, render_plan_text, validate_plan_columns,
};
pub(crate) use access_plan_types::{
    AccessPlanAction, AccessPlanChange, AccessPlanDocument, AccessPlanResourceReport,
    AccessPlanSummary,
};
use access_plan_user::build_user_access_plan_document;

const ACCESS_PLAN_KIND: &str = "grafana-util-access-plan";
const ACCESS_PLAN_SCHEMA_VERSION: i64 = 1;

fn sort_actions(actions: &mut [AccessPlanAction]) {
    actions.sort_by(|left, right| {
        left.resource_kind
            .cmp(&right.resource_kind)
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.action.cmp(&right.action))
    });
}

fn build_access_plan_summary(
    resources: &[AccessPlanResourceReport],
    prune: bool,
) -> AccessPlanSummary {
    AccessPlanSummary {
        resource_count: resources.len(),
        checked: resources.iter().map(|item| item.checked).sum(),
        same: resources.iter().map(|item| item.same).sum(),
        create: resources.iter().map(|item| item.create).sum(),
        update: resources.iter().map(|item| item.update).sum(),
        extra_remote: resources.iter().map(|item| item.extra_remote).sum(),
        delete: resources.iter().map(|item| item.delete).sum(),
        blocked: resources.iter().map(|item| item.blocked).sum(),
        warning: resources.iter().map(|item| item.warning).sum(),
        prune,
    }
}

fn build_access_plan_document_from_parts(
    resources: Vec<AccessPlanResourceReport>,
    mut actions: Vec<AccessPlanAction>,
    prune: bool,
) -> AccessPlanDocument {
    sort_actions(&mut actions);
    AccessPlanDocument {
        kind: ACCESS_PLAN_KIND.to_string(),
        schema_version: ACCESS_PLAN_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        summary: build_access_plan_summary(&resources, prune),
        resources,
        actions,
    }
}

fn build_org_access_plan_document<F>(
    request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let (resource, actions) = build_org_access_plan_actions(request_json, args, &args.input_dir)?;
    let resources = vec![resource];
    Ok(build_access_plan_document_from_parts(
        resources, actions, args.prune,
    ))
}

fn build_access_plan_document<F>(
    request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match args.resource {
        AccessPlanResource::User => build_user_access_plan_document(request_json, args),
        AccessPlanResource::Org => build_org_access_plan_document(request_json, args),
        AccessPlanResource::Team => super::access_plan_team::build_team_access_plan_document(
            request_json,
            args,
            ACCESS_PLAN_KIND,
            ACCESS_PLAN_SCHEMA_VERSION,
        ),
        AccessPlanResource::ServiceAccount => {
            access_plan_service_account::build_service_account_plan_document(request_json, args)
        }
        AccessPlanResource::All => {
            access_plan_all::build_all_access_plan_document(request_json, args)
        }
    }
}

pub(crate) fn access_plan_with_request<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_plan_columns(args)?;
    let document = build_access_plan_document(&mut request_json, args)?;
    match args.output_format {
        PlanOutputFormat::Text => {
            print!("{}", render_plan_text(&document, args));
        }
        PlanOutputFormat::Table => {
            print!("{}", render_plan_table(&document, args));
        }
        PlanOutputFormat::Json => {
            print!("{}", render_plan_json(&document)?);
        }
    }
    Ok(document.actions.len())
}

#[cfg(test)]
#[path = "access_plan_tests.rs"]
mod tests;
