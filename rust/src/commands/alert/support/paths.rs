use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::common::{sanitize_path_component, string_field};

use super::super::{
    CONTACT_POINTS_SUBDIR, CONTACT_POINT_KIND, MUTE_TIMINGS_SUBDIR, MUTE_TIMING_KIND,
    POLICIES_KIND, POLICIES_SUBDIR, RULES_SUBDIR, RULE_KIND, TEMPLATES_SUBDIR, TEMPLATE_KIND,
};

pub fn resource_subdir_by_kind() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (RULE_KIND, RULES_SUBDIR),
        (CONTACT_POINT_KIND, CONTACT_POINTS_SUBDIR),
        (MUTE_TIMING_KIND, MUTE_TIMINGS_SUBDIR),
        (POLICIES_KIND, POLICIES_SUBDIR),
        (TEMPLATE_KIND, TEMPLATES_SUBDIR),
    ])
}

pub fn build_rule_output_path(output_dir: &Path, rule: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_uid = sanitize_path_component(&string_field(rule, "folderUID", "general"));
    let rule_group = sanitize_path_component(&string_field(rule, "ruleGroup", "default"));
    let title = sanitize_path_component(&string_field(rule, "title", "rule"));
    let uid = sanitize_path_component(&string_field(rule, "uid", "unknown"));
    let file_name = format!("{title}__{uid}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(folder_uid).join(rule_group).join(file_name)
    }
}

pub fn build_contact_point_output_path(
    output_dir: &Path,
    contact_point: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(contact_point, "name", "contact-point"));
    let uid = sanitize_path_component(&string_field(contact_point, "uid", "unknown"));
    let file_name = format!("{name}__{uid}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_mute_timing_output_path(
    output_dir: &Path,
    mute_timing: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(mute_timing, "name", "mute-timing"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_policies_output_path(output_dir: &Path) -> PathBuf {
    output_dir.join("notification-policies.json")
}

pub fn build_template_output_path(
    output_dir: &Path,
    template: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(template, "name", "template"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_resource_dirs(raw_dir: &Path) -> BTreeMap<&'static str, PathBuf> {
    resource_subdir_by_kind()
        .into_iter()
        .map(|(kind, subdir)| (kind, raw_dir.join(subdir)))
        .collect()
}
