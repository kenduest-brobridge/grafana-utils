//! Aggregate access plan support for `--resource all`.

use crate::access::access_plan_team;
use crate::access::cli_defs::{AccessPlanArgs, AccessPlanResource};
use crate::access::{
    ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME, DEFAULT_ACCESS_ORG_EXPORT_DIR,
    DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR, DEFAULT_ACCESS_TEAM_EXPORT_DIR,
    DEFAULT_ACCESS_USER_EXPORT_DIR,
};
use crate::common::{message, Result};
use reqwest::Method;
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::{
    access_plan_service_account, build_access_plan_document_from_parts,
    build_org_access_plan_document, build_user_access_plan_document, AccessPlanAction,
    AccessPlanDocument, AccessPlanResourceReport, ACCESS_PLAN_KIND, ACCESS_PLAN_SCHEMA_VERSION,
};

fn missing_access_plan_resource_report(
    resource_kind: &str,
    input_dir: &Path,
    bundle_file: &str,
) -> AccessPlanResourceReport {
    AccessPlanResourceReport {
        resource_kind: resource_kind.to_string(),
        source_path: input_dir.join(bundle_file).to_string_lossy().to_string(),
        bundle_present: false,
        source_count: 0,
        live_count: 0,
        checked: 0,
        same: 0,
        create: 0,
        update: 0,
        extra_remote: 0,
        delete: 0,
        blocked: 0,
        warning: 0,
        scope: None,
        notes: vec![format!(
            "bundle not found; skipped {}",
            input_dir.join(bundle_file).display()
        )],
    }
}

fn scoped_access_plan_args(
    args: &AccessPlanArgs,
    resource: AccessPlanResource,
    input_dir: impl Into<PathBuf>,
) -> AccessPlanArgs {
    let mut scoped = args.clone();
    scoped.resource = resource;
    scoped.input_dir = input_dir.into();
    scoped
}

fn append_access_plan_document(
    resources: &mut Vec<AccessPlanResourceReport>,
    actions: &mut Vec<AccessPlanAction>,
    document: AccessPlanDocument,
) {
    resources.extend(document.resources);
    actions.extend(document.actions);
}

pub(super) fn build_all_access_plan_document<F>(
    mut request_json: F,
    args: &AccessPlanArgs,
) -> Result<AccessPlanDocument>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut resources = Vec::new();
    let mut actions = Vec::new();

    let user_dir = args.input_dir.join(DEFAULT_ACCESS_USER_EXPORT_DIR);
    if user_dir.join(ACCESS_USER_EXPORT_FILENAME).is_file() {
        let scoped = scoped_access_plan_args(args, AccessPlanResource::User, user_dir);
        append_access_plan_document(
            &mut resources,
            &mut actions,
            build_user_access_plan_document(&mut request_json, &scoped)?,
        );
    } else {
        resources.push(missing_access_plan_resource_report(
            "user",
            &user_dir,
            ACCESS_USER_EXPORT_FILENAME,
        ));
    }

    let org_dir = args.input_dir.join(DEFAULT_ACCESS_ORG_EXPORT_DIR);
    if org_dir.join(ACCESS_ORG_EXPORT_FILENAME).is_file() {
        let scoped = scoped_access_plan_args(args, AccessPlanResource::Org, org_dir);
        append_access_plan_document(
            &mut resources,
            &mut actions,
            build_org_access_plan_document(&mut request_json, &scoped)?,
        );
    } else {
        resources.push(missing_access_plan_resource_report(
            "org",
            &org_dir,
            ACCESS_ORG_EXPORT_FILENAME,
        ));
    }

    let team_dir = args.input_dir.join(DEFAULT_ACCESS_TEAM_EXPORT_DIR);
    if team_dir.join(ACCESS_TEAM_EXPORT_FILENAME).is_file() {
        let scoped = scoped_access_plan_args(args, AccessPlanResource::Team, team_dir);
        append_access_plan_document(
            &mut resources,
            &mut actions,
            access_plan_team::build_team_access_plan_document(
                &mut request_json,
                &scoped,
                ACCESS_PLAN_KIND,
                ACCESS_PLAN_SCHEMA_VERSION,
            )?,
        );
    } else {
        resources.push(missing_access_plan_resource_report(
            "team",
            &team_dir,
            ACCESS_TEAM_EXPORT_FILENAME,
        ));
    }

    let service_account_dir = args
        .input_dir
        .join(DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR);
    if service_account_dir
        .join(ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME)
        .is_file()
    {
        let scoped = scoped_access_plan_args(
            args,
            AccessPlanResource::ServiceAccount,
            service_account_dir,
        );
        append_access_plan_document(
            &mut resources,
            &mut actions,
            access_plan_service_account::build_service_account_plan_document(
                &mut request_json,
                &scoped,
            )?,
        );
    } else {
        resources.push(missing_access_plan_resource_report(
            "service-account",
            &service_account_dir,
            ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
        ));
    }

    if actions.is_empty() && resources.iter().all(|item| !item.bundle_present) {
        return Err(message(format!(
            "access plan --resource all did not find any access bundle directories under {}. Expected {}, {}, {}, or {}.",
            args.input_dir.display(),
            DEFAULT_ACCESS_USER_EXPORT_DIR,
            DEFAULT_ACCESS_ORG_EXPORT_DIR,
            DEFAULT_ACCESS_TEAM_EXPORT_DIR,
            DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        )));
    }

    Ok(build_access_plan_document_from_parts(
        resources, actions, args.prune,
    ))
}
